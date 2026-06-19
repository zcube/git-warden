//! cc-langdetect: natural language detection (Korean/English/Japanese/Chinese). Corresponds to Go `internal/langdetect`.
//!
//! Preserving Go's `type Language = string`, language identifiers are handled as `&str`/`String`.
//! Unrecognised values are represented as an empty string (""), same as Go.

use std::cmp::Ordering;

/// Natural language identifier constants used in configuration.
pub const KOREAN: &str = "korean";
pub const ENGLISH: &str = "english";
pub const JAPANESE: &str = "japanese";
pub const CHINESE: &str = "chinese";
pub const ANY: &str = "any";

/// Converts a BCP-47 locale code to a Language constant. Returns "" for unrecognised values.
pub fn locale_to_language(locale: &str) -> &'static str {
    match locale {
        "ko" => KOREAN,
        "en" => ENGLISH,
        "ja" => JAPANESE,
        "zh" | "zh-hans" | "zh-hant" => CHINESE,
        _ => "",
    }
}

/// Normalises a BCP-47 code ("ko") or a legacy language name ("korean") to a canonical Language.
/// Returns "" for unrecognised values.
pub fn normalize_locale(s: &str) -> String {
    let v = s.trim().to_lowercase();
    if v.is_empty() {
        return String::new();
    }
    let lang = locale_to_language(&v);
    if !lang.is_empty() {
        return lang.to_string();
    }
    match v.as_str() {
        KOREAN | ENGLISH | JAPANESE | CHINESE | ANY => v,
        _ => String::new(),
    }
}

/// Converts a Language constant to its canonical BCP-47 code. Returns the input unchanged for unrecognised values.
pub fn language_to_bcp47(lang: &str) -> String {
    match lang {
        KOREAN => "ko".to_string(),
        ENGLISH => "en".to_string(),
        JAPANESE => "ja".to_string(),
        CHINESE => "zh".to_string(),
        other => other.to_string(),
    }
}

/// Prefixes that are always treated as technical/directive comments and skipped regardless of language.
const BUILTIN_SKIP_PREFIXES: &[&str] = &[
    "todo",
    "fixme",
    "hack",
    "note:",
    "xxx",
    "bug",
    "nolint",
    "noqa",
    "nosec",
    "noinspection",
    "go:generate",
    "go:build",
    "go:embed",
    "go:linkname",
    "+build",
    "eslint-",
    "tslint:",
    "prettier-ignore",
    "http://",
    "https://",
    "ftp://",
    "@param",
    "@return",
    "@throws",
    "@type",
    "@deprecated",
    "@ts-ignore",
    "@ts-nocheck",
    "@ts-expect-error",
    "suppress warnings",
];

/// Returns whether comment text has enough characters to perform a language check and is not a pure directive.
/// `extra_skip` is a list of project-specific additional prefixes to skip.
pub fn has_natural_language_content(text: &str, min_letters: usize, extra_skip: &[String]) -> bool {
    if text.chars().count() < min_letters {
        return false;
    }
    if is_xml_tag_only(text) {
        return false;
    }
    let lower = text.trim().to_lowercase();
    for prefix in BUILTIN_SKIP_PREFIXES {
        if lower.starts_with(prefix) {
            return false;
        }
    }
    for prefix in extra_skip {
        if lower.starts_with(&prefix.to_lowercase()) {
            return false;
        }
    }
    let count = text.chars().filter(|c| c.is_alphabetic()).count();
    count >= min_letters
}

/// Returns whether the text consists solely of XML/HTML tags (handles C# XML doc comments).
fn is_xml_tag_only(text: &str) -> bool {
    let mut s = text.trim();
    // CStyleParser strips "//" from "///", leaving "/ <tag>".
    if let Some(rest) = s.strip_prefix("/ ") {
        s = rest.trim();
    } else if s.starts_with('/') {
        s = s[1..].trim();
    }
    if s.is_empty() || !s.starts_with('<') {
        return false;
    }
    loop {
        s = s.trim();
        if s.is_empty() {
            break;
        }
        if !s.starts_with('<') {
            return false; // text exists outside of tags
        }
        match s.find('>') {
            Some(end) => s = &s[end + 1..],
            None => return false, // unclosed tag
        }
    }
    true
}

/// Returns whether comment text satisfies the required language.
/// Return value `(ok, has_content)`:
///   - has_content=false: skipped because the text is too short or is a directive
///   - ok=false: language check failed
pub fn is_required_language(
    text: &str,
    required: &str,
    min_letters: usize,
    extra_skip: &[String],
) -> (bool, bool) {
    if !has_natural_language_content(text, min_letters, extra_skip) {
        return (true, false);
    }
    if required == ANY || required.is_empty() {
        return (true, true);
    }
    // Mixed-language comments are allowed if they contain the required language.
    if has_script(text, required) {
        return (true, true);
    }
    // No characters from the required language. Check whether an identifiable language is present.
    let dom = dominant(text);
    if dom.is_empty() {
        return (true, false); // punctuation/digits only
    }
    (false, true)
}

