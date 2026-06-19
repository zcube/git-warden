//! Conventional Commits format checks. Corresponds to Go `internal/checker/conventional.go`.

use crate::config::ConventionalCommitConfig;
use once_cell::sync::Lazy;
use regex::Regex;

use super::diff_check::truncate;

// type[(scope)][!]: description. Unicode allowed in type (for localized type support).
static CONVENTIONAL_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([^\s(:!]+)(\([^)]*\))?(!)?: .+").unwrap());

/// Validates that the commit message subject conforms to the Conventional Commits spec. Corresponds to Go `checkConventional`.
pub(crate) fn check_conventional(content: &str, cfg: &ConventionalCommitConfig) -> Vec<String> {
    let first_line = content
        .trim_end_matches('\n')
        .split('\n')
        .next()
        .unwrap_or("");
    let subject = first_line.trim();
    if subject.is_empty() {
        return Vec::new();
    }

    if cfg.is_allow_merge_commits() && subject.starts_with("Merge ") {
        return Vec::new();
    }
    if cfg.is_allow_revert_commits() && subject.starts_with("Revert ") {
        return Vec::new();
    }
    if subject.starts_with("fixup! ") || subject.starts_with("squash! ") {
        return Vec::new();
    }

    let caps = match CONVENTIONAL_PATTERN.captures(subject) {
        Some(c) => c,
        None => {
            return vec![crate::t!(
                "msg.conventional_format_error",
                Subject = truncate(subject, 80)
            )];
        }
    };

    let commit_type = caps.get(1).map(|m| m.as_str()).unwrap_or("");
    let scope = caps.get(2).map(|m| m.as_str()).unwrap_or("");

    let all_types = cfg.get_all_allowed_types();
    let type_allowed = all_types
        .iter()
        .any(|t| t.eq_ignore_ascii_case(commit_type));
    if !type_allowed {
        return vec![crate::t!(
            "msg.conventional_type_error",
            Type = commit_type,
            Types = all_types.join(", ")
        )];
    }

    if cfg.is_require_scope() && (scope.is_empty() || scope == "()") {
        return vec![crate::t!(
            "msg.conventional_scope_required",
            Subject = truncate(subject, 80)
        )];
    }

    Vec::new()
}
