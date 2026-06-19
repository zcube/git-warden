//! Full tracked-file checks (run command). Corresponds to Run* functions in Go `internal/checker/run.go`.

use std::sync::atomic::AtomicBool;

use crate::config::Config;

use super::binary::evaluate_binary_policy;
use super::diff_check::{build_comment_units, resolve_file_lang};
use super::for_each_file_concurrent;

/// Checks all tracked files for binary files. Corresponds to Go `RunBinaryFiles`.
pub fn run_binary_files(
    cfg: &Config,
    files: &[String],
    cancel: &AtomicBool,
) -> Result<Vec<String>, String> {
    if !cfg.binary_file.is_enabled() {
        return Ok(Vec::new());
    }
    let mut ignore_patterns = cfg.exceptions.global_ignore.clone();
    ignore_patterns.extend(cfg.binary_file.ignore_files.iter().cloned());

    for_each_file_concurrent(files, cancel, |path| {
        if crate::pathutil::matches_any(path, &ignore_patterns) {
            return Ok(Vec::new());
        }
        let content = match std::fs::read(path) {
            Ok(c) => c,
            Err(_) => return Ok(Vec::new()),
        };
        if crate::encoding::is_binary(&content) {
            let msg = evaluate_binary_policy(&cfg.binary_file, path);
            if !msg.is_empty() {
                return Ok(vec![msg]);
            }
        }
        Ok(Vec::new())
    })
}

/// Validates UTF-8 encoding of all tracked files. Corresponds to Go `RunEncoding`.
pub fn run_encoding(
    cfg: &Config,
    files: &[String],
    cancel: &AtomicBool,
) -> Result<Vec<String>, String> {
    if !cfg.encoding.is_enabled() || !cfg.encoding.is_require_utf8() {
        return Ok(Vec::new());
    }
    let mut ignore_patterns = cfg.exceptions.global_ignore.clone();
    ignore_patterns.extend(cfg.encoding.ignore_files.iter().cloned());

    for_each_file_concurrent(files, cancel, |path| {
        if crate::pathutil::matches_any(path, &ignore_patterns) {
            return Ok(Vec::new());
        }
        // Skip if editorconfig charset is not utf-8 family.
        if let Some(def) = crate::editorconfig::get_definition(path) {
            if !def.charset.is_empty() && def.charset != "utf-8" && def.charset != "utf-8-bom" {
                return Ok(Vec::new());
            }
        }
        let content = match std::fs::read(path) {
            Ok(c) => c,
            Err(_) => return Ok(Vec::new()),
        };
        if crate::encoding::is_binary(&content) {
            return Ok(Vec::new());
        }
        let result = crate::encoding::check_utf8(&content);
        if !result.valid {
            return Ok(vec![crate::t!(
                "diff.encoding_error",
                Path = path,
                Charset = result.detected_charset
            )]);
        }
        Ok(Vec::new())
    })
}

/// Checks all tracked files for .editorconfig rule compliance. Corresponds to Go `RunEditorConfig`.
pub fn run_editorconfig(
    cfg: &Config,
    files: &[String],
    cancel: &AtomicBool,
) -> Result<Vec<String>, String> {
    if !cfg.editorconfig.is_enabled() {
        return Ok(Vec::new());
    }
    if !std::path::Path::new(".editorconfig").exists() {
        return Ok(Vec::new());
    }

    for_each_file_concurrent(files, cancel, |path| {
        if crate::pathutil::matches_any(path, &cfg.exceptions.global_ignore) {
            return Ok(Vec::new());
        }
        if crate::pathutil::matches_any(path, &cfg.editorconfig.ignore_files) {
            return Ok(Vec::new());
        }
        let def = match crate::editorconfig::get_definition(path) {
            Some(d) => d,
            None => return Ok(Vec::new()),
        };
        let content = match std::fs::read(path) {
            Ok(c) => c,
            Err(_) => return Ok(Vec::new()),
        };
        let msgs = crate::editorconfig::check(path, &content, &def)
            .into_iter()
            .map(|v| {
                crate::t!(
                    "diff.editorconfig_error",
                    Path = v.file,
                    Line = v.line,
                    Message = v.message
                )
            })
            .collect();
        Ok(msgs)
    })
}

/// Checks comment language in all tracked source files (always full-file mode). Corresponds to Go `RunCommentLanguage`.
pub fn run_comment_language(
    cfg: &Config,
    files: &[String],
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
    let skip_directives = cfg.comment_language.skip_directives.clone();
    let check_strings = cfg.comment_language.is_check_strings();
    let no_emoji = cfg.comment_language.is_no_emoji();
    let allowed_words = cfg.comment_language.allowed_words.clone();

    let mut ignore_patterns = cfg.exceptions.global_ignore.clone();
    ignore_patterns.extend(cfg.exceptions.comment_language_ignore.iter().cloned());
    ignore_patterns.extend(cfg.comment_language.ignore_files.iter().cloned());

    let kind_comment = crate::t!("diff.kind_comment");

    for_each_file_concurrent(files, cancel, |file_path| {
        if !crate::gitdiff::has_extension(file_path, &extensions) {
            return Ok(Vec::new());
        }
        if crate::pathutil::matches_any(file_path, &ignore_patterns) {
            return Ok(Vec::new());
        }
        let parser = match crate::comment::get_parser(file_path) {
            Some(p) => p,
            None => return Ok(Vec::new()),
        };
        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => return Ok(Vec::new()),
        };
        let comments = match parser.parse_file(&content) {
            Ok(c) => c,
            Err(e) => {
                crate::logger::warn(
                    "comment parse warning",
                    &[("path", file_path.to_string()), ("error", e)],
                );
                Vec::new()
            }
        };
        let file_lang = resolve_file_lang(file_path, cfg);
        let states = crate::directive::analyze(&comments, &file_lang);

        let mut msgs = Vec::new();
        for u in build_comment_units(&comments, &states, check_strings) {
            if u.kind == crate::comment::Kind::String {
                continue;
            }
            let text = crate::langdetect::strip_allowed_words(&u.text, &allowed_words);
            let (ok, has_content) = crate::langdetect::is_required_language(
                &text,
                &u.lang,
                min_length,
                &skip_directives,
            );
            if !has_content {
                continue;
            }
            if !ok {
                let detected = crate::langdetect::dominant_language(&text);
                msgs.push(crate::t!(
                    "diff.comment_language_error",
                    Path = file_path,
                    Line = u.line,
                    Kind = kind_comment,
                    Language = u.lang,
                    Detected = detected,
                    Text = super::truncate(&text, 80)
                ));
            }
            if no_emoji {
                for e in crate::emoji::find_emojis(&text) {
                    msgs.push(crate::t!(
                        "diff.emoji_error",
                        Path = file_path,
                        Line = u.line + e.line as i64 - 1,
                        Kind = kind_comment,
                        Char = e.char,
                        CharCode = format!("{:04X}", e.code)
                    ));
                }
            }
        }
        Ok(msgs)
    })
}
