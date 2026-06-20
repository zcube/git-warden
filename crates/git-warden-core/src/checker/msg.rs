//! Commit message policy checks. Corresponds to Go `internal/checker/msg.go`.

use crate::config::{CommitMessageConfig, CommitMessageLanguageConfig, Config};

use super::conventional::check_conventional;
use super::custom::check_msg_custom_rules;
use super::diff_check::truncate;

/// Checks a commit message for all configured policy violations. Corresponds to Go `CheckMsg`.
/// `content` is the raw byte slice (for detecting invalid UTF-8); character-level checks use lossy-decoded text.
pub fn check_msg(cfg: &Config, content: &[u8]) -> Vec<String> {
    if !cfg.commit_message.is_enabled() {
        return Vec::new();
    }

    // Character-level checks use lossy text matching Go's range-over-string behavior (invalid bytes → U+FFFD).
    let text = String::from_utf8_lossy(content);
    let cm = &cfg.commit_message;
    let mut errs = Vec::new();

    if cm.is_no_ai_coauthor() {
        errs.extend(check_coauthor(&text, cm));
    }
    if cm.is_no_unicode_spaces() {
        errs.extend(check_invisible_chars(&text));
    }
    if cm.is_no_ambiguous_chars() {
        let tables = crate::charset::tables_for_locale(&cm.locale);
        errs.extend(check_ambiguous_chars(&text, &tables));
    }
    if cm.is_no_bad_runes() {
        errs.extend(check_bad_runes(content));
    }
    if cm.is_no_emoji() {
        errs.extend(check_msg_emoji(&text));
    }
    if cm.language_check.is_enabled() {
        errs.extend(check_msg_language(&text, &cm.language_check));
    }
    if cm.conventional_commit.is_enabled() {
        errs.extend(check_conventional(&text, &cm.conventional_commit));
    }
    if cm.subject_limit.is_enabled() {
        errs.extend(check_subject_limit(&text, cm));
    }
    if cm.body_line_limit.is_enabled() {
        errs.extend(check_body_line_limit(&text, cm));
    }
    if !cfg.custom_rules.commit_message.is_empty() {
        errs.extend(check_msg_custom_rules(
            &text,
            &cfg.custom_rules.commit_message,
        ));
    }

    errs
}

/// Reports Co-authored-by: trailer lines whose email matches AI tool patterns. Corresponds to Go `checkCoauthor`.
fn check_coauthor(content: &str, cfg: &CommitMessageConfig) -> Vec<String> {
    let mut errs = Vec::new();
    for (i, line) in content.split('\n').enumerate() {
        let trimmed = line.trim();
        if !trimmed.to_lowercase().starts_with("co-authored-by:") {
            continue;
        }
        let email = crate::config::extract_coauthor_email(trimmed);
        if cfg.coauthor_should_remove(&email) {
            errs.push(crate::t!(
                "msg.ai_coauthor_error",
                Line = i + 1,
                Trailer = trimmed
            ));
        }
    }
    errs
}

/// Detects invisible/non-standard whitespace characters (excluding BOM). Corresponds to Go `checkInvisibleChars`.
fn check_invisible_chars(content: &str) -> Vec<String> {
    let mut errs = Vec::new();
    for (line_idx, line) in content.split('\n').enumerate() {
        let mut col = 0;
        for r in line.chars() {
            col += 1;
            if crate::charset::is_invisible(r) {
                let name = crate::charset::invisible_name(r);
                let mut desc = format!("U+{:04X}", r as u32);
                if !name.is_empty() {
                    desc.push(' ');
                    desc.push_str(name);
                }
                errs.push(crate::t!(
                    "msg.invisible_char_error",
                    Line = line_idx + 1,
                    Col = col,
                    Desc = desc
                ));
            }
        }
    }
    errs
}

/// Detects Unicode characters that are visually confusable with ASCII. Corresponds to Go `checkAmbiguousChars`.
fn check_ambiguous_chars(content: &str, tables: &[&crate::charset::AmbiguousTable]) -> Vec<String> {
    let mut errs = Vec::new();
    for (line_idx, line) in content.split('\n').enumerate() {
        let mut col = 0;
        for r in line.chars() {
            col += 1;
            if let Some(confusable_to) = crate::charset::is_ambiguous(r, tables) {
                errs.push(crate::t!(
                    "msg.ambiguous_char_error",
                    Line = line_idx + 1,
                    Col = col,
                    CharCode = format!("{:04X}", r as u32),
                    Char = r,
                    ASCII = confusable_to,
                    ASCIICode = format!("{:04X}", confusable_to as u32)
                ));
            }
        }
    }
    errs
}

/// Detects invalid UTF-8 byte sequences. Corresponds to Go `checkBadRunes` (reproduces utf8.DecodeRune semantics).
fn check_bad_runes(content: &[u8]) -> Vec<String> {
    let mut errs = Vec::new();
    let mut line_idx = 1;
    let mut col = 1;
    let mut i = 0;
    while i < content.len() {
        let (ch, size) = decode_rune(&content[i..]);
        match ch {
            None => {
                errs.push(crate::t!(
                    "msg.bad_rune_error",
                    Line = line_idx,
                    Col = col,
                    Byte = format!("{:02X}", content[i])
                ));
                col += 1;
            }
            Some('\n') => {
                line_idx += 1;
                col = 1;
            }
            Some(_) => {
                col += 1;
            }
        }
        i += size;
    }
    errs
}

