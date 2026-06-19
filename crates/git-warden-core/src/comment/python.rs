//! Extracts `#` comments, strings, and docstrings from Python source, plus PEP 723 block marking. Corresponds to Go `python_parser.go`.

use super::{Comment, Kind, Parser};

pub struct PythonParser;

#[derive(PartialEq)]
enum St {
    Code,
    Line,
    DQ,
    SQ,
    TripleDQ,
    TripleSQ,
}

impl Parser for PythonParser {
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec![".py"]
    }

    fn parse_file(&self, content: &str) -> Result<Vec<Comment>, String> {
        let runes: Vec<char> = content.chars().collect();
        let n = runes.len();
        let mut result: Vec<Comment> = Vec::new();
        let mut state = St::Code;
        let mut buf = String::new();
        let mut comment_line = 0i64;
        let mut str_line = 0i64;
        let mut line = 1i64;

        let peek_n = |i: usize, off: usize| -> char {
            if i + off < n {
                runes[i + off]
            } else {
                '\0'
            }
        };

        let mut i = 0;
        while i < n {
            let ch = runes[i];
            match state {
                St::Code => {
                    if ch == '\n' {
                        line += 1;
                    } else if ch == '#' {
                        state = St::Line;
                        comment_line = line;
                    } else if ch == '"' && peek_n(i, 1) == '"' && peek_n(i, 2) == '"' {
                        state = St::TripleDQ;
                        str_line = line;
                        buf.clear();
                        i += 2;
                    } else if ch == '\'' && peek_n(i, 1) == '\'' && peek_n(i, 2) == '\'' {
                        state = St::TripleSQ;
                        str_line = line;
                        buf.clear();
                        i += 2;
                    } else if ch == '"' {
                        state = St::DQ;
                        str_line = line;
                        buf.clear();
                    } else if ch == '\'' {
                        state = St::SQ;
                        str_line = line;
                        buf.clear();
                    }
                }
                St::Line => {
                    if ch == '\n' {
                        push_line_comment(&mut result, &buf, comment_line, line);
                        buf.clear();
                        state = St::Code;
                        line += 1;
                    } else {
                        buf.push(ch);
                    }
                }
                St::DQ => {
                    if ch == '\n' {
                        emit_string(&mut result, &mut buf, str_line, line);
                        line += 1;
                        state = St::Code;
                    } else if ch == '\\' && i + 1 < n {
                        i += 1;
                    } else if ch == '"' {
                        emit_string(&mut result, &mut buf, str_line, line);
                        state = St::Code;
                    } else {
                        buf.push(ch);
                    }
                }
                St::SQ => {
                    if ch == '\n' {
                        emit_string(&mut result, &mut buf, str_line, line);
                        line += 1;
                        state = St::Code;
                    } else if ch == '\\' && i + 1 < n {
                        i += 1;
                    } else if ch == '\'' {
                        emit_string(&mut result, &mut buf, str_line, line);
                        state = St::Code;
                    } else {
                        buf.push(ch);
                    }
                }
                St::TripleDQ => {
                    if ch == '\n' {
                        line += 1;
                        buf.push(ch);
                    } else if ch == '"' && peek_n(i, 1) == '"' && peek_n(i, 2) == '"' {
                        emit_string(&mut result, &mut buf, str_line, line);
                        state = St::Code;
                        i += 2;
                    } else {
                        buf.push(ch);
                    }
                }
                St::TripleSQ => {
                    if ch == '\n' {
                        line += 1;
                        buf.push(ch);
                    } else if ch == '\'' && peek_n(i, 1) == '\'' && peek_n(i, 2) == '\'' {
                        emit_string(&mut result, &mut buf, str_line, line);
                        state = St::Code;
                        i += 2;
                    } else {
                        buf.push(ch);
                    }
                }
            }
            i += 1;
        }

        // Handle case where the file ends without a newline.
        if state == St::Line {
            push_line_comment(&mut result, &buf, comment_line, line);
        } else if state == St::DQ
            || state == St::SQ
            || state == St::TripleDQ
            || state == St::TripleSQ
        {
            emit_string(&mut result, &mut buf, str_line, line);
        }

        mark_pep723_blocks(&mut result);
        Ok(result)
    }
}

fn push_line_comment(result: &mut Vec<Comment>, buf: &str, comment_line: i64, line: i64) {
    let text = buf.trim().to_string();
    // Shebang (#!) line is not treated as a comment.
    if !text.is_empty() && (comment_line != 1 || !text.starts_with('!')) {
        result.push(Comment {
            text,
            line: comment_line,
            end_line: line,
            is_block: false,
            kind: Kind::Comment,
        });
    }
}

fn emit_string(result: &mut Vec<Comment>, buf: &mut String, str_line: i64, end_line: i64) {
    let val = std::mem::take(buf);
    if !val.is_empty() {
        result.push(Comment {
            text: val.trim().to_string(),
            line: str_line,
            end_line,
            is_block: false,
            kind: Kind::String,
        });
    }
}

/// Marks PEP 723 inline script metadata blocks as KindImport. Corresponds to Go `markPEP723Blocks`.
fn mark_pep723_blocks(comments: &mut [Comment]) {
    let mut in_block = false;
    for c in comments.iter_mut() {
        if c.kind != Kind::Comment || c.is_block {
            continue;
        }
        if !in_block {
            if is_pep723_open_tag(&c.text) {
                c.kind = Kind::Import;
                in_block = true;
            }
        } else {
            c.kind = Kind::Import;
            if c.text == "///" {
                in_block = false;
            }
        }
    }
}

/// Returns true if the text is a PEP 723 block open tag (`/// <type>`). Corresponds to Go `isPEP723OpenTag`.
fn is_pep723_open_tag(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("/// ") else {
        return false;
    };
    let typ = rest.trim();
    if typ.is_empty() {
        return false;
    }
    typ.chars()
        .all(|ch| ch.is_alphanumeric() || ch == '-' || ch == '.')
}
