//! merge.rs: merges overlay config on top of base (preset/include). Corresponds to Go `internal/config/merge.go`.

use crate::types::Config;

/// Returns a Config merged with overlay on top of base. Overlay explicit values take priority; lists are concatenated.
/// Corresponds to Go `mergeConfigs`.
pub fn merge_configs(base: &Config, overlay: &Config) -> Config {
    let mut result = overlay.clone();

    merge_bool(&mut result.enabled, base.enabled);

    // comment language
    merge_bool(
        &mut result.comment_language.enabled,
        base.comment_language.enabled,
    );
    merge_bool(
        &mut result.comment_language.no_emoji,
        base.comment_language.no_emoji,
    );
    merge_bool(
        &mut result.comment_language.check_strings,
        base.comment_language.check_strings,
    );
    merge_bool(
        &mut result.comment_language.skip_technical_strings,
        base.comment_language.skip_technical_strings,
    );
    merge_string(
        &mut result.comment_language.required_language,
        &base.comment_language.required_language,
    );
    merge_string(
        &mut result.comment_language.check_mode,
        &base.comment_language.check_mode,
    );
    merge_string(
        &mut result.comment_language.locale,
        &base.comment_language.locale,
    );
    merge_string(
        &mut result.comment_language.allowed_words_file,
        &base.comment_language.allowed_words_file,
    );
    merge_string(
        &mut result.comment_language.allowed_words_url,
        &base.comment_language.allowed_words_url,
    );
    merge_int(
        &mut result.comment_language.min_length,
        base.comment_language.min_length,
    );
    result.comment_language.allowed_words = concat(
        &base.comment_language.allowed_words,
        &result.comment_language.allowed_words,
    );
    result.comment_language.skip_directives = concat(
        &base.comment_language.skip_directives,
        &result.comment_language.skip_directives,
    );
    result.comment_language.ignore_files = concat(
        &base.comment_language.ignore_files,
        &result.comment_language.ignore_files,
    );
    if result.comment_language.languages.is_empty() {
        result.comment_language.languages = base.comment_language.languages.clone();
    }
    if result.comment_language.extensions.is_empty() {
        result.comment_language.extensions = base.comment_language.extensions.clone();
    }
    if result.comment_language.file_languages.is_empty() {
        result.comment_language.file_languages = base.comment_language.file_languages.clone();
    }

    // commit message
    merge_bool(
        &mut result.commit_message.enabled,
        base.commit_message.enabled,
    );
    merge_bool(
        &mut result.commit_message.no_ai_coauthor,
        base.commit_message.no_ai_coauthor,
    );
    merge_bool(
        &mut result.commit_message.no_unicode_spaces,
        base.commit_message.no_unicode_spaces,
    );
    merge_bool(
        &mut result.commit_message.no_ambiguous_chars,
        base.commit_message.no_ambiguous_chars,
    );
    merge_bool(
        &mut result.commit_message.no_bad_runes,
        base.commit_message.no_bad_runes,
    );
    merge_bool(
        &mut result.commit_message.no_emoji,
        base.commit_message.no_emoji,
    );
    merge_string(
        &mut result.commit_message.locale,
        &base.commit_message.locale,
    );
    result.commit_message.coauthor_remove_emails = concat(
        &base.commit_message.coauthor_remove_emails,
        &result.commit_message.coauthor_remove_emails,
    );

    // binary file
    merge_bool(&mut result.binary_file.enabled, base.binary_file.enabled);
    result.binary_file.ignore_files = concat(
        &base.binary_file.ignore_files,
        &result.binary_file.ignore_files,
    );

    // encoding
    merge_bool(&mut result.encoding.enabled, base.encoding.enabled);
    merge_bool(
        &mut result.encoding.require_utf8,
        base.encoding.require_utf8,
    );
    merge_bool(
        &mut result.encoding.no_invisible_chars,
        base.encoding.no_invisible_chars,
    );
    merge_bool(
        &mut result.encoding.no_ambiguous_chars,
        base.encoding.no_ambiguous_chars,
    );
    merge_string(&mut result.encoding.locale, &base.encoding.locale);
    result.encoding.ignore_files =
        concat(&base.encoding.ignore_files, &result.encoding.ignore_files);

    // EditorConfig
    merge_bool(&mut result.editorconfig.enabled, base.editorconfig.enabled);
    result.editorconfig.ignore_files = concat(
        &base.editorconfig.ignore_files,
        &result.editorconfig.ignore_files,
    );

    // Exceptions: always concatenate
    result.exceptions.global_ignore = concat(
        &base.exceptions.global_ignore,
        &result.exceptions.global_ignore,
    );
    result.exceptions.comment_language_ignore = concat(
        &base.exceptions.comment_language_ignore,
        &result.exceptions.comment_language_ignore,
    );

    // CustomRules: always concatenate (base first)
    result.custom_rules.commit_message = concat(
        &base.custom_rules.commit_message,
        &result.custom_rules.commit_message,
    );
    result.custom_rules.diff = concat(&base.custom_rules.diff, &result.custom_rules.diff);

    // protected paths: OR enabled, concatenate paths
    if !result.protected_paths.enabled {
        result.protected_paths.enabled = base.protected_paths.enabled;
    }
    result.protected_paths.paths =
        concat(&base.protected_paths.paths, &result.protected_paths.paths);

    // guide
    merge_bool(&mut result.guide.enabled, base.guide.enabled);

    result
}

fn merge_bool(dst: &mut Option<bool>, src: Option<bool>) {
    if dst.is_none() && src.is_some() {
        *dst = src;
    }
}

fn merge_string(dst: &mut String, src: &str) {
    if dst.is_empty() && !src.is_empty() {
        *dst = src.to_string();
    }
}

fn merge_int(dst: &mut i64, src: i64) {
    if *dst == 0 && src != 0 {
        *dst = src;
    }
}

fn concat<T: Clone>(base: &[T], overlay: &[T]) -> Vec<T> {
    let mut v = base.to_vec();
    v.extend_from_slice(overlay);
    v
}
