//! UTF-8 encoding checks for staged files. Corresponds to Go `internal/checker/encoding.go`.

use std::sync::atomic::AtomicBool;

use crate::config::Config;

use super::staged::{get_staged_files, staged_content_bytes};

/// Validates UTF-8 encoding of staged text files. Corresponds to Go `CheckEncoding`.
pub fn check_encoding(cfg: &Config, cancel: &AtomicBool) -> Result<Vec<String>, String> {
    if !cfg.encoding.is_enabled() || !cfg.encoding.is_require_utf8() {
        return Ok(Vec::new());
    }

    let files = get_staged_files()?;
    let mut ignore_patterns = cfg.exceptions.global_ignore.clone();
    ignore_patterns.extend(cfg.encoding.ignore_files.iter().cloned());

    let mut errs = Vec::new();
    for path in &files {
        super::check_cancelled(cancel)?;
        if crate::pathutil::matches_any(path, &ignore_patterns) {
            continue;
        }

        // Skip if editorconfig charset is not utf-8 family.
        if let Some(def) = crate::editorconfig::get_definition(path) {
            if !def.charset.is_empty() && def.charset != "utf-8" && def.charset != "utf-8-bom" {
                continue;
            }
        }

        let raw = match staged_content_bytes(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if crate::encoding::is_binary(&raw) {
            continue;
        }

        let result = crate::encoding::check_utf8(&raw);
        if !result.valid {
            errs.push(crate::t!(
                "diff.encoding_error",
                Path = path,
                Charset = result.detected_charset
            ));
        }
    }
    Ok(errs)
}
