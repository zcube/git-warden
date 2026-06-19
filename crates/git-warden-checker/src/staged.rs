//! Staged file list helpers. Corresponds to Go `getStagedFiles`/`getStagedBinaryFiles`.
//!
//! Also provides **raw byte** content reading for encoding/Unicode checks.
//! `git_warden_gitdiff::get_staged_content` returns a `String` (lossy-decoded), which loses invalid
//! UTF-8 bytes — raw bytes are needed for UTF-8 validity and binary detection.

use std::process::Command;

/// Returns the file content at the current_spec().to revision as raw bytes.
/// Byte-level version of git_warden_gitdiff::get_content_at (no cache).
pub(crate) fn staged_content_bytes(path: &str) -> Result<Vec<u8>, String> {
    let spec = git_warden_gitdiff::current_spec();
    content_at_bytes(&spec.to, path)
}

fn content_at_bytes(reference: &str, path: &str) -> Result<Vec<u8>, String> {
    match reference {
        "" | "index" | "staged" => {
            let out = Command::new("git")
                .args(["show", &format!(":{path}")])
                .output()
                .map_err(|e| e.to_string())?;
            if !out.status.success() {
                return Err(format!("git show :{path}"));
            }
            Ok(out.stdout)
        }
        git_warden_gitdiff::REF_WORKTREE | "wt" | "working-tree" => {
            std::fs::read(path).map_err(|e| e.to_string())
        }
        _ => {
            let out = Command::new("git")
                .args(["show", &format!("{reference}:{path}")])
                .output()
                .map_err(|e| e.to_string())?;
            if !out.status.success() {
                return Err(format!("git show {reference}:{path}"));
            }
            Ok(out.stdout)
        }
    }
}

/// Returns the list of staged file paths (excluding deletions). Corresponds to Go `getStagedFiles`.
pub(crate) fn get_staged_files() -> Result<Vec<String>, String> {
    let out = Command::new("git")
        .args(["diff", "--staged", "--name-only", "--diff-filter=d", "-z"])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() && out.stdout.is_empty() {
        return Err("git diff --staged --name-only failed".to_string());
    }
    Ok(git_warden_gitdiff::split_null_separated(&out.stdout))
}

/// Returns the list of staged files that git identifies as binary. Corresponds to Go `getStagedBinaryFiles`.
/// Items where added/deleted are "-" in the numstat -z output.
pub(crate) fn get_staged_binary_files() -> Result<Vec<String>, String> {
    let out = Command::new("git")
        .args(["diff", "--staged", "--numstat", "-z", "--diff-filter=d"])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() && out.stdout.is_empty() {
        return Err("git diff --staged --numstat failed".to_string());
    }

    let fields = git_warden_gitdiff::split_null_separated(&out.stdout);
    let mut binaries = Vec::new();
    let mut i = 0;
    while i < fields.len() {
        // Record format: "added\tdeleted\t<path>". Binary files have "-" for added/deleted.
        let parts: Vec<&str> = fields[i].splitn(3, '\t').collect();
        if parts.len() != 3 {
            i += 1;
            continue;
        }
        let mut path = parts[2].to_string();
        // Rename: path is empty and the next two NUL-separated fields are (old, new) paths.
        if path.is_empty() && i + 2 < fields.len() {
            path = fields[i + 2].clone();
            i += 2;
        }
        if parts[0] == "-" && parts[1] == "-" && !path.is_empty() {
            binaries.push(path);
        }
        i += 1;
    }
    Ok(binaries)
}
