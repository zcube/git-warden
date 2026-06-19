//! error.rs: converts YAML parse errors into human-readable messages. Corresponds to Go `internal/config/error.go`.
//!
//! Type errors (e.g. a scalar where an object is expected) are reported with per-field hints and
//! correct YAML examples. Go's yaml.TypeError collects multiple errors at once, but serde_yaml
//! stops at the first error, so this port reports the first type error in detail (the rest are
//! surfaced on subsequent runs).

/// User-friendly field name and correct YAML example for a specific config struct. Go `fieldHint`.
struct FieldHint {
    field: &'static str,
    example: &'static str,
}

/// Maps serde-reported struct names to field descriptions/examples. Mirrors Go `typeHints`.
fn type_hint(struct_name: &str) -> Option<FieldHint> {
    let h = |field, example| Some(FieldHint { field, example });
    match struct_name {
        // XML and TOML share the LintRuleConfig type.
        "LintRuleConfig" => h(
            "lint.yaml or lint.xml",
            "lint:\n  yaml:\n    enabled: true\n  xml:\n    enabled: true",
        ),
        "JsonLintConfig" => h(
            "lint.json",
            "lint:\n  json:\n    enabled: true\n    allow_json5: false",
        ),
        "LintConfig" => h("lint", "lint:\n  enabled: true\n  yaml:\n    enabled: true"),
        "BinaryFileConfig" => h("binary_file", "binary_file:\n  enabled: true"),
        "EncodingConfig" => h(
            "encoding",
            "encoding:\n  enabled: true\n  require_utf8: true",
        ),
        "EditorConfigConfig" => h("editorconfig", "editorconfig:\n  enabled: true"),
        "CommentLanguageConfig" => h(
            "comment_language",
            "comment_language:\n  enabled: true\n  required_language: korean",
        ),
        "CommitMessageConfig" => h(
            "commit_message",
            "commit_message:\n  enabled: true\n  no_ai_coauthor: true",
        ),
        "ConventionalCommitConfig" => h(
            "commit_message.conventional_commit",
            "commit_message:\n  conventional_commit:\n    enabled: true",
        ),
        "CommitMessageLanguageConfig" => h(
            "commit_message.language_check",
            "commit_message:\n  language_check:\n    enabled: true\n    required_language: korean",
        ),
        _ => None,
    }
}

/// Formats a config parse error message. Corresponds to Go `formatConfigError`.
pub fn format_config_error(cfg_path: &str, err: &str) -> String {
    // Non-type errors (syntax errors, etc.) get a unified message.
    if !err.contains("invalid type:") {
        return cc_i18n::t!("config.syntax_error", Path = cfg_path, Error = err);
    }

    let line = extract_line(err);
    let (yaml_type, value) = extract_type_value(err);
    let expected = extract_expected_struct(err);

    let mut out = cc_i18n::t!("config.type_error_header", Path = cfg_path);
    out.push('\n');

    match expected.clone().and_then(|s| type_hint(&s)) {
        Some(hint) => {
            out.push_str(&cc_i18n::t!(
                "config.type_error_object_required",
                Line = line,
                Field = hint.field,
                Type = yaml_type,
                Value = value
            ));
            out.push('\n');
            out.push_str(&cc_i18n::t!("config.type_error_example_header"));
            out.push('\n');
            for ex in hint.example.split('\n') {
                out.push_str(&cc_i18n::t!("config.type_error_example_line", Line = ex));
                out.push('\n');
            }
        }
        None => {
            // Unknown struct or non-struct type error → fall back to generic message.
            let go_type = expected.unwrap_or_else(|| extract_expected_any(err));
            out.push_str(&cc_i18n::t!(
                "config.type_error_generic",
                Line = line,
                GoType = go_type,
                Type = yaml_type,
                Value = value
            ));
            out.push('\n');
        }
    }

    out.trim_end().to_string()
}

/// Extracts the line number from "at line N". Returns "?" if not found.
fn extract_line(err: &str) -> String {
    if let Some(idx) = err.find("at line ") {
        let rest = &err[idx + "at line ".len()..];
        let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !num.is_empty() {
            return num;
        }
    }
    "?".to_string()
}

