//! Protected path (fully frozen) checks. Corresponds to Go `internal/checker/protected.go`.

use std::sync::atomic::AtomicBool;

use crate::config::Config;
use crate::gitdiff::FileDiff;

/// Checks protected path violations in the staged diff. Corresponds to Go `CheckProtectedPaths`.
pub fn check_protected_paths(
    cfg: &Config,
    diffs: &[FileDiff],
    cancel: &AtomicBool,
) -> Result<Vec<String>, String> {
    if !cfg.protected_paths.is_enabled() {
        return Ok(Vec::new());
    }
    let ignore_patterns = &cfg.exceptions.global_ignore;
    let mut errs = Vec::new();
    for d in diffs {
        super::check_cancelled(cancel)?;
        if !crate::pathutil::matches_any(&d.path, &cfg.protected_paths.paths) {
            continue;
        }
        if crate::pathutil::matches_any(&d.path, ignore_patterns) {
            continue;
        }
        let key = if d.is_new {
            "diff.protected_path_added"
        } else if d.is_deleted {
            "diff.protected_path_deleted"
        } else {
            "diff.protected_path_modified"
        };
        errs.push(crate::t!(key, Path = d.path));
    }
    Ok(errs)
}
