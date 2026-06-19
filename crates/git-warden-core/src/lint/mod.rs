//! cc-lint: syntax checks for data files (YAML/JSON/JSON5/JSONC/XML/TOML). Corresponds to Go `internal/lint`.
//!
//! Error wording may differ between Go parsers (yaml.v3/encoding/json etc.) and Rust (serde family),
//! but error detection, target file, and line number aim for equivalence.

use std::fmt;

/// A syntax error found during lint validation. Corresponds to Go `ValidationError`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub file: String,
    pub line: i64,
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "{}:{}: {}", self.file, self.line, self.message)
        } else {
            write!(f, "{}: {}", self.file, self.message)
        }
    }
}

impl ValidationError {
    fn new(file: &str, line: i64, message: String) -> Self {
        ValidationError {
            file: file.to_string(),
            line,
            message,
        }
    }
}

/// Files excluded by default from JSON lint (lock files / auto-generated). Corresponds to Go `DefaultJSONIgnoreFiles`.
pub const DEFAULT_JSON_IGNORE_FILES: &[&str] = &[
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "composer.lock",
    "Pipfile.lock",
    "Gemfile.lock",
    "Cargo.lock",
    "go.sum",
];

/// Checks whether the content is valid YAML. Corresponds to Go `ValidateYAML`.
pub fn validate_yaml(filename: &str, content: &str) -> Vec<ValidationError> {
    use serde::Deserialize;
    let mut errs = Vec::new();
    for doc in serde_yaml::Deserializer::from_str(content) {
        // Deserialize each document as a Value to detect syntax errors.
        if let Err(e) = serde_yaml::Value::deserialize(doc) {
            errs.push(ValidationError::new(
                filename,
                0,
                format!("YAML syntax error: {e}"),
            ));
            break;
        }
    }
    errs
}

/// Checks whether the content is valid JSON. Corresponds to Go `ValidateJSON`.
/// Allows whitespace-separated multiple values and empty input, identical to Go's token loop.
pub fn validate_json(filename: &str, content: &str) -> Vec<ValidationError> {
    let de = serde_json::Deserializer::from_str(content);
    for item in de.into_iter::<serde_json::Value>() {
        if let Err(e) = item {
            return vec![ValidationError::new(
                filename,
                e.line() as i64,
                format!("JSON syntax error: {e}"),
            )];
        }
    }
    Vec::new()
}

/// Strips JSON5 comments and trailing commas, then validates as JSON. Corresponds to Go `ValidateJSON5`.
pub fn validate_json5(filename: &str, content: &str) -> Vec<ValidationError> {
    match strip_json5_comments(content) {
        Ok(stripped) => validate_json(filename, &stripped),
        Err(e) => vec![ValidationError::new(
            filename,
            0,
            format!("JSON5 syntax error: {e}"),
        )],
    }
}

/// Strips JSONC comments (//, /* */) then validates as strict JSON. Corresponds to Go `ValidateJSONC`.
pub fn validate_jsonc(filename: &str, content: &str) -> Vec<ValidationError> {
    match strip_comments(content) {
        Ok(stripped) => validate_json(filename, &stripped),
        Err(e) => vec![ValidationError::new(
            filename,
            0,
            format!("JSONC syntax error: {e}"),
        )],
    }
}

/// Checks whether the content is well-formed XML. Corresponds to Go `ValidateXML` (Strict).
pub fn validate_xml(filename: &str, content: &str) -> Vec<ValidationError> {
    use quick_xml::events::Event;
    use quick_xml::Reader;
    let mut reader = Reader::from_str(content);
    // Track depth to detect unclosed tags (Go's xml.Decoder treats unclosed tags as an error at EOF).
    let mut depth: i64 = 0;
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(Event::Start(_)) => depth += 1,
            Ok(Event::End(_)) => depth -= 1,
            Ok(_) => {}
            Err(e) => {
                let line = count_lines(content, reader.buffer_position() as usize);
                return vec![ValidationError::new(
                    filename,
                    line,
                    format!("XML syntax error: {e}"),
                )];
            }
        }
    }
    if depth != 0 {
        return vec![ValidationError::new(
            filename,
            count_lines(content, content.len()),
            "XML syntax error: unexpected EOF".to_string(),
        )];
    }
    Vec::new()
}

