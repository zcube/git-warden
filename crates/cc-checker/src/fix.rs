//! Auto-fix for commit messages and file content. Corresponds to Go `internal/checker/fix.go`.

use cc_config::{CommitMessageConfig, Config};

use crate::msg::decode_rune;

/// Fix result. Corresponds to Go `FixResult`.
#[derive(Debug, Clone)]
pub struct FixResult {
    pub original: String,
    pub fixed: String,
    pub changes: Vec<String>,
}

impl FixResult {
    /// Returns true if there were auto-fixable violations. Corresponds to Go `NeedsFixing`.
    pub fn needs_fixing(&self) -> bool {
        !self.changes.is_empty()
    }
}

/// Fixes Unicode issues in source file content. Corresponds to Go `FixFileContent`.
pub fn fix_file_content(cfg: &Config, content: &[u8]) -> FixResult {
    let mut changes = Vec::new();
    // Handle bad runes first (subsequent fixers are string-based).
    let mut fixed = fix_bad_runes(content, &mut changes);
    if cfg.encoding.is_no_invisible_chars() {
        fixed = fix_invisible_chars(&fixed, &mut changes);
    }
    if cfg.encoding.is_no_ambiguous_chars() {
        let tables = cc_charset::tables_for_locale(&cfg.commit_message.locale);
        fixed = fix_ambiguous_chars(&fixed, &mut changes, &tables);
    }
    FixResult {
        original: String::from_utf8_lossy(content).to_string(),
        fixed,
        changes,
    }
}

/// Applies auto-fixes to a commit message. Corresponds to Go `FixMsg`.
pub fn fix_msg(cfg: &Config, content: &[u8]) -> FixResult {
    let cm = &cfg.commit_message;
    let mut changes = Vec::new();
    let original = String::from_utf8_lossy(content).to_string();

    // Handle bad runes first.
    let mut fixed = if cm.is_no_bad_runes() {
        fix_bad_runes(content, &mut changes)
    } else {
        String::from_utf8_lossy(content).to_string()
    };
    if cm.is_no_ai_coauthor() {
        fixed = fix_coauthor(&fixed, &mut changes, cm);
    }
    if cm.is_no_unicode_spaces() {
        fixed = fix_invisible_chars(&fixed, &mut changes);
    }
    if cm.is_no_ambiguous_chars() {
        let tables = cc_charset::tables_for_locale(&cm.locale);
        fixed = fix_ambiguous_chars(&fixed, &mut changes, &tables);
    }

    FixResult {
        original,
        fixed,
        changes,
    }
}

/// Removes Co-authored-by: trailer lines matching AI patterns. Corresponds to Go `fixCoauthor`.
fn fix_coauthor(content: &str, changes: &mut Vec<String>, cfg: &CommitMessageConfig) -> String {
    let mut kept = Vec::new();
    for (i, line) in content.split('\n').enumerate() {
        let trimmed = line.trim();
        if trimmed.to_lowercase().starts_with("co-authored-by:") {
            let email = cc_config::extract_coauthor_email(trimmed);
            if cfg.coauthor_should_remove(&email) {
                changes.push(cc_i18n::t!(
                    "fix.removed_ai_coauthor",
                    Line = i + 1,
                    Trailer = trimmed
                ));
                continue;
            }
        }
        kept.push(line);
    }
    let mut result = kept.join("\n");
    // Collapse multiple trailing newlines down to at most one.
    while result.ends_with("\n\n") {
        result.truncate(result.len() - 1);
    }
    result
}

/// Replaces invisible/non-standard spaces with regular spaces; removes control/zero-width chars. Corresponds to Go `fixInvisibleChars`.
fn fix_invisible_chars(content: &str, changes: &mut Vec<String>) -> String {
    let mut sb = String::with_capacity(content.len());
    for (line_idx, line) in content.split('\n').enumerate() {
        if line_idx > 0 {
            sb.push('\n');
        }
        let mut col = 0;
        for r in line.chars() {
            col += 1;
            if cc_charset::is_invisible(r) {
                let name = cc_charset::invisible_name(r);
                let mut desc = format!("U+{:04X}", r as u32);
                if !name.is_empty() {
                    desc.push(' ');
                    desc.push_str(name);
                }
                if is_space_variant(r) {
                    changes.push(cc_i18n::t!(
                        "fix.replaced_invisible_space",
                        Line = line_idx + 1,
                        Col = col,
                        Desc = desc
                    ));
                    sb.push(' ');
                } else {
                    changes.push(cc_i18n::t!(
                        "fix.removed_invisible_char",
                        Line = line_idx + 1,
                        Col = col,
                        Desc = desc
                    ));
                }
                continue;
            }
            sb.push(r);
        }
    }
    sb
}

/// Returns true if the invisible character is semantically a space (replaced with U+0020). Corresponds to Go `isSpaceVariant`.
fn is_space_variant(r: char) -> bool {
    matches!(
        r as u32,
        0x00A0
            | 0x1680
            | 0x2000
            | 0x2001
            | 0x2002
            | 0x2003
            | 0x2004
            | 0x2005
            | 0x2006
            | 0x2007
            | 0x2008
            | 0x2009
            | 0x200A
            | 0x202F
            | 0x205F
            | 0x3000
    )
}

/// Replaces ambiguous Unicode characters with their ASCII lookalikes. Corresponds to Go `fixAmbiguousChars`.
fn fix_ambiguous_chars(
    content: &str,
    changes: &mut Vec<String>,
    tables: &[&cc_charset::AmbiguousTable],
) -> String {
    let mut sb = String::with_capacity(content.len());
    for (line_idx, line) in content.split('\n').enumerate() {
        if line_idx > 0 {
            sb.push('\n');
        }
        let mut col = 0;
        for r in line.chars() {
            col += 1;
            if let Some(confusable_to) = cc_charset::is_ambiguous(r, tables) {
                changes.push(cc_i18n::t!(
                    "fix.replaced_ambiguous_char",
                    Line = line_idx + 1,
                    Col = col,
                    CharCode = format!("{:04X}", r as u32),
                    Char = r,
                    ASCII = confusable_to,
                    ASCIICode = format!("{:04X}", confusable_to as u32)
                ));
                sb.push(confusable_to);
                continue;
            }
            sb.push(r);
        }
    }
    sb
}

/// Removes invalid UTF-8 byte sequences. Corresponds to Go `fixBadRunes`.
fn fix_bad_runes(content: &[u8], changes: &mut Vec<String>) -> String {
    let mut sb = String::new();
    let mut line_idx = 1;
    let mut col = 1;
    let mut i = 0;
    while i < content.len() {
        let (ch, size) = decode_rune(&content[i..]);
        match ch {
            None => {
                changes.push(cc_i18n::t!(
                    "fix.removed_bad_rune",
                    Line = line_idx,
                    Col = col,
                    Byte = format!("{:02X}", content[i])
                ));
                i += 1;
                continue;
            }
            Some('\n') => {
                line_idx += 1;
                col = 1;
                sb.push('\n');
            }
            Some(c) => {
                col += 1;
                sb.push(c);
            }
        }
        i += size;
    }
    sb
}
