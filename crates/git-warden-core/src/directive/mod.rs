//! cc-directive: parses git-warden inline directives embedded in source comments.
//! Corresponds to Go `internal/directive`.
//!
//! Supported directives (case-insensitive, `git-warden:` prefix):
//!   - `git-warden:disable` / `:disable:lang=<L>` / `:enable`
//!   - `git-warden:ignore` (skips the immediately following comment)
//!   - `git-warden:lang=<L>` (changes the required language from this point)
//!   - `git-warden:file-lang=<L>` (sets the required language for the entire file)

use crate::comment::Comment;

const PREFIX: &str = "git-warden:";

/// Describes how to handle each comment after processing directives. Corresponds to Go `CommentState`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommentState {
    /// Whether to skip language checking for this comment.
    pub skip: bool,
    /// Required language for this comment (empty string means use caller's default).
    pub language: String,
}

/// Iterates comments in source order and returns a CommentState for each. Corresponds to Go `Analyze`.
pub fn analyze(comments: &[Comment], default_lang: &str) -> Vec<CommentState> {
    let mut states = vec![CommentState::default(); comments.len()];

    let mut disabled = false;
    let mut disabled_lang = String::new();
    let mut skip_next = false;
    let mut lang_override = String::new();
    let mut file_lang = String::new();

    for (i, c) in comments.iter().enumerate() {
        let text = c.text.trim();

        if !is_directive_inner(text) {
            if !file_lang.is_empty() {
                // file-lang overrides everything except an active disable.
                if disabled {
                    states[i] = CommentState {
                        skip: disabled_lang.is_empty(),
                        language: disabled_lang.clone(),
                    };
                } else if skip_next {
                    states[i] = CommentState {
                        skip: true,
                        language: String::new(),
                    };
                    skip_next = false;
                } else {
                    let lang = if !lang_override.is_empty() {
                        lang_override.clone()
                    } else {
                        file_lang.clone()
                    };
                    states[i] = CommentState {
                        skip: false,
                        language: lang,
                    };
                }
            } else if disabled {
                states[i] = CommentState {
                    skip: disabled_lang.is_empty(),
                    language: disabled_lang.clone(),
                };
                skip_next = false;
            } else if skip_next {
                states[i] = CommentState {
                    skip: true,
                    language: String::new(),
                };
                skip_next = false;
            } else {
                states[i] = CommentState {
                    skip: false,
                    language: lang_override.clone(),
                };
            }
            continue;
        }

        // Directives themselves are always excluded from checking.
        states[i] = CommentState {
            skip: true,
            language: String::new(),
        };

        let lower = text.to_lowercase();
        if let Some(rest) = strip_ci(text, &lower, &format!("{PREFIX}file-lang=")) {
            file_lang = resolve_language(rest);
        } else if let Some(rest) = strip_ci(text, &lower, &format!("{PREFIX}disable:lang=")) {
            disabled = true;
            disabled_lang = resolve_language(rest);
        } else if lower.starts_with(&format!("{PREFIX}disable")) {
            disabled = true;
            disabled_lang = String::new();
        } else if lower.starts_with(&format!("{PREFIX}enable")) {
            disabled = false;
            disabled_lang = String::new();
        } else if lower.starts_with(&format!("{PREFIX}ignore")) {
            skip_next = true;
        } else if let Some(rest) = strip_ci(text, &lower, &format!("{PREFIX}lang=")) {
            lang_override = resolve_language(rest);
        }
    }

    // Fill empty language fields with default_lang.
    for s in states.iter_mut() {
        if !s.skip && s.language.is_empty() {
            s.language = default_lang.to_string();
        }
    }

    states
}

// Case-insensitive prefix match; returns the value part sliced from the original text.
fn strip_ci<'a>(text: &'a str, lower: &str, prefix_lower: &str) -> Option<&'a str> {
    if lower.starts_with(prefix_lower) {
        Some(&text[prefix_lower.len()..])
    } else {
        None
    }
}

/// Returns whether the comment text is a git-warden directive. Corresponds to Go `IsDirective`.
pub fn is_directive(text: &str) -> bool {
    is_directive_inner(text.trim())
}

fn is_directive_inner(text: &str) -> bool {
    text.to_lowercase().starts_with(PREFIX)
}