/// Extracts (Go-style type name, value) from "invalid type: <T> `v`" or "<T> \"v\"".
fn extract_type_value(err: &str) -> (String, String) {
    let Some(idx) = err.find("invalid type: ") else {
        return (String::new(), String::new());
    };
    let rest = &err[idx + "invalid type: ".len()..];
    // Everything before ", expected" is the "<type> <value>" segment.
    let seg = rest.split(", expected").next().unwrap_or(rest);

    // Value: the part wrapped in backticks or double quotes.
    let (type_part, value) = if let Some(b0) = seg.find('`') {
        let after = &seg[b0 + 1..];
        let v = after.split('`').next().unwrap_or("").to_string();
        (seg[..b0].trim().to_string(), v)
    } else if let Some(q0) = seg.find('"') {
        let after = &seg[q0 + 1..];
        let v = after.split('"').next().unwrap_or("").to_string();
        (seg[..q0].trim().to_string(), v)
    } else {
        (seg.trim().to_string(), String::new())
    };

    (map_yaml_type(&type_part), value)
}

/// Maps serde type names to Go YAML short tag names (str/bool/int, etc.).
fn map_yaml_type(t: &str) -> String {
    match t {
        "boolean" => "bool",
        "string" => "str",
        "integer" => "int",
        "floating point" => "float",
        "sequence" => "seq",
        "map" => "map",
        "unit value" | "null" => "null",
        other => other.split_whitespace().next().unwrap_or(other),
    }
    .to_string()
}

/// Extracts the struct name X from "expected struct X".
fn extract_expected_struct(err: &str) -> Option<String> {
    let idx = err.find("expected struct ")?;
    let rest = &err[idx + "expected struct ".len()..];
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Extracts the expected type description from non-struct errors like "expected a string" (used for the generic message).
fn extract_expected_any(err: &str) -> String {
    if let Some(idx) = err.find("expected ") {
        let rest = &err[idx + "expected ".len()..];
        let seg = rest.split(" at line").next().unwrap_or(rest);
        return seg
            .trim_start_matches("a ")
            .trim_start_matches("an ")
            .to_string();
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_error_path() {
        // Syntax errors (not type errors) use the unified syntax_error message (assert locale-agnostic content).
        let m = format_config_error("c.yml", "did not find expected key at line 3 column 1");
        assert!(m.contains("c.yml"));
        assert!(m.contains("did not find expected key"));
    }

    #[test]
    fn type_error_object_required_with_example() {
        // The global locale races with other tests, so only assert locale-agnostic content.
        let serde =
            "lint: invalid type: boolean `true`, expected struct LintConfig at line 1 column 7";
        let m = format_config_error("c.yml", serde);
        assert!(m.contains("c.yml"));
        assert!(m.contains("'lint'"));
        assert!(m.contains("bool(true)"));
        // The example YAML (locale-agnostic) must be present.
        assert!(m.contains("enabled: true"));
    }

    #[test]
    fn type_error_string_value() {
        let serde = "binary_file: invalid type: string \"nope\", expected struct BinaryFileConfig at line 2 column 14";
        let m = format_config_error("c.yml", serde);
        assert!(m.contains("'binary_file'"));
        assert!(m.contains("str(nope)"));
        assert!(m.contains("binary_file:\n"));
    }

    #[test]
    fn type_helpers() {
        assert_eq!(extract_line("x at line 7 column 1"), "7");
        assert_eq!(map_yaml_type("boolean"), "bool");
        assert_eq!(map_yaml_type("string"), "str");
        let (t, v) = extract_type_value(
            "invalid type: boolean `true`, expected struct X at line 1 column 1",
        );
        assert_eq!((t.as_str(), v.as_str()), ("bool", "true"));
        assert_eq!(
            extract_expected_struct("... expected struct LintConfig at line 1").as_deref(),
            Some("LintConfig")
        );
    }
}
