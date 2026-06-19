//! Data file syntax lint (staged and tracked files). Corresponds to Go `internal/checker/lint.go` (CheckLint) + run.go (RunLint).

use std::path::Path;
use std::sync::atomic::AtomicBool;

use cc_config::Config;
use cc_lint::ValidationError;

use crate::staged::get_staged_files;

/// Returns the file extension in lowercase (including the leading '.'). Corresponds to Go filepath.Ext + ToLower.
fn ext_lower(path: &str) -> String {
    let base = Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    match base.rfind('.') {
        Some(i) => base[i..].to_lowercase(),
        None => String::new(),
    }
}

/// Runs lint validation on a single file, applying per-extension enable/ignore filters; returns a list of ValidationErrors.
/// `content` is the file content already read by the caller. Returns empty if the extension is not a target.
fn lint_validate(cfg: &Config, path: &str, content: &str) -> Vec<ValidationError> {
    match ext_lower(path).as_str() {
        ".yaml" | ".yml" => {
            if !cfg.lint.yaml.is_enabled() {
                return Vec::new();
            }
            if cc_pathutil::matches_any(path, &cfg.lint.yaml.ignore_files) {
                return Vec::new();
            }
            if cfg.lint.yaml.is_comment_filter() && cc_lint::has_lint_disable_comment(content, "#")
            {
                return Vec::new();
            }
            cc_lint::validate_yaml(path, content)
        }
        ".jsonc" => {
            if !cfg.lint.json.is_enabled() {
                return Vec::new();
            }
            cc_lint::validate_json5(path, content)
        }
        ".json" => {
            if !cfg.lint.json.is_enabled() {
                return Vec::new();
            }
            // Apply default exclusions only when the user has no custom ignore settings.
            let user_ignore = &cfg.lint.json.ignore_files;
            let ignored = if user_ignore.is_empty() {
                let defaults: Vec<String> = cc_lint::DEFAULT_JSON_IGNORE_FILES
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                cc_pathutil::matches_any(path, &defaults)
            } else {
                cc_pathutil::matches_any(path, user_ignore)
            };
            if ignored {
                return Vec::new();
            }
            if cfg.lint.json.is_allow_json5() {
                cc_lint::validate_json5(path, content)
            } else if cfg.lint.json.is_comment_filter() {
                cc_lint::validate_jsonc(path, content)
            } else {
                cc_lint::validate_json(path, content)
            }
        }
        ".xml" => {
            if !cfg.lint.xml.is_enabled() {
                return Vec::new();
            }
            if cc_pathutil::matches_any(path, &cfg.lint.xml.ignore_files) {
                return Vec::new();
            }
            cc_lint::validate_xml(path, content)
        }
        ".toml" => {
            if !cfg.lint.toml.is_enabled() {
                return Vec::new();
            }
            if cc_pathutil::matches_any(path, &cfg.lint.toml.ignore_files) {
                return Vec::new();
            }
            cc_lint::validate_toml(path, content)
        }
        _ => Vec::new(),
    }
}

fn to_messages(errs: Vec<ValidationError>) -> Vec<String> {
    errs.into_iter()
        .map(|ve| {
            cc_i18n::t!(
                "diff.lint_error",
                Path = ve.file,
                Line = ve.line,
                Message = ve.message
            )
        })
        .collect()
}

/// Checks staged data files for syntax errors. Corresponds to Go `CheckLint`.
pub fn check_lint(cfg: &Config, cancel: &AtomicBool) -> Result<Vec<String>, String> {
    if !cfg.lint.is_enabled() {
        return Ok(Vec::new());
    }
    let files = get_staged_files()?;
    let global_ignore = &cfg.exceptions.global_ignore;
    let mut errs = Vec::new();
    for path in &files {
        crate::check_cancelled(cancel)?;
        if cc_pathutil::matches_any(path, global_ignore) {
            continue;
        }
        // Skip reading if the extension is not a target.
        if ext_lower(path).is_empty() {
            continue;
        }
        let content = match cc_gitdiff::get_staged_content(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        errs.extend(to_messages(lint_validate(cfg, path, &content)));
    }
    Ok(errs)
}

/// Checks all tracked data files for syntax errors. Corresponds to Go `RunLint`.
pub fn run_lint(
    cfg: &Config,
    files: &[String],
    cancel: &AtomicBool,
) -> Result<Vec<String>, String> {
    if !cfg.lint.is_enabled() {
        return Ok(Vec::new());
    }
    let global_ignore = cfg.exceptions.global_ignore.clone();
    crate::for_each_file_concurrent(files, cancel, |path| {
        if cc_pathutil::matches_any(path, &global_ignore) {
            return Ok(Vec::new());
        }
        if ext_lower(path).is_empty() {
            return Ok(Vec::new());
        }
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Ok(Vec::new()),
        };
        Ok(to_messages(lint_validate(cfg, path, &content)))
    })
}