/// Normalizes a language value: locale codes are expanded to full names; unknown values are lowercased as-is. Corresponds to Go `resolveLanguage`.
fn resolve_language(raw: &str) -> String {
    let raw = raw.trim();
    let lower = raw.to_lowercase();
    let mapped = crate::langdetect::locale_to_language(&lower);
    if !mapped.is_empty() {
        mapped.to_string()
    } else {
        lower
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comment::{Comment, Kind};

    fn comments(texts: &[&str]) -> Vec<Comment> {
        texts
            .iter()
            .enumerate()
            .map(|(i, t)| Comment {
                text: t.to_string(),
                line: (i + 1) as i64,
                end_line: (i + 1) as i64,
                is_block: false,
                kind: Kind::Comment,
            })
            .collect()
    }

    #[test]
    fn no_directives_all_checked() {
        let cs = comments(&["한국어 주석", "또 다른 주석"]);
        for s in analyze(&cs, "korean") {
            assert!(!s.skip);
            assert_eq!(s.language, "korean");
        }
    }

    #[test]
    fn ignore_skips_next() {
        let cs = comments(&[
            "git-warden:ignore",
            "This English comment should be skipped",
            "한국어 주석은 체크됨",
        ]);
        let s = analyze(&cs, "korean");
        assert!(s[0].skip);
        assert!(s[1].skip);
        assert!(!s[2].skip);
        assert_eq!(s[2].language, "korean");
    }

    #[test]
    fn ignore_only_one() {
        let cs = comments(&[
            "git-warden:ignore",
            "skipped",
            "also checked",
            "also checked",
        ]);
        let s = analyze(&cs, "korean");
        assert!(s[1].skip);
        assert!(!s[2].skip);
        assert!(!s[3].skip);
    }

    #[test]
    fn disable_enable() {
        let cs = comments(&[
            "한국어",
            "git-warden:disable",
            "English comment",
            "another English",
            "git-warden:enable",
            "한국어 재개",
        ]);
        let s = analyze(&cs, "korean");
        assert!(!s[0].skip && s[0].language == "korean");
        assert!(s[1].skip);
        assert!(s[2].skip);
        assert!(s[3].skip);
        assert!(s[4].skip);
        assert!(!s[5].skip && s[5].language == "korean");
    }

    #[test]
    fn disable_with_lang() {
        let cs = comments(&[
            "git-warden:disable:lang=english",
            "This English comment is allowed",
            "Another English comment",
            "git-warden:enable",
            "한국어 재개",
        ]);
        let s = analyze(&cs, "korean");
        assert!(s[0].skip);
        assert!(!s[1].skip);
        assert_eq!(s[1].language, "english");
        assert_eq!(s[2].language, "english");
        assert_eq!(s[4].language, "korean");
    }

    #[test]
    fn lang_switch() {
        let cs = comments(&[
            "한국어 주석",
            "git-warden:lang=english",
            "This English is now required",
            "Another English comment",
        ]);
        let s = analyze(&cs, "korean");
        assert_eq!(s[0].language, "korean");
        assert!(s[1].skip);
        assert_eq!(s[2].language, "english");
        assert_eq!(s[3].language, "english");
    }

    #[test]
    fn lang_locale_code() {
        let cs = comments(&["git-warden:lang=ja", "これは日本語のコメントです"]);
        let s = analyze(&cs, "korean");
        assert_eq!(s[1].language, "japanese");
    }

    #[test]
    fn file_lang() {
        let cs = comments(&[
            "git-warden:file-lang=english",
            "This English comment should pass",
            "Another English line",
        ]);
        let s = analyze(&cs, "korean");
        assert!(s[0].skip);
        for st in &s[1..] {
            assert!(!st.skip);
            assert_eq!(st.language, "english");
        }
    }

    #[test]
    fn file_lang_locale_code() {
        let cs = comments(&["git-warden:file-lang=zh", "这是一个中文注释内容示例"]);
        let s = analyze(&cs, "korean");
        assert_eq!(s[1].language, "chinese");
    }

    #[test]
    fn file_lang_with_disable() {
        let cs = comments(&[
            "git-warden:file-lang=english",
            "English comment",
            "git-warden:disable",
            "skipped comment",
            "git-warden:enable",
            "English resumed",
        ]);
        let s = analyze(&cs, "korean");
        assert_eq!(s[1].language, "english");
        assert!(s[3].skip);
        assert_eq!(s[5].language, "english");
    }

    #[test]
    fn case_insensitive() {
        let cs = comments(&[
            "Git-Warden:Ignore",
            "This should be skipped",
            "GIT-WARDEN:DISABLE",
            "Also skipped",
            "git-warden:enable",
            "Checked",
        ]);
        let s = analyze(&cs, "korean");
        assert!(s[1].skip);
        assert!(s[3].skip);
        assert!(!s[5].skip);
    }

    #[test]
    fn is_directive_helper() {
        for s in [
            "git-warden:ignore",
            "git-warden:disable",
            "git-warden:enable",
            "git-warden:lang=english",
            "git-warden:file-lang=ko",
            "GIT-WARDEN:DISABLE",
            "  git-warden:ignore  ",
        ] {
            assert!(is_directive(s), "{s}");
        }
        for s in [
            "한국어 주석입니다",
            "This is a comment",
            "// nolint",
            "commit",
            "checker:ignore",
        ] {
            assert!(!is_directive(s), "{s}");
        }
    }
}
