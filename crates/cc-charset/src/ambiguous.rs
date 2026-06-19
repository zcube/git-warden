//! Detection of ambiguous (confusable) Unicode characters. Corresponds to Go `internal/charset/ambiguous.go`.
//!
//! Logic adapted from Gitea (MIT License).
//! Source data: https://github.com/hediet/vscode-unicode-data/blob/main/out/ambiguous.json

use crate::ambiguous_gen::AMBIGUOUS_CHARACTERS;

/// Maps runes that can be confused with ASCII characters in a given locale.
pub struct AmbiguousTable {
    /// Sorted slice of confusable rune code points.
    pub confusable: &'static [u32],
    /// Parallel slice: the ASCII character that `confusable[i]` resembles.
    pub with_: &'static [u32],
    pub locale: &'static str,
}

/// Looks up a table by key (`_common`/`_default`/`ja`/`ko`/`ru`/`zh-hans`/`zh-hant`).
pub fn lookup(key: &str) -> Option<&'static AmbiguousTable> {
    AMBIGUOUS_CHARACTERS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, t)| t)
}

/// Returns the ambiguous-character tables for the given locale.
/// `locale` is a BCP-47 tag such as "ko", "ja", "zh-hans", or "en".
/// The returned slice always contains the locale-specific table (or the _default fallback) and the _common table.
/// Corresponds to Go `TablesForLocale`.
pub fn tables_for_locale(locale: &str) -> Vec<&'static AmbiguousTable> {
    let mut table: Option<&'static AmbiguousTable> = None;
    let mut key = locale.to_string();
    while !key.is_empty() {
        if let Some(t) = lookup(&key) {
            table = Some(t);
            break;
        }
        match key.rfind(['-', '_']) {
            Some(idx) => key.truncate(idx),
            None => key.clear(),
        }
    }
    // zh-CN → zh-hans fallback
    if table.is_none() && (locale == "zh-CN" || locale == "zh_CN") {
        table = lookup("zh-hans");
    }
    if table.is_none() && locale.starts_with("zh") {
        table = lookup("zh-hant");
    }
    if table.is_none() {
        table = lookup("_default");
    }
    let mut out = Vec::with_capacity(2);
    if let Some(t) = table {
        out.push(t);
    }
    if let Some(common) = lookup("_common") {
        out.push(common);
    }
    out
}

/// Returns the ASCII character that `r` visually resembles if it is ambiguous in the given tables.
/// Corresponds to Go `IsAmbiguous` (check + set confusableTo).
pub fn is_ambiguous(r: char, tables: &[&AmbiguousTable]) -> Option<char> {
    let rv = r as u32;
    for table in tables {
        match table.confusable.binary_search(&rv) {
            Ok(i) => {
                return char::from_u32(table.with_[i]);
            }
            Err(_) => continue,
        }
    }
    None
}
