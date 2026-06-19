//! cc-editorconfig: .editorconfig rule checking. Corresponds to Go `internal/editorconfig`.
//! .editorconfig parsing uses the ec4rs crate (corresponding to Go's editorconfig-core-go).
//! Violation messages retain the same English literals as the original Go implementation.

use std::fmt;
use std::path::Path;

/// An .editorconfig rule violation. Corresponds to Go `Violation`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub file: String,
    pub line: i64,
    pub message: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "{}:{}: {}", self.file, self.line, self.message)
        } else {
            write!(f, "{}: {}", self.file, self.message)
        }
    }
}

/// The editorconfig definition applied to a file path. Covers the used portion of Go `ec.Definition`.
#[derive(Debug, Clone, Default)]
pub struct Definition {
    pub charset: String,
    pub end_of_line: String,
    pub insert_final_newline: Option<bool>,
    pub trim_trailing_whitespace: Option<bool>,
    pub indent_style: String,
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.to_lowercase().as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// Returns the editorconfig definition for a file path. Corresponds to Go `GetDefinition`.
/// Returns None if no .editorconfig is found or on error.
pub fn get_definition(file_path: &str) -> Option<Definition> {
    let props = ec4rs::properties_of(Path::new(file_path)).ok()?;
    let raw = |key: &str| -> Option<String> {
        props
            .get_raw_for_key(key)
            .into_option()
            .map(|s| s.to_string())
    };
    Some(Definition {
        charset: raw("charset").map(|s| s.to_lowercase()).unwrap_or_default(),
        end_of_line: raw("end_of_line")
            .map(|s| s.to_lowercase())
            .unwrap_or_default(),
        insert_final_newline: raw("insert_final_newline").and_then(|s| parse_bool(&s)),
        trim_trailing_whitespace: raw("trim_trailing_whitespace").and_then(|s| parse_bool(&s)),
        indent_style: raw("indent_style")
            .map(|s| s.to_lowercase())
            .unwrap_or_default(),
    })
}

/// Validates file content against an editorconfig definition. Corresponds to Go `Check`.
pub fn check(filename: &str, content: &[u8], def: &Definition) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text = String::from_utf8_lossy(content).to_string();
    let lines: Vec<&str> = text.split('\n').collect();

    // charset check: BOM present when charset=utf-8 is a violation.
    if def.charset == "utf-8"
        && content.len() >= 3
        && content[0] == 0xEF
        && content[1] == 0xBB
        && content[2] == 0xBF
    {
        violations.push(Violation {
            file: filename.to_string(),
            line: 1,
            message: "file has UTF-8 BOM but charset=utf-8 (no BOM expected)".to_string(),
        });
    }

    // Line ending check.
    if !def.end_of_line.is_empty() {
        for (i, line) in lines.iter().enumerate() {
            if i == lines.len() - 1 {
                break;
            }
            let has_cr = line.ends_with('\r');
            if def.end_of_line == "lf" && has_cr {
                violations.push(Violation {
                    file: filename.to_string(),
                    line: (i + 1) as i64,
                    message: "expected LF line ending, found CRLF".to_string(),
                });
                break;
            }
            if def.end_of_line == "crlf" && !has_cr {
                violations.push(Violation {
                    file: filename.to_string(),
                    line: (i + 1) as i64,
                    message: "expected CRLF line ending, found LF".to_string(),
                });
                break;
            }
        }
    }

    // Final newline check.
    if def.insert_final_newline == Some(true) && !text.is_empty() && !text.ends_with('\n') {
        violations.push(Violation {
            file: filename.to_string(),
            line: 0,
            message: "file must end with a newline".to_string(),
        });
    }

    // Trailing whitespace check.
    if def.trim_trailing_whitespace == Some(true) {
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_end_matches([' ', '\t', '\r']);
            if trimmed != line.trim_end_matches('\r') {
                violations.push(Violation {
                    file: filename.to_string(),
                    line: (i + 1) as i64,
                    message: "trailing whitespace".to_string(),
                });
                break;
            }
        }
    }

    // Indentation style check (sample up to 100 indented lines).
    if !def.indent_style.is_empty() {
        let mut checked = 0;
        for (i, line) in lines.iter().enumerate() {
            if checked >= 100 {
                break;
            }
            let trimmed = line.trim_start_matches([' ', '\t', '\r']);
            if trimmed.is_empty() || trimmed == *line {
                continue;
            }
            checked += 1;
            let indent = &line[..line.len() - trimmed.len()];

            if def.indent_style == "space" && indent.contains('\t') {
                violations.push(Violation {
                    file: filename.to_string(),
                    line: (i + 1) as i64,
                    message: "expected spaces for indentation, found tabs".to_string(),
                });
                break;
            }
            if def.indent_style == "tab" && !indent.starts_with('\t') && indent.contains(' ') {
                violations.push(Violation {
                    file: filename.to_string(),
                    line: (i + 1) as i64,
                    message: "expected tabs for indentation, found spaces".to_string(),
                });
                break;
            }
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(
        indent: &str,
        eol: &str,
        charset: &str,
        ifn: Option<bool>,
        ttw: Option<bool>,
    ) -> Definition {
        Definition {
            charset: charset.to_string(),
            end_of_line: eol.to_string(),
            insert_final_newline: ifn,
            trim_trailing_whitespace: ttw,
            indent_style: indent.to_string(),
        }
    }

    #[test]
    fn utf8_bom_violation() {
        let d = def("", "", "utf-8", None, None);
        let mut content = vec![0xEF, 0xBB, 0xBF];
        content.extend_from_slice(b"hello\n");
        let v = check("f", &content, &d);
        assert_eq!(v.len(), 1);
        assert!(v[0].message.contains("BOM"));
        assert!(check("f", b"hello\n", &d).is_empty());
    }

    #[test]
    fn end_of_line_lf_crlf() {
        let lf = def("", "lf", "", None, None);
        assert!(!check("f", b"a\r\nb\n", &lf).is_empty());
        assert!(check("f", b"a\nb\n", &lf).is_empty());
        let crlf = def("", "crlf", "", None, None);
        assert!(!check("f", b"a\nb\n", &crlf).is_empty());
    }

    #[test]
    fn insert_final_newline() {
        let d = def("", "", "", Some(true), None);
        assert!(!check("f", b"no newline", &d).is_empty());
        assert!(check("f", b"with newline\n", &d).is_empty());
        assert!(check("f", b"", &d).is_empty());
    }

    #[test]
    fn trailing_whitespace() {
        let d = def("", "", "", None, Some(true));
        assert!(!check("f", b"line with space \nok\n", &d).is_empty());
        assert!(check("f", b"clean\nlines\n", &d).is_empty());
    }

    #[test]
    fn indent_style_space_tab() {
        let space = def("space", "", "", None, None);
        assert!(!check("f", "\tindented\n".as_bytes(), &space).is_empty());
        assert!(check("f", "    indented\n".as_bytes(), &space).is_empty());
        let tab = def("tab", "", "", None, None);
        assert!(!check("f", "    spaces\n".as_bytes(), &tab).is_empty());
        assert!(check("f", "\ttabbed\n".as_bytes(), &tab).is_empty());
    }

    #[test]
    fn empty_definition_no_violations() {
        let d = Definition::default();
        assert!(check("f", "anything \t\r\n  mixed\n".as_bytes(), &d).is_empty());
    }
}