/// Checks whether the content is valid TOML. Corresponds to Go `ValidateTOML`.
pub fn validate_toml(filename: &str, content: &str) -> Vec<ValidationError> {
    match content.parse::<toml::Table>() {
        Ok(_) => Vec::new(),
        Err(e) => {
            let line = match e.span() {
                Some(span) => count_lines(content, span.start),
                None => 1,
            };
            vec![ValidationError::new(
                filename,
                line,
                format!("TOML syntax error: {e}"),
            )]
        }
    }
}

/// Strips only //, /* */ comments from JSON/JSONC/JSON5 content (trailing commas preserved). Corresponds to Go `stripComments`.
fn strip_comments(input: &str) -> Result<String, String> {
    let runes: Vec<char> = input.chars().collect();
    let n = runes.len();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < n {
        let ch = runes[i];
        if ch == '"' || ch == '\'' {
            let quote = ch;
            out.push(ch);
            i += 1;
            while i < n {
                let c = runes[i];
                out.push(c);
                i += 1;
                if c == '\\' && i < n {
                    out.push(runes[i]);
                    i += 1;
                } else if c == quote {
                    break;
                }
            }
        } else if ch == '/' && i + 1 < n {
            let next = runes[i + 1];
            if next == '/' {
                i += 2;
                while i < n && runes[i] != '\n' {
                    i += 1;
                }
            } else if next == '*' {
                i += 2;
                let mut found = false;
                while i + 1 < n {
                    if runes[i] == '*' && runes[i + 1] == '/' {
                        i += 2;
                        found = true;
                        break;
                    }
                    if runes[i] == '\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
                if !found {
                    return Err("unterminated block comment".to_string());
                }
            } else {
                out.push(ch);
                i += 1;
            }
        } else {
            out.push(ch);
            i += 1;
        }
    }
    Ok(out)
}

/// Strips comments and trailing commas from JSON5 content. Corresponds to Go `StripJSON5Comments`.
pub fn strip_json5_comments(input: &str) -> Result<String, String> {
    let stripped = strip_comments(input)?;
    Ok(strip_trailing_commas(&stripped))
}

/// Removes commas (with optional whitespace) immediately before } or ]. Corresponds to Go `stripTrailingCommas`.
fn strip_trailing_commas(s: &str) -> String {
    let runes: Vec<char> = s.chars().collect();
    let n = runes.len();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < n {
        if runes[i] == ',' {
            let mut j = i + 1;
            while j < n && matches!(runes[j], ' ' | '\t' | '\n' | '\r') {
                j += 1;
            }
            if j < n && (runes[j] == '}' || runes[j] == ']') {
                i += 1;
                continue;
            }
        }
        out.push(runes[i]);
        i += 1;
    }
    out
}

/// Returns whether the file contains a "git-warden: skip-lint" directive. Corresponds to Go `HasLintDisableComment`.
pub fn has_lint_disable_comment(content: &str, comment_prefix: &str) -> bool {
    let prefix = format!("{comment_prefix} git-warden: skip-lint");
    content
        .split('\n')
        .any(|line| line.trim().contains(&prefix))
}

/// Returns the 1-based line number for a byte offset. Corresponds to Go `countLines`.
fn count_lines(content: &str, offset: usize) -> i64 {
    let offset = offset.min(content.len());
    // offset may not be on a character boundary, so count newlines using a byte slice.
    content.as_bytes()[..offset]
        .iter()
        .filter(|&&b| b == b'\n')
        .count() as i64
        + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_valid_invalid_empty_multidoc() {
        assert!(validate_yaml("t.yaml", "key: value\nlist:\n  - a\n  - b\n").is_empty());
        assert!(!validate_yaml("t.yaml", "key: [invalid: yaml: here").is_empty());
        assert!(validate_yaml("t.yaml", "").is_empty());
        assert!(validate_yaml("t.yaml", "---\nfoo: bar\n---\nbaz: qux\n").is_empty());
    }

    #[test]
    fn json_cases() {
        assert!(validate_json("t.json", r#"{"key": "value", "list": [1,2,3]}"#).is_empty());
        assert!(!validate_json("t.json", r#"{"key": "value",}"#).is_empty());
        assert!(!validate_json("t.json", r#"{key: value}"#).is_empty());
        assert!(validate_json("t.json", "").is_empty());
    }

    #[test]
    fn json5_cases() {
        let c =
            "{\n  // single\n  \"key\": \"value\",\n  /* multi\n  line */\n  \"list\": [1,2,3],\n}";
        assert!(validate_json5("t.json", c).is_empty());
        assert!(validate_json5("t.json", r#"{"key": "value", "list": [1,2,3,],}"#).is_empty());
        assert!(!validate_json5("t.json", r#"{key: }"#).is_empty());
        assert!(validate_json5("t.json", r#"{"url": "http://example.com"}"#).is_empty());
        assert!(!validate_json5("t.json", r#"{"key": "value" /* unterminated"#).is_empty());
    }

    #[test]
    fn jsonc_cases() {
        assert!(validate_jsonc("t.jsonc", "{\n// c\n\"key\": \"value\"\n}").is_empty());
        assert!(validate_jsonc("t.jsonc", "{\n/* b */\n\"key\": \"value\"\n}").is_empty());
        assert!(!validate_jsonc("t.jsonc", r#"{"key": "value",}"#).is_empty());
        assert!(!validate_jsonc("t.jsonc", r#"{key: value}"#).is_empty());
    }

    #[test]
    fn xml_cases() {
        assert!(validate_xml(
            "t.xml",
            r#"<?xml version="1.0"?><root><item>v</item></root>"#
        )
        .is_empty());
        assert!(!validate_xml("t.xml", "<root><unclosed>").is_empty());
        assert!(validate_xml("t.xml", "").is_empty());
    }

    #[test]
    fn toml_cases() {
        assert!(validate_toml("t.toml", "[section]\nkey = \"value\"\nnumber = 42\n").is_empty());
        assert!(!validate_toml("t.toml", "invalid = [unclosed\n").is_empty());
        assert!(validate_toml("t.toml", "").is_empty());
    }

    #[test]
    fn has_disable_comment() {
        assert!(has_lint_disable_comment(
            "# git-warden: skip-lint\nkey: value\n",
            "#"
        ));
        assert!(has_lint_disable_comment(
            "{\n// git-warden: skip-lint\n\"key\": \"value\"\n}",
            "//"
        ));
        assert!(!has_lint_disable_comment("key: value\n# normal\n", "#"));
    }

    #[test]
    fn strip_json5_preserves_strings_and_strips_comments() {
        let r = strip_json5_comments("{\n// comment\n\"key\": \"value\"\n}").unwrap();
        assert!(!r.contains("// comment"));
        assert!(r.contains("\"key\""));
        let r2 = strip_json5_comments(r#"{"url": "http://example.com/path"}"#).unwrap();
        assert_eq!(r2, r#"{"url": "http://example.com/path"}"#);
    }

    #[test]
    fn strip_trailing_commas_cases() {
        assert_eq!(strip_trailing_commas("[1, 2, 3,]"), "[1, 2, 3]");
        assert_eq!(strip_trailing_commas(r#"{"a": 1,}"#), r#"{"a": 1}"#);
        assert_eq!(strip_trailing_commas("[1, 2, 3]"), "[1, 2, 3]");
        assert_eq!(
            strip_trailing_commas(r#"{"a": 1, "b": 2,}"#),
            r#"{"a": 1, "b": 2}"#
        );
    }

    #[test]
    fn validation_error_string() {
        assert_eq!(
            ValidationError::new("test.json", 5, "syntax error".into()).to_string(),
            "test.json:5: syntax error"
        );
        assert_eq!(
            ValidationError::new("test.json", 0, "error".into()).to_string(),
            "test.json: error"
        );
    }
}
