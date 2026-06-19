//! Invisible/ambiguous Unicode character checks (staged and tracked files). Corresponds to Go `internal/checker/unicode.go`.

use std::sync::atomic::AtomicBool;

use git_warden_charset::AmbiguousTable;
use git_warden_config::Config;

use crate::staged::{get_staged_files, staged_content_bytes};

/// Checks staged files for invisible/ambiguous Unicode characters. Corresponds to Go `CheckUnicode`.
pub fn check_unicode(cfg: &Config, cancel: &AtomicBool) -> Result<Vec<String>, String> {
    if !cfg.encoding.is_enabled() {
        return Ok(Vec::new());
    }
    if !cfg.encoding.is_no_invisible_chars() && !cfg.encoding.is_no_ambiguous_chars() {
        return Ok(Vec::new());
    }
    let files = get_staged_files()?;
    check_unicode_files(cfg, &files, cancel, staged_content_bytes)
}

/// Checks all tracked files for invisible/ambiguous Unicode characters. Corresponds to Go `RunUnicode`.
pub fn run_unicode(
    cfg: &Config,
    files: &[String],
    cancel: &AtomicBool,
) -> Result<Vec<String>, String> {
    if !cfg.encoding.is_enabled() {
        return Ok(Vec::new());
    }
    if !cfg.encoding.is_no_invisible_chars() && !cfg.encoding.is_no_ambiguous_chars() {
        return Ok(Vec::new());
    }
    check_unicode_files(cfg, files, cancel, |path| {
        std::fs::read(path).map_err(|e| e.to_string())
    })
}

fn check_unicode_files<R>(
    cfg: &Config,
    files: &[String],
    cancel: &AtomicBool,
    read_content: R,
) -> Result<Vec<String>, String>
where
    R: Fn(&str) -> Result<Vec<u8>, String>,
{
    let mut ignore_patterns = cfg.exceptions.global_ignore.clone();
    ignore_patterns.extend(cfg.encoding.ignore_files.iter().cloned());
    let check_invisible = cfg.encoding.is_no_invisible_chars();
    let check_ambiguous = cfg.encoding.is_no_ambiguous_chars();

    let tables: Vec<&AmbiguousTable> = if check_ambiguous {
        git_warden_charset::tables_for_locale(&cfg.encoding.locale)
    } else {
        Vec::new()
    };

    let mut errs = Vec::new();
    for path in files {
        crate::check_cancelled(cancel)?;
        if git_warden_pathutil::matches_any(path, &ignore_patterns) {
            continue;
        }

        let raw = match read_content(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if git_warden_encoding::is_binary(&raw) {
            continue;
        }

        // Character-level checks use lossy-decoded text (invalid bytes → U+FFFD).
        let content = String::from_utf8_lossy(&raw);
        for (line_no, line) in content.split('\n').enumerate() {
            for r in line.chars() {
                if check_invisible && git_warden_charset::is_invisible(r) {
                    let mut name = git_warden_charset::invisible_name(r).to_string();
                    if name.is_empty() {
                        name = format!("U+{:04X}", r as u32);
                    }
                    errs.push(git_warden_i18n::t!(
                        "diff.file_invisible_char",
                        Path = path,
                        Line = line_no + 1,
                        Char = format!("U+{:04X}", r as u32),
                        Name = name
                    ));
                }
                if check_ambiguous {
                    if let Some(confused_with) = git_warden_charset::is_ambiguous(r, &tables) {
                        errs.push(git_warden_i18n::t!(
                            "diff.file_ambiguous_char",
                            Path = path,
                            Line = line_no + 1,
                            Char = format!("U+{:04X}", r as u32),
                            LooksAs = confused_with
                        ));
                    }
                }
            }
        }
    }
    Ok(errs)
}
