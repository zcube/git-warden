//! Comment language checks (staged diff) + comment grouping + technical string helpers.
//! Corresponds to Go `internal/checker/diff.go`, `comment_group.go`.

use std::sync::atomic::AtomicBool;

use crate::comment::{Comment, Kind};
use crate::config::Config;
use crate::directive::CommentState;
use crate::gitdiff::FileDiff;

/// Basic unit for language checks. Corresponds to Go `commentUnit`.
pub(crate) struct CommentUnit {
    pub text: String,
    pub line: i64,
    pub end_line: i64,
    pub lang: String,
    pub kind: Kind,
}

/// Builds language check units from a comment list and directive states. Corresponds to Go `buildCommentUnits`.
pub(crate) fn build_comment_units(
    comments: &[Comment],
    states: &[CommentState],
    check_strings: bool,
) -> Vec<CommentUnit> {
    let n = comments.len();
    let mut units = Vec::new();
    let mut i = 0;
    while i < n {
        let c = &comments[i];
        let s = &states[i];

        if s.skip || c.kind == Kind::Import {
            i += 1;
            continue;
        }
        if c.kind == Kind::String && !check_strings {
            i += 1;
            continue;
        }

        // Block comment or string literal: treat as individual unit.
        if c.is_block || c.kind == Kind::String {
            units.push(CommentUnit {
                text: c.text.trim().to_string(),
                line: c.line,
                end_line: c.end_line,
                lang: s.language.clone(),
                kind: c.kind,
            });
            i += 1;
            continue;
        }

        // Line comment: group consecutive ones into a single unit.
        let start_line = c.line;
        let mut prev_end_line = c.end_line;
        let lang = s.language.clone();
        let mut texts: Vec<String> = Vec::new();

        while i < n {
            let ci = &comments[i];
            let si = &states[i];

            if si.skip || ci.kind == Kind::Import {
                break;
            }
            if ci.is_block || ci.kind == Kind::String {
                break;
            }
            if si.language != lang {
                break;
            }
            // Line continuity: must be immediately after the previous comment.
            if !texts.is_empty() && ci.line > prev_end_line + 1 {
                break;
            }

            let t = ci.text.trim();
            if !t.is_empty() {
                texts.push(t.to_string());
            }
            prev_end_line = ci.end_line;
            i += 1;
        }

        if !texts.is_empty() {
            units.push(CommentUnit {
                text: texts.join("\n"),
                line: start_line,
                end_line: prev_end_line,
                lang,
                kind: Kind::Comment,
            });
        }
    }
    units
}