/// Decodes the first rune in content[..]. Returns (Some(char), size) on success, (None, 1) on invalid byte.
/// Rejects overlong encodings, surrogates, and out-of-range codepoints identically to Go's utf8.DecodeRune.
pub(crate) fn decode_rune(b: &[u8]) -> (Option<char>, usize) {
    if b.is_empty() {
        return (None, 0);
    }
    let b0 = b[0];
    if b0 < 0x80 {
        return (Some(b0 as char), 1);
    }
    let (size, min, mut cp) = if b0 & 0xE0 == 0xC0 {
        (2usize, 0x80u32, (b0 & 0x1F) as u32)
    } else if b0 & 0xF0 == 0xE0 {
        (3, 0x800, (b0 & 0x0F) as u32)
    } else if b0 & 0xF8 == 0xF0 {
        (4, 0x10000, (b0 & 0x07) as u32)
    } else {
        return (None, 1);
    };
    if b.len() < size {
        return (None, 1);
    }
    for &cb in &b[1..size] {
        if cb & 0xC0 != 0x80 {
            return (None, 1);
        }
        cp = (cp << 6) | (cb & 0x3F) as u32;
    }
    // Reject overlong encodings, surrogates, and out-of-range codepoints.
    if cp < min || (0xD800..=0xDFFF).contains(&cp) || cp > 0x10FFFF {
        return (None, 1);
    }
    match char::from_u32(cp) {
        Some(c) => (Some(c), size),
        None => (None, 1),
    }
}

/// Checks whether the commit message body is written in the required language. Corresponds to Go `checkMsgLanguage`.
fn check_msg_language(content: &str, cfg: &CommitMessageLanguageConfig) -> Vec<String> {
    let trimmed_content = content.trim_end_matches('\n');
    let lines: Vec<&str> = trimmed_content.split('\n').collect();
    if lines.is_empty() {
        return Vec::new();
    }

    let subject = lines[0].trim();
    for prefix in &cfg.skip_prefixes {
        if subject.starts_with(prefix.as_str()) {
            return Vec::new();
        }
    }

    let required = cfg.get_locale();
    let min_length = cfg.min_length.max(0) as usize;

    let mut errs = Vec::new();
    let mut check_line = |line_num: usize, text: &str| {
        let text = text.trim();
        let (ok, has_content) =
            crate::langdetect::is_required_language(text, &required, min_length, &[]);
        if has_content && !ok {
            errs.push(crate::t!(
                "msg.language_error",
                Line = line_num,
                Language = required,
                Detected = crate::langdetect::dominant_language(text),
                Text = truncate(text, 80)
            ));
        }
    };

    // Strip the Conventional Commit prefix (e.g. "ci: ", "feat(scope)!: ") from the subject
    // before language detection to avoid false positives from the type token.
    let subject_text = strip_cc_prefix(subject).unwrap_or(subject);
    check_line(1, subject_text);
    for (i, line) in lines.iter().enumerate().skip(1) {
        check_line(i + 1, line);
    }
    errs
}

/// Strips a Conventional Commit prefix (`type(scope)!: `) from a subject line.
/// Returns the description part, or `None` if no prefix was found.
fn strip_cc_prefix(s: &str) -> Option<&str> {
    // type: one or more alphanumeric / hyphen / underscore chars
    let type_end = s.find(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')?;
    if type_end == 0 {
        return None;
    }
    let rest = &s[type_end..];
    // optional scope: (...)
    let rest = if rest.starts_with('(') {
        let close = rest.find(')')?;
        &rest[close + 1..]
    } else {
        rest
    };
    // optional breaking-change marker
    let rest = rest.strip_prefix('!').unwrap_or(rest);
    // required ": "
    let rest = rest.strip_prefix(':')?;
    Some(rest.trim_start_matches(' '))
}

/// Checks the commit message subject line character count limit. Corresponds to Go `checkSubjectLimit`.
fn check_subject_limit(content: &str, cfg: &CommitMessageConfig) -> Vec<String> {
    let subject = content
        .trim_end_matches('\n')
        .split('\n')
        .next()
        .unwrap_or("")
        .trim();
    let max_len = cfg.subject_limit.get_max_length();
    let rune_len = subject.chars().count() as i64;
    if rune_len > max_len {
        return vec![crate::t!(
            "msg.subject_too_long",
            Length = rune_len,
            Max = max_len
        )];
    }
    Vec::new()
}

/// Checks the character count limit for each body line of the commit message. Corresponds to Go `checkBodyLineLimit`.
fn check_body_line_limit(content: &str, cfg: &CommitMessageConfig) -> Vec<String> {
    let lines: Vec<&str> = content.trim_end_matches('\n').split('\n').collect();
    let max_len = cfg.body_line_limit.get_max_length();
    let mut errs = Vec::new();
    // Skip the subject (first line) + the blank separator (second line).
    let mut start = 1;
    if lines.len() > 1 && lines[1].trim().is_empty() {
        start = 2;
    }
    for (i, line) in lines.iter().enumerate().skip(start) {
        let rune_len = line.chars().count() as i64;
        if rune_len > max_len {
            errs.push(crate::t!(
                "msg.body_line_too_long",
                Line = i + 1,
                Length = rune_len,
                Max = max_len
            ));
        }
    }
    errs
}

/// Detects emojis in the commit message. Corresponds to Go `checkMsgEmoji`.
fn check_msg_emoji(content: &str) -> Vec<String> {
    crate::emoji::find_emojis(content)
        .into_iter()
        .map(|e| {
            crate::t!(
                "msg.emoji_error",
                Line = e.line,
                Col = e.col,
                Char = e.char,
                CharCode = format!("{:04X}", e.code)
            )
        })
        .collect()
}
