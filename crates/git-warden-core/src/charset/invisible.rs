//! Detection of invisible/zero-width Unicode characters. Corresponds to Go `internal/charset/invisible.go`.
//!
//! The InvisibleRanges table is adapted from Gitea (MIT License).

/// A Unicode range in (lo, hi, stride) form. Same semantics as Go `unicode.Range16/Range32`:
/// `r` is included if `lo <= r <= hi` and `(r - lo) % stride == 0`.
struct Range {
    lo: u32,
    hi: u32,
    stride: u32,
}

const fn r(lo: u32, hi: u32, stride: u32) -> Range {
    Range { lo, hi, stride }
}

/// Combined range table of Go InvisibleRanges R16 + R32.
static INVISIBLE_RANGES: &[Range] = &[
    // R16
    r(11, 13, 1),
    r(127, 160, 33),
    r(173, 847, 674),
    r(1564, 4447, 2883),
    r(4448, 6068, 1620),
    r(6069, 6155, 86),
    r(6156, 6158, 1),
    r(7355, 7356, 1),
    r(8192, 8207, 1),
    r(8234, 8239, 1),
    r(8287, 8303, 1),
    r(10240, 12288, 2048),
    r(12644, 65024, 52380),
    r(65025, 65039, 1),
    // Note: 65279 (U+FEFF, BOM) is intentionally excluded.
    r(65440, 65440, 1),
    r(65520, 65528, 1),
    r(65532, 65532, 1),
    // R32
    r(78844, 119155, 40311),
    r(119156, 119162, 1),
    r(917504, 917631, 1),
    r(917760, 917999, 1),
];

fn in_ranges(c: u32) -> bool {
    for rg in INVISIBLE_RANGES {
        if c < rg.lo {
            // Table is in ascending order, no need to look further — break rather than continue for safety.
            break;
        }
        if c <= rg.hi && (c - rg.lo).is_multiple_of(rg.stride) {
            return true;
        }
    }
    false
}

/// Returns true if `r` is an invisible/zero-width character that must not appear in commit messages.
///
/// Allowed characters: plain space U+0020, tab U+0009, LF U+000A, CR U+000D, and U+FEFF (BOM).
pub fn is_invisible(r: char) -> bool {
    if r == ' ' || r == '\t' || r == '\n' || r == '\r' {
        return false;
    }
    in_ranges(r as u32)
}

/// Returns a human-readable description for an invisible rune, or an empty string if unknown.
pub fn invisible_name(r: char) -> &'static str {
    match r as u32 {
        0x00A0 => "NO-BREAK SPACE",
        0x00AD => "SOFT HYPHEN",
        0x034F => "COMBINING GRAPHEME JOINER",
        0x1680 => "OGHAM SPACE MARK",
        0x180E => "MONGOLIAN VOWEL SEPARATOR",
        0x2000 => "EN QUAD",
        0x2001 => "EM QUAD",
        0x2002 => "EN SPACE",
        0x2003 => "EM SPACE",
        0x2004 => "THREE-PER-EM SPACE",
        0x2005 => "FOUR-PER-EM SPACE",
        0x2006 => "SIX-PER-EM SPACE",
        0x2007 => "FIGURE SPACE",
        0x2008 => "PUNCTUATION SPACE",
        0x2009 => "THIN SPACE",
        0x200A => "HAIR SPACE",
        0x200B => "ZERO WIDTH SPACE",
        0x200C => "ZERO WIDTH NON-JOINER",
        0x200D => "ZERO WIDTH JOINER",
        0x200E => "LEFT-TO-RIGHT MARK",
        0x200F => "RIGHT-TO-LEFT MARK",
        0x202A => "LEFT-TO-RIGHT EMBEDDING",
        0x202B => "RIGHT-TO-LEFT EMBEDDING",
        0x202C => "POP DIRECTIONAL FORMATTING",
        0x202D => "LEFT-TO-RIGHT OVERRIDE",
        0x202E => "RIGHT-TO-LEFT OVERRIDE",
        0x202F => "NARROW NO-BREAK SPACE",
        0x205F => "MEDIUM MATHEMATICAL SPACE",
        0x2060 => "WORD JOINER",
        0x2061 => "FUNCTION APPLICATION",
        0x2062 => "INVISIBLE TIMES",
        0x2063 => "INVISIBLE SEPARATOR",
        0x2064 => "INVISIBLE PLUS",
        0x206A => "INHIBIT SYMMETRIC SWAPPING",
        0x206B => "ACTIVATE SYMMETRIC SWAPPING",
        0x206C => "INHIBIT ARABIC FORM SHAPING",
        0x206D => "ACTIVATE ARABIC FORM SHAPING",
        0x206E => "NATIONAL DIGIT SHAPES",
        0x206F => "NOMINAL DIGIT SHAPES",
        0x2800 => "BRAILLE PATTERN BLANK",
        0x3000 => "IDEOGRAPHIC SPACE",
        0xFFA0 => "HALFWIDTH HANGUL FILLER",
        _ => "",
    }
}