/// Returns whether the text contains at least one character belonging to the specified language.
pub fn has_script(text: &str, lang: &str) -> bool {
    for c in text.chars() {
        let hit = match lang {
            KOREAN => is_korean(c),
            JAPANESE => is_japanese(c),
            CHINESE => is_chinese(c),
            ENGLISH => is_latin(c),
            _ => false,
        };
        if hit {
            return true;
        }
    }
    false
}

/// Returns the dominant language script. Returns "" if none is found.
fn dominant(text: &str) -> String {
    let (mut korean, mut japanese, mut chinese, mut latin) = (0usize, 0usize, 0usize, 0usize);
    for c in text.chars() {
        if is_korean(c) {
            korean += 1;
        } else if is_japanese(c) {
            japanese += 1;
        } else if is_chinese(c) {
            chinese += 1;
        } else if is_latin(c) {
            latin += 1;
        }
    }
    let mut max = korean;
    let mut dom = KOREAN;
    if japanese > max {
        max = japanese;
        dom = JAPANESE;
    }
    if chinese > max {
        max = chinese;
        dom = CHINESE;
    }
    if latin > max {
        max = latin;
        dom = ENGLISH;
    }
    if max == 0 {
        return String::new();
    }
    dom.to_string()
}

/// Returns the detected dominant natural language (public function for error messages). Returns "unknown" if none found.
pub fn dominant_language(text: &str) -> String {
    let d = dominant(text);
    if d.is_empty() {
        "unknown".to_string()
    } else {
        d
    }
}

fn is_korean(c: char) -> bool {
    let r = c as u32;
    (0xAC00..=0xD7A3).contains(&r)
        || (0x1100..=0x11FF).contains(&r)
        || (0x3130..=0x318F).contains(&r)
        || (0xA960..=0xA97F).contains(&r)
        || (0xD7B0..=0xD7FF).contains(&r)
}

fn is_japanese(c: char) -> bool {
    let r = c as u32;
    (0x3041..=0x309F).contains(&r)
        || (0x30A0..=0x30FF).contains(&r)
        || (0x31F0..=0x31FF).contains(&r)
}

fn is_chinese(c: char) -> bool {
    let r = c as u32;
    (0x4E00..=0x9FFF).contains(&r)
        || (0x3400..=0x4DBF).contains(&r)
        || (0x20000..=0x2A6DF).contains(&r)
}

fn is_latin(c: char) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_lowercase()
}