/// Checks staged diff for comment language violations. Corresponds to Go `CheckDiff`.
pub fn check_diff(
    cfg: &Config,
    diffs: &[FileDiff],
    cancel: &AtomicBool,
) -> Result<Vec<String>, String> {
    if !cfg.comment_language.is_enabled() {
        return Ok(Vec::new());
    }

    let extensions = if !cfg.comment_language.languages.is_empty() {
        crate::comment::extensions_for_languages(&cfg.comment_language.languages)
    } else {
        cfg.comment_language.extensions.clone()
    };

    let min_length = cfg.comment_language.min_length.max(0) as usize;
    let skip_directives = &cfg.comment_language.skip_directives;
    let full_mode = cfg.comment_language.is_full_mode();
    let check_strings = cfg.comment_language.is_check_strings();
    let no_emoji = cfg.comment_language.is_no_emoji();
    let allowed_words = &cfg.comment_language.allowed_words;

    let mut ignore_patterns = cfg.exceptions.global_ignore.clone();
    ignore_patterns.extend(cfg.exceptions.comment_language_ignore.iter().cloned());
    ignore_patterns.extend(cfg.comment_language.ignore_files.iter().cloned());

    let kind_comment = crate::t!("diff.kind_comment");
    let mut errs = Vec::new();

    for diff in diffs {
        super::check_cancelled(cancel)?;
        if diff.is_deleted || diff.is_submodule || diff.is_symlink {
            continue;
        }
        if !full_mode && diff.added_lines.is_empty() {
            continue;
        }
        if !crate::gitdiff::has_extension(&diff.path, &extensions) {
            continue;
        }
        if crate::pathutil::matches_any(&diff.path, &ignore_patterns) {
            continue;
        }

        let parser = match crate::comment::get_parser(&diff.path) {
            Some(p) => p,
            None => continue,
        };

        let staged_content = match crate::gitdiff::get_staged_content(&diff.path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let comments = match parser.parse_file(&staged_content) {
            Ok(c) => c,
            Err(e) => {
                crate::logger::warn(
                    "comment parse warning",
                    &[("path", diff.path.clone()), ("error", e)],
                );
                Vec::new()
            }
        };

        let file_lang = resolve_file_lang(&diff.path, cfg);
        let states = crate::directive::analyze(&comments, &file_lang);

        for u in build_comment_units(&comments, &states, check_strings) {
            // diff mode: unit line range must overlap with added lines.
            if !full_mode {
                let mut overlaps = false;
                for ln in u.line..=u.end_line {
                    if diff.added_lines.contains(&ln) {
                        overlaps = true;
                        break;
                    }
                }
                if !overlaps {
                    continue;
                }
            }

            if u.kind == Kind::String {
                continue;
            }
            let text = crate::langdetect::strip_allowed_words(&u.text, allowed_words);
            let (ok, has_content) = crate::langdetect::is_required_language(
                &text,
                &u.lang,
                min_length,
                skip_directives,
            );
            if !has_content {
                continue;
            }
            if !ok {
                let detected = crate::langdetect::dominant_language(&text);
                errs.push(crate::t!(
                    "diff.comment_language_error",
                    Path = diff.path,
                    Line = u.line,
                    Kind = kind_comment,
                    Language = u.lang,
                    Detected = detected,
                    Text = truncate(&text, 80)
                ));
            }

            if no_emoji {
                for e in crate::emoji::find_emojis(&text) {
                    errs.push(crate::t!(
                        "diff.emoji_error",
                        Path = diff.path,
                        Line = u.line + e.line as i64 - 1,
                        Kind = kind_comment,
                        Char = e.char,
                        CharCode = format!("{:04X}", e.code)
                    ));
                }
            }
        }
    }

    Ok(errs)
}

/// Resolves the required language for a file path by checking file_languages rules in order. Corresponds to Go `resolveFileLang`.
pub(crate) fn resolve_file_lang(path: &str, cfg: &Config) -> String {
    for rule in &cfg.comment_language.file_languages {
        if crate::pathutil::matches_any(path, std::slice::from_ref(&rule.pattern)) {
            let v = rule.get_locale();
            if !v.is_empty() {
                return v;
            }
        }
    }
    cfg.comment_language.get_locale()
}

/// Truncates to at most max runes, appending an ellipsis if exceeded. Corresponds to Go `truncate`.
pub fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > max {
        let mut out: String = chars[..max].iter().collect();
        out.push('…');
        out
    } else {
        s.to_string()
    }
}

/// Strings containing a slash (/) are treated as path/MIME types. Corresponds to Go `IsPathLikeString`.
pub fn is_path_like_string(s: &str) -> bool {
    s.contains('/')
}

/// Pure uppercase ASCII strings (no lowercase, no non-ASCII) are treated as constant identifiers. Corresponds to Go `IsAllUppercaseASCII`.
pub fn is_all_uppercase_ascii(s: &str) -> bool {
    for r in s.chars() {
        if r as u32 > 0x7F {
            return false;
        }
        if r.is_ascii_lowercase() {
            return false;
        }
    }
    true
}

/// Determines whether a string is technical and should be excluded from language checks. Corresponds to Go `IsTechnicalString`.
pub fn is_technical_string(s: &str) -> bool {
    is_path_like_string(s) || is_all_uppercase_ascii(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_appends_ellipsis() {
        assert_eq!(truncate("hello", 80), "hello");
        assert_eq!(truncate("abcdef", 3), "abc…");
        // multibyte safe.
        assert_eq!(truncate("한국어테스트", 3), "한국어…");
    }

    #[test]
    fn technical_strings() {
        assert!(is_path_like_string("/api/v1"));
        assert!(is_path_like_string("application/json"));
        assert!(!is_path_like_string("hello"));
        assert!(is_all_uppercase_ascii("ERR_TOKEN"));
        assert!(is_all_uppercase_ascii("MAX_SIZE"));
        assert!(!is_all_uppercase_ascii("Hello"));
        assert!(!is_all_uppercase_ascii("한국어"));
        assert!(is_technical_string("/path"));
        assert!(is_technical_string("CONST_NAME"));
        assert!(!is_technical_string("hello world"));
    }
}
