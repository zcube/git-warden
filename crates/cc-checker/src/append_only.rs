//! Append-only path checks. Corresponds to Go `internal/checker/append_only.go`.

use std::process::Command;
use std::sync::atomic::AtomicBool;

use cc_config::Config;
use cc_gitdiff::FileDiff;

/// Sentinel indicating the file is absent in the comparison base tree.
const ERR_NOT_IN_FROM_TREE: &str = "__not_in_from_tree__";

/// Checks staged diff for append-only path violations. Corresponds to Go `CheckAppendOnly`.
pub fn check_append_only(
    cfg: &Config,
    diffs: &[FileDiff],
    cancel: &AtomicBool,
) -> Result<Vec<String>, String> {
    if !cfg.append_only.is_enabled() {
        return Ok(Vec::new());
    }

    let spec = cc_gitdiff::current_spec();
    let from_ref = if spec.from.is_empty() {
        "HEAD".to_string()
    } else {
        spec.from
    };
    let tree_ref = resolve_tree_ref(&from_ref);

    let ignore_patterns = &cfg.exceptions.global_ignore;
    let mut errs = Vec::new();

    for d in diffs {
        crate::check_cancelled(cancel)?;
        if !cc_pathutil::matches_any(&d.path, &cfg.append_only.paths) {
            continue;
        }
        if cc_pathutil::matches_any(&d.path, ignore_patterns) {
            continue;
        }

        if d.is_deleted {
            errs.push(cc_i18n::t!("diff.append_only_deleted", Path = d.path));
            continue;
        }

        if d.is_new {
            if cfg.append_only.is_filename_order_numeric() {
                if let Some(msg) = check_filename_order(&tree_ref, &d.path, &cfg.append_only.paths)
                {
                    errs.push(msg);
                }
            }
            continue;
        }

        match check_file_content(&tree_ref, &d.path) {
            Ok(Some(key)) => {
                errs.push(cc_i18n::t!(&key, Path = d.path));
            }
            Ok(None) => {}
            Err(e) if e == ERR_NOT_IN_FROM_TREE => continue,
            Err(e) => return Err(format!("append-only check {}: {e}", d.path)),
        }
    }
    Ok(errs)
}

/// Returns the ref unchanged if it resolves to a commit, otherwise "". Corresponds to Go `resolveTreeRef`.
fn resolve_tree_ref(reference: &str) -> String {
    let ok = Command::new("git")
        .args([
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{reference}^{{commit}}"),
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if ok {
        reference.to_string()
    } else {
        String::new()
    }
}

/// Lists all file paths in the treeRef tree. Corresponds to Go `listTreeFiles`.
fn list_tree_files(tree_ref: &str) -> Vec<String> {
    let out = match Command::new("git")
        .args(["ls-tree", "-r", "--name-only", "-z", tree_ref])
        .output()
    {
        Ok(o) if o.status.success() => o.stdout,
        _ => return Vec::new(),
    };
    cc_gitdiff::split_null_separated(&out)
}

/// Directory portion of a '/'-separated path (equivalent to Go path.Dir).
fn path_dir(p: &str) -> String {
    match p.rfind('/') {
        Some(i) => p[..i].to_string(),
        None => ".".to_string(),
    }
}

/// Last element of a '/'-separated path (equivalent to Go path.Base).
fn path_base(p: &str) -> String {
    match p.rfind('/') {
        Some(i) => p[i + 1..].to_string(),
        None => p.to_string(),
    }
}

/// Checks that the new filename sorts after existing files in the same directory. Corresponds to Go `checkFilenameOrder`.
fn check_filename_order(tree_ref: &str, new_path: &str, patterns: &[String]) -> Option<String> {
    if tree_ref.is_empty() {
        return None;
    }
    let new_dir = path_dir(new_path);
    let new_base = path_base(new_path);

    let mut max_existing = String::new();
    for name in list_tree_files(tree_ref) {
        if path_dir(&name) != new_dir {
            continue;
        }
        if !cc_pathutil::matches_any(&name, patterns) {
            continue;
        }
        let base = path_base(&name);
        if max_existing.is_empty() || natural_less(&max_existing, &base) {
            max_existing = base;
        }
    }

    if max_existing.is_empty() {
        return None;
    }
    if !natural_less(&max_existing, &new_base) {
        let max_file = if new_dir == "." {
            max_existing.clone()
        } else {
            format!("{new_dir}/{max_existing}")
        };
        return Some(cc_i18n::t!(
            "diff.append_only_filename_order",
            Path = new_path,
            MaxFile = max_file
        ));
    }
    None
}

/// Compares HEAD content with staged content and returns a violation i18n key if found. Corresponds to Go `checkFileContent`.
fn check_file_content(tree_ref: &str, file_path: &str) -> Result<Option<String>, String> {
    if tree_ref.is_empty() {
        return Ok(None);
    }
    let obj_spec = format!("{tree_ref}:{file_path}");

    let exists = Command::new("git")
        .args(["cat-file", "-e", &obj_spec])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !exists {
        return Err(ERR_NOT_IN_FROM_TREE.to_string());
    }

    let out = Command::new("git")
        .args(["cat-file", "blob", &obj_spec])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("git cat-file {obj_spec}"));
    }
    let head_content = String::from_utf8_lossy(&out.stdout).to_string();

    let staged_content = cc_gitdiff::get_staged_content(file_path)?;

    // Staged content must start with HEAD content (prefix match = no deletions, modifications, or mid-insertions).
    if !staged_content.starts_with(&head_content) {
        return Ok(Some("diff.append_only_modified".to_string()));
    }
    Ok(None)
}

/// Returns true if a comes before b in natural sort order. Corresponds to Go `naturalLess`.
fn natural_less(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    let mut ai = 0;
    let mut bi = 0;
    loop {
        if bi >= b.len() {
            return false;
        }
        if ai >= a.len() {
            return true;
        }
        let a_digit = a[ai].is_ascii_digit();
        let b_digit = b[bi].is_ascii_digit();
        if a_digit && b_digit {
            let a_end = numeric_run_end(&a[ai..]) + ai;
            let b_end = numeric_run_end(&b[bi..]) + bi;
            let a_num = parse_uint(&a[ai..a_end]);
            let b_num = parse_uint(&b[bi..b_end]);
            if a_num != b_num {
                return a_num < b_num;
            }
            ai = a_end;
            bi = b_end;
        } else {
            if a[ai] != b[bi] {
                return a[ai] < b[bi];
            }
            ai += 1;
            bi += 1;
        }
    }
}

fn numeric_run_end(s: &[u8]) -> usize {
    let mut i = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    i
}

fn parse_uint(s: &[u8]) -> u64 {
    let mut n: u64 = 0;
    for &b in s {
        n = n.wrapping_mul(10).wrapping_add((b - b'0') as u64);
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_less_cases() {
        assert!(natural_less("9.sql", "10.sql"));
        assert!(!natural_less("10.sql", "9.sql"));
        assert!(natural_less("001.sql", "002.sql"));
        assert!(natural_less("a", "b"));
        assert!(!natural_less("b", "a"));
        assert!(natural_less("", "x"));
        assert!(!natural_less("x", ""));
        assert!(natural_less("v1_2", "v1_10"));
    }
}