/// Replaces allowed words in text with spaces. Word boundary rule: no adjacent Latin character on either side.
/// Longer words are processed first to prevent partial matches.
pub fn strip_allowed_words(text: &str, words: &[String]) -> String {
    if words.is_empty() {
        return text.to_string();
    }
    let mut sorted: Vec<&String> = words.iter().collect();
    // Go: descending by length.
    sorted.sort_by(|a, b| {
        let (la, lb) = (a.chars().count(), b.chars().count());
        lb.cmp(&la).then(Ordering::Equal)
    });

    let runes: Vec<char> = text.chars().collect();
    let lower: Vec<char> = text.to_lowercase().chars().collect();
    // Note: to_lowercase may change the character count, but the original Go makes the same assumption (rune 1:1).
    let mut skip = vec![false; runes.len()];

    for word in sorted {
        let w_runes: Vec<char> = word.to_lowercase().chars().collect();
        let w_len = w_runes.len();
        if w_len == 0 || lower.len() < w_len {
            continue;
        }
        for i in 0..=(lower.len() - w_len) {
            if skip.get(i).copied().unwrap_or(true) {
                continue;
            }
            let mut matched = true;
            for j in 0..w_len {
                if lower[i + j] != w_runes[j] {
                    matched = false;
                    break;
                }
            }
            if !matched {
                continue;
            }
            // Word boundary check: no adjacent Latin character on either side.
            if i > 0 && is_latin(runes[i - 1]) {
                continue;
            }
            if i + w_len < runes.len() && is_latin(runes[i + w_len]) {
                continue;
            }
            for j in 0..w_len {
                skip[i + j] = true;
            }
        }
    }

    let mut sb = String::with_capacity(text.len());
    for (i, &r) in runes.iter().enumerate() {
        if skip[i] {
            sb.push(' ');
        } else {
            sb.push(r);
        }
    }
    sb
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ndr(text: &str, required: &str) -> (bool, bool) {
        is_required_language(text, required, 5, &[])
    }

    #[test]
    fn required_language_korean() {
        let cases: &[(&str, bool, bool)] = &[
            ("이것은 한국어 주석입니다", true, true),
            ("변수 name을 설정합니다", true, true),
            ("This is an English comment", false, true),
            ("TODO", true, false),
            ("nolint:errcheck", true, false),
            ("hi", true, false),
            ("https://example.com/path", true, false),
        ];
        for (text, ok, content) in cases {
            assert_eq!(ndr(text, KOREAN), (*ok, *content), "text={text:?}");
        }
    }

    #[test]
    fn required_language_any() {
        assert_eq!(ndr("anything goes here", ANY), (true, true));
    }

    #[test]
    fn required_language_english() {
        assert_eq!(ndr("This is English", ENGLISH), (true, true));
        let (ok, _) = ndr("이것은 한국어입니다", ENGLISH);
        assert!(!ok);
    }

    #[test]
    fn required_language_japanese() {
        let cases: &[(&str, bool, bool)] = &[
            ("これは日本語のコメントです", true, true),
            ("ユーザーデータを処理する", true, true),
            ("変数 name を設定する", true, true),
            ("이것은 한국어 주석입니다", false, true),
            ("This is an English comment", false, true),
            ("这是一个中文注释内容示例", false, true),
        ];
        for (text, ok, content) in cases {
            assert_eq!(ndr(text, JAPANESE), (*ok, *content), "text={text:?}");
        }
    }

    #[test]
    fn required_language_chinese() {
        let cases: &[(&str, bool, bool)] = &[
            ("这是一个中文注释内容示例", true, true),
            ("处理 user 数据的函数", true, true),
            ("이것은 한국어 주석입니다", false, true),
            ("This is an English comment", false, true),
            ("これはひらがなのコメント", false, true),
        ];
        for (text, ok, content) in cases {
            assert_eq!(ndr(text, CHINESE), (*ok, *content), "text={text:?}");
        }
    }

    #[test]
    fn cross_language() {
        let cases: &[(&str, &str, bool)] = &[
            ("이것은 한국어 주석입니다", KOREAN, true),
            ("이것은 한국어 주석입니다", JAPANESE, false),
            ("이것은 한국어 주석입니다", CHINESE, false),
            ("이것은 한국어 주석입니다", ENGLISH, false),
            ("これはひらがなとカタカナです", JAPANESE, true),
            ("これはひらがなとカタカナです", KOREAN, false),
            ("これはひらがなとカタカナです", CHINESE, false),
            ("これはひらがなとカタカナです", ENGLISH, false),
            ("これは日本語のコメントです", JAPANESE, true),
            ("これは日本語のコメントです", CHINESE, true),
            ("这是一个中文注释内容示例", CHINESE, true),
            ("这是一个中文注释内容示例", JAPANESE, false),
            ("这是一个中文注释内容示例", KOREAN, false),
            ("这是一个中文注释内容示例", ENGLISH, false),
            ("This is an English comment here", ENGLISH, true),
            ("This is an English comment here", KOREAN, false),
            ("This is an English comment here", JAPANESE, false),
            ("This is an English comment here", CHINESE, false),
        ];
        for (text, required, want_ok) in cases {
            let (ok, content) = ndr(text, required);
            assert!(content, "unexpected no-content for {text:?}");
            assert_eq!(ok, *want_ok, "text={text:?} required={required}");
        }
    }

    #[test]
    fn dominant_detection() {
        assert_eq!(dominant_language("안녕하세요 반갑습니다"), KOREAN);
        assert_eq!(dominant_language("Hello world here"), ENGLISH);
        assert_eq!(dominant_language("これは日本語です"), JAPANESE);
        assert_eq!(dominant_language("这是中文内容示例"), CHINESE);
    }

    #[test]
    fn locale_conversions() {
        assert_eq!(locale_to_language("ko"), KOREAN);
        assert_eq!(locale_to_language("zh-hant"), CHINESE);
        assert_eq!(locale_to_language("xx"), "");
        assert_eq!(normalize_locale(" KO "), KOREAN);
        assert_eq!(normalize_locale("korean"), KOREAN);
        assert_eq!(normalize_locale("nope"), "");
        assert_eq!(language_to_bcp47(KOREAN), "ko");
        assert_eq!(language_to_bcp47("any"), "any");
    }

    #[test]
    fn strip_allowed_words_boundaries() {
        let words = vec!["Java".to_string()];
        assert_eq!(
            strip_allowed_words("JavaScript 코드", &words),
            "JavaScript 코드"
        );
        assert_eq!(strip_allowed_words("Java 언어", &words), "     언어");
        let words2 = vec!["TypeScript".to_string(), "Type".to_string()];
        let out = strip_allowed_words("TypeScript 설정", &words2);
        assert_eq!(out, "           설정");
    }
}
