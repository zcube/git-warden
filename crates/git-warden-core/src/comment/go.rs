//! Comment, string, and import extraction for Go source. Corresponds to Go `go_parser.go`.
//!
//! The original Go implementation uses the go/parser AST; this Rust port reproduces equivalent
//! behaviour with a state machine: `//`/`/* */` comments, `"..."`/backtick strings,
//! `'...'` runes (not extracted), and import paths (KindImport).

use super::{clean_block_comment, Comment, Kind, Parser};

pub struct GoParser;

#[derive(PartialEq)]
enum St {
    Code,
    Line,
    Block,
    DQ,
    Raw,
    Rune,
}

impl Parser for GoParser {
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec![".go"]
    }

    fn parse_file(&self, content: &str) -> Result<Vec<Comment>, String> {
        let runes: Vec<char> = content.chars().collect();
        let n = runes.len();

        // go/parser only collects comments/strings when a valid `package <identifier>` clause exists.
        // If the clause is absent or malformed (including with leading comments), f.Comments is empty
        // and f != nil, so the result is empty with no error (see the `err != nil && f == nil` branch
        // in `go_parser.go`). Even if the body is broken, comments are collected as long as the
        // package clause is valid, so the gate checks only the package clause.
        if !has_valid_package_clause(&runes) {
            return Ok(Vec::new());
        }

        let mut result: Vec<Comment> = Vec::new();
        let mut state = St::Code;
        let mut buf = String::new();
        let mut line_pre = String::new();
        let mut comment_line = 0i64;
        let mut str_line = 0i64;
        let mut line = 1i64;
        let mut import_block = false;

        let peek = |i: usize| -> char {
            if i + 1 < n {
                runes[i + 1]
            } else {
                '\0'
            }
        };

        // Whether the current string is in an import-path context: inside an import block
        // or the first token on the line is "import".
        // Covers `import "x"`, `import f "x"`, `import _ "x"`, `import . "x"`.
        let import_kind = |line_pre: &str, import_block: bool| -> Kind {
            let first = line_pre.split_whitespace().next();
            if import_block || first == Some("import") {
                Kind::Import
            } else {
                Kind::String
            }
        };

        let mut i = 0;
        while i < n {
            let ch = runes[i];
            match state {
                St::Code => {
                    if ch == '\n' {
                        line += 1;
                        line_pre.clear();
                    } else if ch == '/' && peek(i) == '/' {
                        state = St::Line;
                        comment_line = line;
                        i += 1;
                    } else if ch == '/' && peek(i) == '*' {
                        state = St::Block;
                        comment_line = line;
                        i += 1;
                    } else if ch == '"' {
                        state = St::DQ;
                        str_line = line;
                        buf.clear();
                    } else if ch == '`' {
                        state = St::Raw;
                        str_line = line;
                        buf.clear();
                    } else if ch == '\'' {
                        state = St::Rune;
                    } else {
                        // Track import block entry/exit.
                        if ch == '(' && line_pre.trim().ends_with("import") {
                            import_block = true;
                        } else if ch == ')' && import_block {
                            import_block = false;
                        }
                        line_pre.push(ch);
                    }
                }
                St::Line => {
                    if ch == '\n' {
                        // go/parser also collects empty comments (`//`), so push empty text as-is
                        // (required to maintain line continuity for adjacent line-comment grouping).
                        result.push(Comment {
                            text: buf.trim().to_string(),
                            line: comment_line,
                            end_line: line,
                            is_block: false,
                            kind: Kind::Comment,
                        });
                        buf.clear();
                        state = St::Code;
                        line += 1;
                        line_pre.clear();
                    } else {
                        buf.push(ch);
                    }
                }
                St::Block => {
                    if ch == '*' && peek(i) == '/' {
                        let text = clean_block_comment(&buf);
                        result.push(Comment {
                            text,
                            line: comment_line,
                            end_line: line,
                            is_block: true,
                            kind: Kind::Comment,
                        });
                        buf.clear();
                        state = St::Code;
                        i += 1;
                    } else {
                        if ch == '\n' {
                            line += 1;
                        }
                        buf.push(ch);
                    }
                }
                St::DQ => {
                    if ch == '\n' {
                        // Unclosed interpreted string (terminated by newline): Go's unquote-failure
                        // fallback uses the raw interior with the last character removed
                        // (go_parser.go: raw[1:len-1]).
                        let v = drop_last_char(&buf);
                        buf.clear();
                        emit_value(
                            &mut result,
                            v,
                            str_line,
                            line,
                            import_kind(&line_pre, import_block),
                        );
                        line += 1;
                        line_pre.clear();
                        state = St::Code;
                    } else if ch == '\\' && i + 1 < n {
                        // Preserve escape sequences as-is (prevents misidentifying the closing quote).
                        buf.push('\\');
                        buf.push(runes[i + 1]);
                        i += 1;
                    } else if ch == '"' {
                        let v = unquote_interpreted(&buf);
                        buf.clear();
                        emit_value(
                            &mut result,
                            v,
                            str_line,
                            line,
                            import_kind(&line_pre, import_block),
                        );
                        state = St::Code;
                    } else {
                        buf.push(ch);
                    }
                }
                St::Raw => {
                    if ch == '`' {
                        // Raw string: go/scanner removes CR (\r) from the value.
                        let v = buf.replace('\r', "");
                        buf.clear();
                        emit_value(
                            &mut result,
                            v,
                            str_line,
                            line,
                            import_kind(&line_pre, import_block),
                        );
                        state = St::Code;
                    } else {
                        if ch == '\n' {
                            line += 1;
                        }
                        buf.push(ch);
                    }
                }
                St::Rune => {
                    // Rune literals are not extracted — skip to the closing ' (handle escapes).
                    if ch == '\\' && i + 1 < n {
                        i += 1;
                    } else if ch == '\'' {
                        state = St::Code;
                    } else if ch == '\n' {
                        line += 1;
                        state = St::Code;
                    }
                }
            }
            i += 1;
        }

        // Handle case where the file ends without a newline.
        if state == St::Line {
            // Preserve empty comments as well (same as line-comment handling above).
            result.push(Comment {
                text: buf.trim().to_string(),
                line: comment_line,
                end_line: line,
                is_block: false,
                kind: Kind::Comment,
            });
        } else if state == St::DQ {
            // Unclosed interpreted string ending at EOF: same fallback as the newline case above.
            let v = drop_last_char(&buf);
            emit_value(
                &mut result,
                v,
                str_line,
                line,
                import_kind(&line_pre, import_block),
            );
        } else if state == St::Raw {
            let v = buf.replace('\r', "");
            emit_value(
                &mut result,
                v,
                str_line,
                line,
                import_kind(&line_pre, import_block),
            );
        }

        // Stable sort by line number; within the same line, comments come before strings/imports
        // (go_parser.go collects comments first and then sorts, so comments precede other items on the same line).
        result.sort_by_key(|c| (c.line, if c.kind == Kind::Comment { 0 } else { 1 }));
        Ok(result)
    }
}

