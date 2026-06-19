//! Binary file policy checks. Corresponds to Go `internal/checker/binary.go`.

use std::process::Command;
use std::sync::atomic::AtomicBool;

use crate::config::{BinaryFileConfig, Config};

use super::staged::get_staged_binary_files;

/// Checks staged diff for binary files. Corresponds to Go `CheckBinaryFiles`.
pub fn check_binary_files(cfg: &Config, cancel: &AtomicBool) -> Result<Vec<String>, String> {
    if !cfg.binary_file.is_enabled() {
        return Ok(Vec::new());
    }

    let files = get_staged_binary_files()?;

    let mut ignore_patterns = cfg.exceptions.global_ignore.clone();
    ignore_patterns.extend(cfg.binary_file.ignore_files.iter().cloned());

    let mut errs = Vec::new();
    for path in &files {
        super::check_cancelled(cancel)?;
        if crate::pathutil::matches_any(path, &ignore_patterns) {
            continue;
        }
        let msg = evaluate_binary_policy(&cfg.binary_file, path);
        if !msg.is_empty() {
            errs.push(msg);
        }
    }
    Ok(errs)
}

/// Returns an i18n error message based on the policy for path (empty string if allow or lfs-tracked).
/// Corresponds to Go `evaluateBinaryPolicy`.
pub(crate) fn evaluate_binary_policy(cfg: &BinaryFileConfig, path: &str) -> String {
    match cfg.policy_for(path).as_str() {
        "allow" => String::new(),
        "lfs" => {
            if is_lfs_tracked(path) {
                String::new()
            } else {
                crate::t!("diff.binary_file_lfs_required", Path = path)
            }
        }
        // "block" and others
        _ => crate::t!("diff.binary_file_error", Path = path),
    }
}

/// Checks whether path is tracked by the git LFS filter. Corresponds to Go `isLFSTracked`.
fn is_lfs_tracked(path: &str) -> bool {
    let out = match Command::new("git")
        .args(["check-attr", "-z", "filter", "--", path])
        .output()
    {
        Ok(o) if o.status.success() => o.stdout,
        _ => return false,
    };
    // check-attr -z output: <path>\0filter\0<value>\0
    let s = String::from_utf8_lossy(&out);
    let trimmed = s.trim_end_matches('\u{0}');
    let parts: Vec<&str> = trimmed.split('\u{0}').collect();
    parts.len() >= 3 && parts[2] == "lfs"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BinaryFileConfig, BinaryFilePolicyRule};

    fn cfg_with(default_policy: &str, rules: Vec<BinaryFilePolicyRule>) -> BinaryFileConfig {
        BinaryFileConfig {
            enabled: Some(true),
            default_policy: default_policy.to_string(),
            rules,
            ignore_files: vec![],
        }
    }

    #[test]
    fn block_policy_reports() {
        let cfg = cfg_with("block", vec![]);
        let msg = evaluate_binary_policy(&cfg, "a.out");
        assert!(msg.contains("a.out"));
    }

    #[test]
    fn allow_policy_silent() {
        let cfg = cfg_with("allow", vec![]);
        assert_eq!(evaluate_binary_policy(&cfg, "a.out"), "");
    }

    #[test]
    fn image_extension_allowed_by_default() {
        let cfg = cfg_with("block", vec![]);
        // Built-in image extensions are allowed by default.
        assert_eq!(evaluate_binary_policy(&cfg, "logo.png"), "");
    }

    #[test]
    fn rule_overrides_default() {
        let cfg = cfg_with(
            "block",
            vec![BinaryFilePolicyRule {
                extensions: vec![".zip".to_string()],
                policy: "allow".to_string(),
            }],
        );
        assert_eq!(evaluate_binary_policy(&cfg, "data.zip"), "");
    }
}
