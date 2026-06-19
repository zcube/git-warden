//! Cache/build directory checks. Corresponds to Go `internal/checker/cache_dir.go`.

use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use cc_config::Config;

fn base_name(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn rel_to(root: &Path, target: &Path) -> String {
    target
        .strip_prefix(root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| target.to_string_lossy().to_string())
}

/// Blocks files inside cache/build directories from being staged. Corresponds to Go `CheckCacheDirStaged`.
pub fn check_cache_dir_staged(
    cfg: &Config,
    diffs: &[cc_gitdiff::FileDiff],
    cancel: &AtomicBool,
) -> Result<Vec<String>, String> {
    if !cfg.cache_dir.is_enabled() {
        return Ok(Vec::new());
    }
    let repo_root = match cc_cachedir::find_repo_root(Path::new(".")) {
        Ok(r) => r,
        Err(_) => return Ok(Vec::new()),
    };
    let ignore_dirs: HashSet<String> = cfg.cache_dir.ignore_dirs.iter().cloned().collect();

    let mut errs = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for d in diffs {
        crate::check_cancelled(cancel)?;
        if d.is_deleted {
            continue;
        }
        if cc_pathutil::matches_any(&d.path, &cfg.exceptions.global_ignore) {
            continue;
        }
        let abs_path = repo_root.join(&d.path);
        let cache_dir = match cc_cachedir::find_cache_dir_ancestor(&repo_root, &abs_path) {
            Some(c) => c,
            None => continue,
        };
        if ignore_dirs.contains(&base_name(&cache_dir)) {
            continue;
        }
        let rel = rel_to(&repo_root, &cache_dir);
        if seen.contains(&rel) {
            continue;
        }
        seen.insert(rel.clone());
        errs.push(cc_i18n::t!(
            "diff.cache_dir_staged",
            Path = d.path,
            CacheDir = rel
        ));
    }
    Ok(errs)
}

/// Reports tracked files that reside inside cache/build directories. Corresponds to Go `CheckCacheDirCommitted`.
pub fn check_cache_dir_committed(cfg: &Config, cancel: &AtomicBool) -> Result<Vec<String>, String> {
    if !cfg.cache_dir.is_enabled() {
        return Ok(Vec::new());
    }
    let repo_root = match cc_cachedir::find_repo_root(Path::new(".")) {
        Ok(r) => r,
        Err(_) => return Ok(Vec::new()),
    };
    let cache_dirs = cc_cachedir::find_cache_dirs_in_repo(&repo_root);
    if cache_dirs.is_empty() {
        return Ok(Vec::new());
    }
    let ignore_dirs: HashSet<String> = cfg.cache_dir.ignore_dirs.iter().cloned().collect();

    let mut errs = Vec::new();
    for dir in &cache_dirs {
        crate::check_cancelled(cancel)?;
        if ignore_dirs.contains(&base_name(dir)) {
            continue;
        }
        let tracked = match cc_cachedir::list_tracked_entries(&repo_root, dir) {
            Ok(t) if !t.is_empty() => t,
            _ => continue,
        };
        let rel = rel_to(&repo_root, dir);
        if cc_pathutil::matches_any(&rel, &cfg.exceptions.global_ignore) {
            continue;
        }
        errs.push(cc_i18n::t!(
            "diff.cache_dir_committed",
            CacheDir = rel,
            Count = tracked.len()
        ));
    }
    Ok(errs)
}
