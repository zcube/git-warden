//! cc-config: config loading, merging, migration, and validation. Corresponds to Go `internal/config`.
//!
//! Project config (.git-warden.yml) takes priority; falls back to global config.
//! Merges preset/include/allowed_words and applies defaults. Schema version is
//! auto-detected and migrated.

mod accessors;
mod allowed_words;
mod cache;
mod defaults;
mod error;
mod include;
mod loader;
mod merge;
pub mod schema;
mod schema_check;
mod types;
mod validate;

pub use accessors::extract_coauthor_email;
pub use loader::{global_config_path, load};
pub use merge::merge_configs;
pub use schema_check::{validate_against_schema, validate_schema_file};
pub use types::*;
pub use validate::validate;

// Re-exports for tests and advanced use.
pub use allowed_words::parse_word_lines;
pub use defaults::apply_defaults;
pub use include::expand_tilde;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_accessors() {
        let cfg = Config::default();
        assert!(cfg.is_enabled());
        assert!(cfg.comment_language.is_enabled());
        assert!(cfg.commit_message.is_enabled());
        assert!(cfg.commit_message.is_no_ai_coauthor());
        assert!(!cfg.commit_message.is_no_emoji());
        assert!(cfg.binary_file.is_enabled());
        assert!(cfg.lint.is_enabled());
        assert!(cfg.encoding.is_enabled());
        assert!(cfg.encoding.is_require_utf8());
        assert!(!cfg.encoding.is_no_invisible_chars());
        assert!(cfg.cache_dir.is_enabled());
        assert!(cfg.guide.is_enabled());
        assert!(!cfg.commit_message.conventional_commit.is_enabled());
        assert!(!cfg.protected_paths.is_enabled());
        assert!(!cfg.append_only.is_enabled());
    }

    #[test]
    fn apply_defaults_locale_and_extensions() {
        let mut cfg = Config::default();
        apply_defaults(&mut cfg);
        assert_eq!(cfg.comment_language.locale, "korean");
        assert_eq!(cfg.comment_language.min_length, 5);
        assert_eq!(cfg.comment_language.check_mode, "diff");
        assert!(cfg.comment_language.extensions.contains(&".go".to_string()));
        assert_eq!(cfg.encoding.locale, "ko");
        assert_eq!(
            cfg.commit_message.language_check.skip_prefixes,
            vec!["Merge", "Revert", "fixup!", "squash!"]
        );
    }

    #[test]
    fn binary_policy_for() {
        let mut cfg = Config::default();
        cfg.binary_file.default_policy = "block".to_string();
        assert_eq!(cfg.binary_file.policy_for("a.png"), "allow");
        assert_eq!(cfg.binary_file.policy_for("a.out"), "block");
    }

    #[test]
    fn coauthor_should_remove_builtin() {
        let cfg = CommitMessageConfig::default();
        assert!(cfg.coauthor_should_remove("noreply@anthropic.com"));
        assert!(cfg.coauthor_should_remove("foo-copilot@users.noreply.github.com"));
        assert!(!cfg.coauthor_should_remove("human@example.com"));
    }

    #[test]
    fn extract_email() {
        assert_eq!(
            extract_coauthor_email("Co-authored-by: X <x@y.com>"),
            "x@y.com"
        );
        assert_eq!(extract_coauthor_email("no email here"), "");
    }

    #[test]
    fn merge_overlay_wins_lists_concat() {
        let mut base = Config::default();
        base.comment_language.allowed_words = vec!["Base".into()];
        base.comment_language.locale = "english".into();
        let mut overlay = Config::default();
        overlay.comment_language.allowed_words = vec!["Over".into()];
        overlay.comment_language.locale = "korean".into();
        let merged = merge_configs(&base, &overlay);
        assert_eq!(merged.comment_language.locale, "korean");
        assert_eq!(
            merged.comment_language.allowed_words,
            vec!["Base".to_string(), "Over".to_string()]
        );
    }
}
