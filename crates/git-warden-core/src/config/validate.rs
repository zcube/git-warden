//! validate.rs: validates config values and returns warnings. Corresponds to Go `internal/config/validate.go`.
//! Warning messages are translated from the Go original.

use super::types::Config;

fn is_valid_language(v: &str) -> bool {
    matches!(
        v.to_lowercase().as_str(),
        "korean"
            | "english"
            | "japanese"
            | "chinese"
            | "any"
            | "ko"
            | "en"
            | "ja"
            | "zh"
            | "zh-hans"
            | "zh-hant"
    )
}

fn check_locale(warns: &mut Vec<String>, cfg_path: &str, section: &str, field: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    if !is_valid_language(value) {
        warns.push(format!(
            "{cfg_path}: {section}.{field} unknown value: {value:?} (ko/en/ja/zh or korean/english/japanese/chinese/any)"
        ));
    }
}

/// Detects Go filepath.Match ErrBadPattern conditions (unclosed character class `[`).
fn glob_pattern_invalid(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let mut i = 0;
    let n = bytes.len();
    while i < n {
        match bytes[i] as char {
            '\\' => {
                // Escape: consume next character. Truncated at end is bad.
                i += 1;
                if i >= n {
                    return true;
                }
                i += 1;
            }
            '[' => {
                // Character class: must be closed with ']'.
                i += 1;
                if i < n && (bytes[i] as char == '^' || bytes[i] as char == '!') {
                    i += 1;
                }
                // First ']' is allowed as a literal.
                if i < n && bytes[i] as char == ']' {
                    i += 1;
                }
                let mut closed = false;
                while i < n {
                    let c = bytes[i] as char;
                    if c == '\\' {
                        i += 2;
                        continue;
                    }
                    if c == ']' {
                        closed = true;
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                if !closed {
                    return true;
                }
            }
            _ => i += 1,
        }
    }
    false
}

/// Validates config values and returns a list of warnings (not errors). Corresponds to Go `Validate`.
pub fn validate(cfg: &Config, cfg_path: &str) -> Vec<String> {
    let mut warns = Vec::new();

    check_locale(
        &mut warns,
        cfg_path,
        "comment_language",
        "locale",
        &cfg.comment_language.locale,
    );
    check_locale(
        &mut warns,
        cfg_path,
        "comment_language",
        "required_language",
        &cfg.comment_language.required_language,
    );

    if cfg.commit_message.language_check.is_enabled() {
        check_locale(
            &mut warns,
            cfg_path,
            "commit_message.language_check",
            "locale",
            &cfg.commit_message.language_check.locale,
        );
        check_locale(
            &mut warns,
            cfg_path,
            "commit_message.language_check",
            "required_language",
            &cfg.commit_message.language_check.required_language,
        );
    }
    for (i, fl) in cfg.comment_language.file_languages.iter().enumerate() {
        let section = format!("comment_language.file_languages[{i}]");
        check_locale(&mut warns, cfg_path, &section, "locale", &fl.locale);
        check_locale(&mut warns, cfg_path, &section, "language", &fl.language);
        if glob_pattern_invalid(&fl.pattern) {
            warns.push(format!(
                "{cfg_path}: {section}.pattern invalid glob pattern: {:?} (syntax error in pattern)",
                fl.pattern
            ));
        }
    }

    let mut check_globs = |section: &str, patterns: &[String]| {
        for p in patterns {
            if glob_pattern_invalid(p) {
                warns.push(format!(
                    "{cfg_path}: {section} invalid glob pattern: {p:?} (syntax error in pattern)"
                ));
            }
        }
    };
    check_globs(
        "comment_language.ignore_files",
        &cfg.comment_language.ignore_files,
    );
    check_globs("binary_file.ignore_files", &cfg.binary_file.ignore_files);
    check_globs("encoding.ignore_files", &cfg.encoding.ignore_files);
    check_globs("editorconfig.ignore_files", &cfg.editorconfig.ignore_files);
    check_globs("exceptions.global_ignore", &cfg.exceptions.global_ignore);
    check_globs(
        "exceptions.comment_language_ignore",
        &cfg.exceptions.comment_language_ignore,
    );
    check_globs("lint.yaml.ignore_files", &cfg.lint.yaml.ignore_files);
    check_globs("lint.json.ignore_files", &cfg.lint.json.ignore_files);
    check_globs("lint.xml.ignore_files", &cfg.lint.xml.ignore_files);

    for p in &cfg.commit_message.coauthor_remove_emails {
        let low = p.trim().to_lowercase();
        if glob_pattern_invalid(&low) {
            warns.push(format!(
                "{cfg_path}: commit_message.coauthor_remove_emails invalid glob pattern: {p:?} (syntax error in pattern)"
            ));
        }
    }

    if !cfg.comment_language.allowed_words_file.is_empty()
        && std::fs::metadata(&cfg.comment_language.allowed_words_file).is_err()
    {
        warns.push(format!(
            "{cfg_path}: comment_language.allowed_words_file file not found: {:?}",
            cfg.comment_language.allowed_words_file
        ));
    }

    warns
}
