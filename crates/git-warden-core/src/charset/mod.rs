//! cc-charset: Unicode character classification utilities. Corresponds to Go `internal/charset`.
//!
//! - [`is_invisible`] / [`invisible_name`]: invisible/zero-width character detection.
//! - [`tables_for_locale`] / [`is_ambiguous`]: locale-aware ambiguous (confusable) character detection.

mod ambiguous;
mod ambiguous_gen;
mod invisible;

pub use ambiguous::{is_ambiguous, lookup, tables_for_locale, AmbiguousTable};
pub use invisible::{invisible_name, is_invisible};

#[cfg(test)]
mod tests {
    use super::*;

    // ---- is_invisible ----

    #[test]
    fn invisible_allowed_whitespace() {
        for r in [' ', '\t', '\n', '\r'] {
            assert!(!is_invisible(r), "U+{:04X} should be allowed", r as u32);
        }
    }

    #[test]
    fn invisible_bom_allowed() {
        assert!(!is_invisible('\u{FEFF}'), "U+FEFF (BOM) should be allowed");
    }

    #[test]
    fn invisible_chars_detected() {
        let invisible = [
            0x00A0u32, 0x200B, 0x200C, 0x200D, 0x200E, 0x200F, 0x202A, 0x202E, 0x2060, 0x3000,
            0xFFA0,
        ];
        for c in invisible {
            assert!(
                is_invisible(char::from_u32(c).unwrap()),
                "U+{c:04X} should be invisible"
            );
        }
        // U+2028/U+2029 are outside the ranges and are therefore not invisible.
        assert!(!is_invisible('\u{2028}'));
        assert!(!is_invisible('\u{2029}'));
    }

    #[test]
    fn normal_chars_not_invisible() {
        for r in ['A', 'z', '0', '한', '日', '!', '.'] {
            assert!(
                !is_invisible(r),
                "U+{:04X} should not be invisible",
                r as u32
            );
        }
    }

    // ---- invisible_name ----

    #[test]
    fn invisible_name_known() {
        let cases = [
            (0x00A0u32, "NO-BREAK SPACE"),
            (0x00AD, "SOFT HYPHEN"),
            (0x200B, "ZERO WIDTH SPACE"),
            (0x200C, "ZERO WIDTH NON-JOINER"),
            (0x200D, "ZERO WIDTH JOINER"),
            (0x200E, "LEFT-TO-RIGHT MARK"),
            (0x200F, "RIGHT-TO-LEFT MARK"),
            (0x202A, "LEFT-TO-RIGHT EMBEDDING"),
            (0x202E, "RIGHT-TO-LEFT OVERRIDE"),
            (0x202F, "NARROW NO-BREAK SPACE"),
            (0x2003, "EM SPACE"),
            (0x2060, "WORD JOINER"),
            (0x3000, "IDEOGRAPHIC SPACE"),
            (0xFFA0, "HALFWIDTH HANGUL FILLER"),
        ];
        for (c, want) in cases {
            assert_eq!(invisible_name(char::from_u32(c).unwrap()), want);
        }
    }

    #[test]
    fn invisible_name_all_known_non_empty() {
        let known = [
            0x00A0u32, 0x00AD, 0x034F, 0x1680, 0x180E, 0x2000, 0x2001, 0x2002, 0x2003, 0x2004,
            0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200A, 0x200B, 0x200C, 0x200D, 0x200E, 0x200F,
            0x202A, 0x202B, 0x202C, 0x202D, 0x202E, 0x202F, 0x205F, 0x2060, 0x2061, 0x2062, 0x2063,
            0x2064, 0x206A, 0x206B, 0x206C, 0x206D, 0x206E, 0x206F, 0x2800, 0x3000, 0xFFA0,
        ];
        for c in known {
            assert!(
                !invisible_name(char::from_u32(c).unwrap()).is_empty(),
                "U+{c:04X} should have a name"
            );
        }
    }

    // ---- tables_for_locale ----

    #[test]
    fn tables_for_known_locales() {
        for locale in ["ko", "ja", "ru", "zh-hans", "zh-hant", "_default"] {
            let tables = tables_for_locale(locale);
            assert_eq!(tables.len(), 2, "{locale} should have 2 tables");
        }
    }

    #[test]
    fn tables_unknown_locale_falls_back_to_default() {
        let tables = tables_for_locale("xx-unknown");
        assert_eq!(tables.len(), 2);
    }

    #[test]
    fn tables_zh_cn_falls_back_to_zh_hans() {
        for locale in ["zh-CN", "zh_CN"] {
            let tables = tables_for_locale(locale);
            assert_eq!(tables.len(), 2);
            assert!(std::ptr::eq(tables[0], lookup("zh-hans").unwrap()));
        }
    }

    #[test]
    fn tables_zh_variants() {
        for locale in ["zh", "zh-TW", "zh-something"] {
            let tables = tables_for_locale(locale);
            assert_eq!(tables.len(), 2);
        }
    }

    #[test]
    fn tables_always_include_common() {
        for locale in ["ko", "en", "unknown"] {
            let tables = tables_for_locale(locale);
            assert_eq!(tables.len(), 2);
            assert_eq!(tables[1].locale, "_common");
        }
    }

    #[test]
    fn subtag_fallback() {
        // ko-KR falls back to ko → same static table reference.
        let ko_kr = tables_for_locale("ko-KR");
        let ko = tables_for_locale("ko");
        assert!(std::ptr::eq(ko_kr[0], ko[0]));
    }

    // ---- is_ambiguous ----

    #[test]
    fn ambiguous_cyrillic_a() {
        let tables = tables_for_locale("ko");
        // U+0410 CYRILLIC CAPITAL LETTER A resembles Latin A.
        assert_eq!(is_ambiguous('\u{0410}', &tables), Some('A'));
    }

    #[test]
    fn ambiguous_cyrillic_o() {
        let tables = tables_for_locale("ko");
        // U+043E CYRILLIC SMALL LETTER O resembles Latin o.
        assert_eq!(is_ambiguous('\u{043E}', &tables), Some('o'));
    }

    #[test]
    fn ambiguous_normal_ascii_not() {
        let tables = tables_for_locale("ko");
        for r in "ABCabc123".chars() {
            assert_eq!(
                is_ambiguous(r, &tables),
                None,
                "{r:?} should not be ambiguous"
            );
        }
    }

    #[test]
    fn ambiguous_korean_hangul_not() {
        let tables = tables_for_locale("ko");
        for r in "안녕하세요".chars() {
            assert_eq!(is_ambiguous(r, &tables), None);
        }
    }

    #[test]
    fn ambiguous_empty_tables_skipped() {
        // An empty table list returns None without panicking.
        assert_eq!(is_ambiguous('\u{0410}', &[]), None);
    }

    #[test]
    fn ambiguous_japanese_locale() {
        let tables = tables_for_locale("ja");
        assert!(is_ambiguous('\u{0410}', &tables).is_some());
    }
}