/// Returns true if the source begins with a valid `package <identifier>` clause (the condition
/// under which go/parser collects comments). Leading whitespace and `//`/`/* */` comments are
/// skipped; the `package` keyword and an identifier must follow.
fn has_valid_package_clause(runes: &[char]) -> bool {
    let n = runes.len();
    let mut i = skip_ws_comments(runes, 0);

    // `package` keyword (must be followed by an identifier boundary).
    const KW: [char; 7] = ['p', 'a', 'c', 'k', 'a', 'g', 'e'];
    if i + KW.len() > n || runes[i..i + KW.len()] != KW {
        return false;
    }
    let after = i + KW.len();
    // If the character immediately after the keyword is an identifier character, it is a different
    // token (e.g., `packagex`) — not a valid package clause.
    if after < n && is_ident_char(runes[after]) {
        return false;
    }
    i = skip_ws_comments(runes, after);

    // Package name: must start with a valid identifier character (Unicode letter or '_').
    i < n && (runes[i].is_alphabetic() || runes[i] == '_')
}

/// Returns the index after skipping whitespace and comments (`//`, `/* */`) starting from runes[start..].
fn skip_ws_comments(runes: &[char], start: usize) -> usize {
    let n = runes.len();
    let mut i = start;
    loop {
        while i < n && runes[i].is_whitespace() {
            i += 1;
        }
        if i + 1 < n && runes[i] == '/' && runes[i + 1] == '/' {
            i += 2;
            while i < n && runes[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if i + 1 < n && runes[i] == '/' && runes[i + 1] == '*' {
            i += 2;
            while i + 1 < n && !(runes[i] == '*' && runes[i + 1] == '/') {
                i += 1;
            }
            i = (i + 2).min(n); // consume `*/` (advance to end if unclosed).
            continue;
        }
        break;
    }
    i
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Returns the string with its last character removed (fallback for unclosed strings).
fn drop_last_char(s: &str) -> String {
    let mut v = s.to_string();
    v.pop();
    v
}

/// Pushes a string value. Skips empty values (`val == ""`), matching go_parser.go behaviour.
fn emit_value(result: &mut Vec<Comment>, val: String, str_line: i64, end_line: i64, kind: Kind) {
    if !val.is_empty() {
        result.push(Comment {
            text: val,
            line: str_line,
            end_line,
            is_block: false,
            kind,
        });
    }
}

/// Unescapes the interior (excluding quotes) of a Go interpreted string (`"..."`) equivalently to strconv.Unquote.
/// On failure, returns the raw interior string as-is, matching go_parser.go behaviour.
fn unquote_interpreted(inner: &str) -> String {
    match try_unquote_interpreted(inner) {
        Some(s) => s,
        None => inner.to_string(),
    }
}

fn try_unquote_interpreted(inner: &str) -> Option<String> {
    let b = inner.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'\\' {
            out.push(b[i]);
            i += 1;
            continue;
        }
        i += 1;
        if i >= b.len() {
            return None;
        }
        match b[i] {
            b'a' => {
                out.push(0x07);
                i += 1;
            }
            b'b' => {
                out.push(0x08);
                i += 1;
            }
            b'f' => {
                out.push(0x0c);
                i += 1;
            }
            b'n' => {
                out.push(b'\n');
                i += 1;
            }
            b'r' => {
                out.push(b'\r');
                i += 1;
            }
            b't' => {
                out.push(b'\t');
                i += 1;
            }
            b'v' => {
                out.push(0x0b);
                i += 1;
            }
            b'\\' => {
                out.push(b'\\');
                i += 1;
            }
            b'"' => {
                out.push(b'"');
                i += 1;
            }
            b'\'' => {
                out.push(b'\'');
                i += 1;
            }
            b'x' => {
                // \xHH (2 hex digits = 1 byte)
                let h = b.get(i + 1..i + 3)?;
                let v = u8::from_str_radix(std::str::from_utf8(h).ok()?, 16).ok()?;
                out.push(v);
                i += 3;
            }
            b'u' | b'U' => {
                // \uHHHH (4 digits) / \UHHHHHHHH (8 digits) → Unicode code point
                let len = if b[i] == b'u' { 4 } else { 8 };
                let h = b.get(i + 1..i + 1 + len)?;
                let cp = u32::from_str_radix(std::str::from_utf8(h).ok()?, 16).ok()?;
                let ch = char::from_u32(cp)?;
                let mut tmp = [0u8; 4];
                out.extend_from_slice(ch.encode_utf8(&mut tmp).as_bytes());
                i += 1 + len;
            }
            b'0'..=b'7' => {
                // \ooo (exactly 3 octal digits, value < 256)
                let h = b.get(i..i + 3)?;
                if !h.iter().all(|c| (b'0'..=b'7').contains(c)) {
                    return None;
                }
                let v = u16::from_str_radix(std::str::from_utf8(h).ok()?, 8).ok()?;
                if v > 0xff {
                    return None;
                }
                out.push(v as u8);
                i += 3;
            }
            _ => return None,
        }
    }
    // All valid escape sequences have been handled. Go strings can hold arbitrary bytes, but
    // Rust String cannot, so apply lossy conversion if the result is non-UTF-8 (e.g., `\x80`).
    // This value is only used as KindString/Import, not for language checking, so lossy conversion
    // has no impact on output.
    Some(String::from_utf8_lossy(&out).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Matching go/parser behaviour: comments are only extracted when a valid package clause exists.
    fn count(src: &str) -> usize {
        GoParser.parse_file(src).unwrap().len()
    }

    #[test]
    fn no_package_clause_yields_no_comments() {
        // Leading comment + non-package token → go/parser returns zero comments.
        assert_eq!(count("// english comment here\nfunc main() {}\n"), 0);
        assert_eq!(count("// just a comment\n"), 0);
        assert_eq!(count("@@@bad\n// english comment\n"), 0);
    }

    #[test]
    fn valid_package_yields_comments() {
        assert_eq!(
            count("package main\n// english comment\nfunc main(){}\n"),
            1
        );
        // Even if the body is malformed, comments are collected as long as the package clause is valid.
        assert_eq!(count("package main\n// c1\nfunc (\n"), 1);
        // Leading comment + valid package clause: the leading comment is also collected.
        assert_eq!(count("// header\npackage main\n// c1\n"), 2);
        // Block comment before the package clause.
        assert_eq!(count("/* license */\npackage main\n// c\n"), 2);
    }

    #[test]
    fn packagex_is_not_keyword() {
        // `packagex` is an identifier — not a valid package clause.
        assert_eq!(count("packagex main\n// c\n"), 0);
    }

    #[test]
    fn empty_line_comments_preserved() {
        // Empty `//` comments are preserved to maintain line continuity (grouping accuracy).
        assert_eq!(count("package main\n//\n//\n//\nvar x=1\n"), 3);
    }

    #[test]
    fn interpreted_string_escapes_unquoted() {
        // Verify that escape sequences are unquoted equivalently to strconv.Unquote (value preservation).
        let p = GoParser;
        let cs = Parser::parse_file(&p, "package main\nvar s = \"a\\tb\\u00e9c\"\n").unwrap();
        let s = cs.iter().find(|c| c.kind == Kind::String).unwrap();
        assert_eq!(s.text, "a\tbéc");
    }

    #[test]
    fn import_alias_classified_as_import() {
        let p = GoParser;
        let cs =
            Parser::parse_file(&p, "package main\nimport f \"fmt\"\nimport _ \"io\"\n").unwrap();
        assert!(cs
            .iter()
            .all(|c| c.kind == Kind::Import || c.kind == Kind::Comment));
        assert_eq!(cs.iter().filter(|c| c.kind == Kind::Import).count(), 2);
    }
}
