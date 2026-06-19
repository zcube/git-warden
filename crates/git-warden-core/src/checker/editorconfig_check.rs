//! .editorconfig rule checks (staged files). Corresponds to Go `internal/checker/editorconfig_check.go`.

use std::path::Path;
use std::sync::atomic::AtomicBool;

use crate::config::Config;

use super::staged::{get_staged_files, staged_content_bytes};

/// Validates staged files against .editorconfig rules. Corresponds to Go `CheckEditorConfig`.
pub fn check_editorconfig(cfg: &Config, cancel: &AtomicBool) -> Result<Vec<String>, String> {
    if !cfg.editorconfig.is_enabled() {
        return Ok(Vec::new());
    }
    // Skip if .editorconfig does not exist.
    if !Path::new(".editorconfig").exists() {
        return Ok(Vec::new());
    }

    let files = get_staged_files()?;
    let mut errs = Vec::new();
    for path in &files {
        super::check_cancelled(cancel)?;
        if crate::pathutil::matches_any(path, &cfg.exceptions.global_ignore) {
            continue;
        }
        if crate::pathutil::matches_any(path, &cfg.editorconfig.ignore_files) {
            continue;
        }

        let def = match crate::editorconfig::get_definition(path) {
            Some(d) => d,
            None => continue,
        };
        let content = match staged_content_bytes(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for v in crate::editorconfig::check(path, &content, &def) {
            errs.push(crate::t!(
                "diff.editorconfig_error",
                Path = v.file,
                Line = v.line,
                Message = v.message
            ));
        }
    }
    Ok(errs)
}
