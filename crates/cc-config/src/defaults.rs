//! defaults.rs: apply defaults and normalize locales for loaded config. Corresponds to Go `internal/config/defaults.go`.

use crate::types::Config;

pub fn apply_defaults(cfg: &mut Config) {
    // CommentLanguage: normalize to Locale > RequiredLanguage > "korean".
    {
        let mut lang = cc_langdetect::normalize_locale(&cfg.comment_language.locale);
        if lang.is_empty() {
            lang = cc_langdetect::normalize_locale(&cfg.comment_language.required_language);
        }
        if lang.is_empty() {
            lang = cc_langdetect::KOREAN.to_string();
        }
        cfg.comment_language.locale = lang.clone();
        cfg.comment_language.required_language = lang;
    }

    // FileLanguages: normalize Locale/Language for each entry.
    for r in cfg.comment_language.file_languages.iter_mut() {
        let mut lang = cc_langdetect::normalize_locale(&r.locale);
        if lang.is_empty() {
            lang = cc_langdetect::normalize_locale(&r.language);
        }
        if !lang.is_empty() {
            r.locale = lang.clone();
            r.language = lang;
        }
    }

    if cfg.comment_language.extensions.is_empty() && cfg.comment_language.languages.is_empty() {
        cfg.comment_language.extensions = [
            ".go",
            ".ts",
            ".tsx",
            ".js",
            ".jsx",
            ".mjs",
            ".java",
            ".kt",
            ".py",
            ".c",
            ".cpp",
            ".cs",
            ".swift",
            ".rs",
            ".hcl",
            ".tf",
            ".tfvars",
            "dockerfile",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
    }
    if cfg.comment_language.min_length == 0 {
        cfg.comment_language.min_length = 5;
    }
    if cfg.comment_language.check_mode.is_empty() {
        cfg.comment_language.check_mode = "diff".to_string();
    }
    if cfg.encoding.locale.is_empty() {
        cfg.encoding.locale = "ko".to_string();
    }
    if cfg.commit_message.locale.is_empty() {
        cfg.commit_message.locale = "ko".to_string();
    }

    // CommitMessage.LanguageCheck: Locale > RequiredLanguage > CommitMessage.Locale > "korean".
    {
        let mut lang = cc_langdetect::normalize_locale(&cfg.commit_message.language_check.locale);
        if lang.is_empty() {
            lang = cc_langdetect::normalize_locale(
                &cfg.commit_message.language_check.required_language,
            );
        }
        if lang.is_empty() {
            lang = cc_langdetect::normalize_locale(&cfg.commit_message.locale);
        }
        if lang.is_empty() {
            lang = cc_langdetect::KOREAN.to_string();
        }
        cfg.commit_message.language_check.locale = lang.clone();
        cfg.commit_message.language_check.required_language = lang;
    }
    if cfg.commit_message.language_check.min_length == 0 {
        cfg.commit_message.language_check.min_length = 5;
    }
    if cfg.commit_message.language_check.skip_prefixes.is_empty() {
        cfg.commit_message.language_check.skip_prefixes = ["Merge", "Revert", "fixup!", "squash!"]
            .iter()
            .map(|s| s.to_string())
            .collect();
    }
}
