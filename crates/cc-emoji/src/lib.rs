//! cc-emoji: detects emoji characters in commit messages and source code.
//! 1:1 port of Go `internal/emoji`.

/// Checks whether the given character is an emoji character.
/// Covers common emoji ranges: emoticons, symbols, dingbats, transport, flags, supplemental pictographs.
/// Same ranges and special cases as Go `IsEmoji`.
// Skin tone modifiers (0x1F3FB..=0x1F3FF) are a subset of 0x1F300..=0x1F5FF and thus an
// unreachable arm, but the original Go switch has the same overlap and both return true — behaviour is identical.
#[allow(unreachable_patterns)]
pub fn is_emoji(c: char) -> bool {
    let r = c as u32;
    match r {
        // Variation Selector-16 (forces emoji presentation)
        0xFE0F => true,
        // Zero Width Joiner (joins emoji sequences)
        0x200D => true,
        // Combining Enclosing Keycap
        0x20E3 => true,
        // Miscellaneous Symbols
        0x2600..=0x26FF => true,
        // Dingbats
        0x2700..=0x27BF => true,
        // CJK symbols and some emoji
        0x2B50 | 0x2B55 | 0x2B1B | 0x2B1C => true,
        // Treated as plain text (not emoji): © ®
        0x00A9 | 0x00AE => false,
        // ‼ ⁉
        0x203C | 0x2049 => true,
        // ™ ↔ etc.
        0x2122..=0x2199 => true,
        // ↩ ↪
        0x21A9..=0x21AA => true,
        // ⌚ ⌛
        0x231A..=0x231B => true,
        // ⌨
        0x2328 => true,
        // ⏏
        0x23CF => true,
        // ⏩-⏳
        0x23E9..=0x23F3 => true,
        // ⏸-⏺
        0x23F8..=0x23FA => true,
        // ▪ ▫
        0x25AA | 0x25AB => true,
        // ▶ ◀
        0x25B6 | 0x25C0 => true,
        // ◻-◾
        0x25FB..=0x25FE => true,
        // Emoticons
        0x1F600..=0x1F64F => true,
        // Miscellaneous Symbols and Pictographs
        0x1F300..=0x1F5FF => true,
        // Transport and Map Symbols
        0x1F680..=0x1F6FF => true,
        // Regional Indicator Symbols (flags)
        0x1F1E0..=0x1F1FF => true,
        // Supplemental Symbols and Pictographs
        0x1F900..=0x1F9FF => true,
        // Symbols and Pictographs Extended-A
        0x1FA70..=0x1FAFF => true,
        // Chess Symbols
        0x1FA00..=0x1FA6F => true,
        // Skin Tone Modifiers
        0x1F3FB..=0x1F3FF => true,
        // Tags (used in flag sequences)
        0xE0020..=0xE007F => true,
        _ => false,
    }
}

/// Detected emoji character info. Corresponds to Line/Col/Char/Code in Go `EmojiInfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmojiInfo {
    /// Line number (1-based).
    pub line: usize,
    /// Character (rune) column within the line (1-based).
    pub col: usize,
    /// The emoji character.
    pub char: char,
    /// Code point.
    pub code: u32,
}

/// Scans text and returns all emoji occurrences with their positions. Corresponds to Go `FindEmojis`.
pub fn find_emojis(text: &str) -> Vec<EmojiInfo> {
    let mut results = Vec::new();
    let mut line = 1usize;
    let mut col = 0usize;
    for r in text.chars() {
        col += 1;
        if r == '\n' {
            line += 1;
            col = 0;
            continue;
        }
        // Skip variation selectors (FE0F) and ZWJ; they are part of sequences and should not be reported individually.
        if r as u32 == 0xFE0F || r as u32 == 0x200D {
            continue;
        }
        if is_emoji(r) && !r.is_whitespace() {
            results.push(EmojiInfo {
                line,
                col,
                char: r,
                code: r as u32,
            });
        }
    }
    results
}

/// Returns whether the text contains any emoji characters. Corresponds to Go `ContainsEmoji`.
pub fn contains_emoji(text: &str) -> bool {
    for r in text.chars() {
        if r as u32 == 0xFE0F || r as u32 == 0x200D {
            continue;
        }
        if is_emoji(r) && !r.is_whitespace() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_emoji() {
        let cases: &[(char, bool)] = &[
            ('A', false),
            ('가', false),
            ('あ', false),
            (' ', false),
            ('©', false), // plain text character, not an emoji
            ('😀', true),
            ('🎉', true),
            ('🚀', true),
            ('❌', true),
            ('✅', true),
            ('⭐', true),
            ('🇰', true), // regional indicator symbol
            ('🤔', true),
            ('👍', true),
            ('🔥', true),
            ('💡', true),
        ];
        for (r, want) in cases {
            assert_eq!(is_emoji(*r), *want, "is_emoji({r:?} U+{:04X})", *r as u32);
        }
    }

    #[test]
    fn test_contains_emoji() {
        let cases: &[(&str, bool)] = &[
            ("hello world", false),
            ("변수 설정", false),
            ("fix: bug 수정", false),
            ("fix: 🐛 bug fix", true),
            ("feat: ✨ new feature", true),
            ("🚀 deploy", true),
            ("no emoji here! @#$%", false),
        ];
        for (text, want) in cases {
            assert_eq!(contains_emoji(text), *want, "contains_emoji({text:?})");
        }
    }

    #[test]
    fn test_find_emojis() {
        let text = "line1 😀 text\nline2 🎉 more";
        let emojis = find_emojis(text);
        assert_eq!(emojis.len(), 2, "expected 2 emojis");
        assert_eq!(emojis[0].line, 1);
        assert_eq!(emojis[0].char, '😀');
        assert_eq!(emojis[1].line, 2);
        assert_eq!(emojis[1].char, '🎉');
    }
}
