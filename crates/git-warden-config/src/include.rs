//! include.rs: conditional config inclusion similar to git includeIf. Corresponds to Go `internal/config/include.go`.

use crate::loader::parse_yaml_config;
use crate::merge::merge_configs;
use crate::types::Config;
use std::path::{Path, PathBuf};

/// Resolves cfg.include entries as base layers and merges them with the main config. Corresponds to Go `resolveIncludes`.
pub fn resolve_includes(cfg: &Config, cfg_path: &str) -> Config {
    if cfg.include.is_empty() {
        return cfg.clone();
    }
    let work_dir = match std::env::current_dir() {
        Ok(d) => d.to_string_lossy().to_string(),
        Err(e) => {
            git_warden_logger::warn(
                "include: failed to determine working directory, skipping gitdir conditional includes",
                &[("config", cfg_path.to_string()), ("error", e.to_string())],
            );
            String::new()
        }
    };

    let mut base: Option<Config> = None;
    for rule in &cfg.include {
        if rule.path.is_empty() {
            git_warden_logger::warn(
                "include: skipping entry with empty path",
                &[("config", cfg_path.to_string())],
            );
            continue;
        }
        if !rule.gitdir.is_empty()
            && (work_dir.is_empty() || !gitdir_match(&rule.gitdir, &work_dir))
        {
            continue;
        }
        let Some(inc) = load_include_file(&rule.path, cfg_path) else {
            continue;
        };
        base = Some(match base {
            None => inc,
            Some(b) => merge_configs(&b, &inc),
        });
    }
    match base {
        None => cfg.clone(),
        Some(b) => merge_configs(&b, cfg),
    }
}

fn load_include_file(path: &str, cfg_path: &str) -> Option<Config> {
    let resolved = resolve_include_path(path, cfg_path);
    let data = match std::fs::read(&resolved) {
        Ok(d) => d,
        Err(e) => {
            git_warden_logger::warn(
                "include: cannot read file, skipping",
                &[
                    ("path", resolved.to_string_lossy().to_string()),
                    ("error", e.to_string()),
                ],
            );
            return None;
        }
    };
    let mut inc = match parse_yaml_config(&data) {
        Ok(c) => c,
        Err(e) => {
            git_warden_logger::warn(
                "include: YAML parse failed, skipping",
                &[
                    ("path", resolved.to_string_lossy().to_string()),
                    ("error", e),
                ],
            );
            return None;
        }
    };
    if !inc.include.is_empty() {
        git_warden_logger::warn(
            "include: nested includes are not supported, ignoring",
            &[("path", resolved.to_string_lossy().to_string())],
        );
        inc.include.clear();
    }
    Some(inc)
}

fn resolve_include_path(path: &str, cfg_path: &str) -> PathBuf {
    let p = expand_tilde(path);
    let pb = PathBuf::from(&p);
    if pb.is_absolute() {
        clean(&pb)
    } else {
        let dir = Path::new(cfg_path).parent().unwrap_or(Path::new("."));
        clean(&dir.join(&pb))
    }
}

/// Checks work_dir matching using git includeIf "gitdir:" semantics. Corresponds to Go `gitdirMatch`.
fn gitdir_match(pattern: &str, work_dir: &str) -> bool {
    let dir_suffix = pattern.ends_with('/');
    let mut p = to_slash(&expand_tilde(pattern));
    if dir_suffix {
        p = format!("{}/**", p.trim_end_matches('/'));
    }
    p = resolve_symlink_prefix(&p);

    let wd = match std::fs::canonicalize(work_dir) {
        Ok(r) => r.to_string_lossy().to_string(),
        Err(_) => work_dir.to_string(),
    };
    git_warden_pathutil::match_path(&to_slash(&wd), &p)
}

fn resolve_symlink_prefix(pattern: &str) -> String {
    let segs: Vec<&str> = pattern.split('/').collect();
    let mut lit = 0;
    while lit < segs.len() {
        if segs[lit].contains(['*', '?', '[']) {
            break;
        }
        lit += 1;
    }
    if lit == 0 {
        return pattern.to_string();
    }
    let prefix = segs[..lit].join("/");
    if prefix.is_empty() {
        return pattern.to_string();
    }
    let resolved = match std::fs::canonicalize(&prefix) {
        Ok(r) => to_slash(&r.to_string_lossy()),
        Err(_) => return pattern.to_string(),
    };
    let rest = &segs[lit..];
    if rest.is_empty() {
        resolved
    } else {
        format!("{}/{}", resolved, rest.join("/"))
    }
}

/// Expands '~' or '~/' to the home directory. Corresponds to Go `expandTilde`.
pub fn expand_tilde(p: &str) -> String {
    if p == "~" {
        return dirs::home_dir()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|| p.to_string());
    }
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest).to_string_lossy().to_string();
        }
    }
    p.to_string()
}

fn to_slash(p: &str) -> String {
    p.replace('\\', "/")
}

// Normalizes . and .. components like filepath.Clean.
fn clean(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        use std::path::Component::*;
        match comp {
            CurDir => {}
            ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}
