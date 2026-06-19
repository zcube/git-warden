//! cc-encoding: UTF-8 validation and binary/charset detection. Corresponds to Go `internal/encoding`.
//!
//! - UTF-8 validity ports Go's naive byte scanner (structural validation only; no overlong or
//!   surrogate checks). This differs from the stricter `str::from_utf8` in the standard library,
//!   so a direct implementation is used for equivalence.
//! - When UTF-8 is invalid, chardetng provides an estimated charset name (corresponding to Go's saintfish/chardet).
//! - Binary detection uses infer (magic bytes) + NUL heuristic (corresponding to Go's debug/elf·macho·pe + mimetype).

/// File encoding check result. Corresponds to Go `Result`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodingResult {
    pub valid: bool,
    pub has_bom: bool,
    pub detected_charset: String,
    pub confidence: i32,
}

/// Validates whether content is valid UTF-8. Corresponds to Go `CheckUTF8`.
pub fn check_utf8(content: &[u8]) -> EncodingResult {
    let has_bom =
        content.len() >= 3 && content[0] == 0xEF && content[1] == 0xBB && content[2] == 0xBF;

    if content.is_empty() {
        return EncodingResult {
            valid: true,
            has_bom: false,
            detected_charset: "UTF-8".to_string(),
            confidence: 100,
        };
    }

    if is_valid_utf8(content) {
        return EncodingResult {
            valid: true,
            has_bom,
            detected_charset: "UTF-8".to_string(),
            confidence: 100,
        };
    }

    // Only provide an estimated charset name via chardetng when invalid.
    let mut det = chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Allow);
    det.feed(content, true);
    let enc = det.guess(None, chardetng::Utf8Detection::Deny);
    EncodingResult {
        valid: false,
        has_bom,
        detected_charset: enc.name().to_string(),
        confidence: 0,
    }
}

/// Returns true if every byte forms a valid UTF-8 sequence (structural check only). Corresponds to Go `isValidUTF8`.
fn is_valid_utf8(content: &[u8]) -> bool {
    let mut i = 0;
    let n = content.len();
    while i < n {
        if content[i] < 0x80 {
            i += 1;
            continue;
        }
        let size = if content[i] & 0xE0 == 0xC0 {
            2
        } else if content[i] & 0xF0 == 0xE0 {
            3
        } else if content[i] & 0xF8 == 0xF0 {
            4
        } else {
            return false;
        };
        if i + size > n {
            return false;
        }
        for j in 1..size {
            if content[i + j] & 0xC0 != 0x80 {
                return false;
            }
        }
        i += size;
    }
    true
}

/// Returns true if content is binary. Corresponds to Go `IsBinary`.
/// Uses infer to detect executables/images/archives by magic bytes; otherwise falls back to NUL byte heuristic.
pub fn is_binary(content: &[u8]) -> bool {
    if content.is_empty() {
        return false;
    }
    // Detect magic bytes for known binary formats (ELF/Mach-O/PE/images/archives/PDF, etc.).
    if infer::get(content).is_some() {
        return true;
    }
    // Otherwise: treat as binary if a NUL byte is present (not safe to treat as text), otherwise treat as text.
    content.contains(&0x00)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_ascii() {
        let r = check_utf8(b"hello world");
        assert!(r.valid);
        assert!(!r.has_bom);
    }

    #[test]
    fn valid_korean() {
        assert!(check_utf8("한국어 텍스트".as_bytes()).valid);
    }

    #[test]
    fn with_bom() {
        let mut c = vec![0xEF, 0xBB, 0xBF];
        c.extend_from_slice(b"hello");
        let r = check_utf8(&c);
        assert!(r.valid);
        assert!(r.has_bom);
    }

    #[test]
    fn invalid_bytes() {
        let c = [0xFF, 0xFE, 0x68, 0x65, 0x6C, 0x6C, 0x6F];
        assert!(!check_utf8(&c).valid);
    }

    #[test]
    fn latin1_invalid() {
        let c = [0xC4, 0xD6, 0xDC]; // ÄÖÜ (Latin-1)
        assert!(!check_utf8(&c).valid);
    }

    #[test]
    fn iso8859_9_ascii_is_valid() {
        // Pure ASCII must be valid UTF-8 (regardless of chardet misfires).
        let r = check_utf8(b"* text=auto\n*.go text eol=lf\n");
        assert!(r.valid, "detected: {}", r.detected_charset);
    }

    #[test]
    fn empty_is_valid() {
        assert!(check_utf8(b"").valid);
    }

    #[test]
    fn is_binary_text() {
        assert!(!is_binary(b"hello world\nfoo bar\n"));
    }

    #[test]
    fn is_binary_elf() {
        let c = [0x7f, b'E', b'L', b'F', 0, 0, 0, 0];
        assert!(is_binary(&c));
    }

    #[test]
    fn is_binary_empty() {
        assert!(!is_binary(b""));
    }
}
