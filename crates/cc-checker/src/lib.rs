//! cc-checker: core check logic for git-warden. Corresponds to Go `internal/checker`.
//!
//! - Run* functions check all tracked files (rayon parallel).
//! - Check* functions check staged diffs (sequential).
//! - Message checks (check_msg) / auto-fixes (fix_*) take commit message byte slices.
//!
//! Go's context.Context cancellation is replaced by `&AtomicBool` (true = cancelled).

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;

mod append_only;
mod binary;
mod cache_dir;
mod conventional;
mod custom;
mod diff_check;
mod editorconfig_check;
mod encoding_check;
mod fix;
mod lint_check;
mod msg;
mod protected;
mod run;
mod staged;
mod unicode_check;

// Re-export public API.
pub use append_only::check_append_only;
pub use binary::check_binary_files;
pub use cache_dir::{check_cache_dir_committed, check_cache_dir_staged};
pub use custom::{check_diff_custom_rules, check_msg_custom_rules};
pub use diff_check::{
    check_diff, is_all_uppercase_ascii, is_path_like_string, is_technical_string, truncate,
};
pub use editorconfig_check::check_editorconfig;
pub use encoding_check::check_encoding;
pub use fix::{fix_file_content, fix_msg, FixResult};
pub use lint_check::{check_lint, run_lint};
pub use msg::check_msg;
pub use protected::check_protected_paths;
pub use run::{run_binary_files, run_comment_language, run_editorconfig, run_encoding};
pub use unicode_check::{check_unicode, run_unicode};

/// Sentinel error string returned on cancellation.
pub(crate) const INTERRUPTED: &str = "interrupted";

/// Returns the full list of tracked files via git ls-files. Corresponds to Go `GetTrackedFiles`.
pub fn get_tracked_files() -> Result<Vec<String>, String> {
    let out = Command::new("git")
        .args(["ls-files", "-z"])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() && out.stdout.is_empty() {
        return Err("git ls-files failed".to_string());
    }
    Ok(cc_gitdiff::split_null_separated(&out.stdout))
}

/// Iterates files in parallel and collects violation messages returned by fn. Corresponds to Go `forEachFileConcurrent`.
/// Merges results preserving input order. Returns Err on cancellation.
pub(crate) fn for_each_file_concurrent<F>(
    files: &[String],
    cancel: &AtomicBool,
    f: F,
) -> Result<Vec<String>, String>
where
    F: Fn(&str) -> Result<Vec<String>, String> + Sync,
{
    let results: Vec<Result<Vec<String>, String>> = files
        .par_iter()
        .map(|path| {
            if cancel.load(Ordering::SeqCst) {
                return Ok(Vec::new());
            }
            f(path)
        })
        .collect();

    let mut errs = Vec::new();
    for r in results {
        errs.extend(r?);
    }
    if cancel.load(Ordering::SeqCst) {
        return Err(INTERRUPTED.to_string());
    }
    Ok(errs)
}

/// Cancellation check helper (for sequential loops).
pub(crate) fn check_cancelled(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::SeqCst) {
        Err(INTERRUPTED.to_string())
    } else {
        Ok(())
    }
}
