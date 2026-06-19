//! Comment and string extraction for C-style languages (TS/JS/Java/Kotlin/C/C++/C#/Swift/Rust/PHP). Corresponds to Go `cstyle_parser.go`.

use super::{clean_block_comment, Comment, Kind, Parser};

/// State-machine parser for C-style languages.
pub struct CStyleParser {
    extensions: Vec<&'static str>,
    has_template: bool,
}

impl CStyleParser {
    /// Creates a parser for the given extensions. Set has_template=true for JS/TS backtick templates.
    pub fn new(extensions: Vec<&'static str>, has_template: bool) -> Self {
        CStyleParser {
            extensions,
            has_template,
        }
    }
}

fn kind_for_import(is_import: bool) -> Kind {
    if is_import {
        Kind::Import
    } else {
        Kind::String
    }
}

#[derive(PartialEq)]
enum St {
    Code,
    Line,
    Block,
    DQ,
    SQ,
    Template,
}

impl Parser for CStyleParser {
    fn supported_extensions(&self) -> Vec<&'static str> {
        self.extensions.clone()
    }

    fn parse_file(&self, content: &str) -> Result<Vec<Comment>, String> {
        let runes: Vec<char> = content.chars().collect();
        let n = runes.len();
        let mut result: Vec<Comment> = Vec::new();
        let mut state = St::Code;
        let mut buf = String::new();
        let mut line_pre = String::new();
        let mut comment_line = 0i64;
        let mut str_line = 0i64;
        let mut line = 1i64;

        let peek = |i: usize| -> char {
            if i + 1 < n {
                runes[i + 1]
            } else {
                '\0'
            }
        };

        // Whether the current line prefix is an import/include context.
        let is_import_context = |line_pre: &str| -> bool {
            let pre = line_pre.trim();
            if pre.starts_with("#include") {
                return true;
            }
            if pre == "from" || pre.ends_with(" from") {
                return true;
            }
            if pre == "import" {
                return true;
            }
            false
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
                    } else if ch == '\'' {
                        state = St::SQ;
                        str_line = line;
                        buf.clear();
                    } else if self.has_template && ch == '`' {
                        state = St::Template;
                        str_line = line;
                        buf.clear();
                    } else {
                        line_pre.push(ch);
                    }
                }
                St::Line => {
                    if ch == '\n' {
                        let text = buf.trim().to_string();
                        if !text.is_empty() {
                            result.push(Comment {
                                text,
                                line: comment_line,
                                end_line: line,
                                is_block: false,
                                kind: Kind::Comment,
                            });
                        }
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
                        emit_string(
                            &mut result,
                            &mut buf,
                            str_line,
                            line,
                            kind_for_import(is_import_context(&line_pre)),
                        );
                        line += 1;
                        line_pre.clear();
                        state = St::Code;
                    } else if ch == '\\' && i + 1 < n {
                        i += 1;
                    } else if ch == '"' {
                        emit_string(
                            &mut result,
                            &mut buf,
                            str_line,
                            line,
                            kind_for_import(is_import_context(&line_pre)),
                        );
                        state = St::Code;
                    } else {
                        buf.push(ch);
                    }
                }
                St::SQ => {
                    if ch == '\n' {
                        emit_string(
                            &mut result,
                            &mut buf,
                            str_line,
                            line,
                            kind_for_import(is_import_context(&line_pre)),
                        );
                        line += 1;
                        line_pre.clear();
                        state = St::Code;
                    } else if ch == '\\' && i + 1 < n {
                        i += 1;
                    } else if ch == '\'' {
                        emit_string(
                            &mut result,
                            &mut buf,
                            str_line,
                            line,
                            kind_for_import(is_import_context(&line_pre)),
                        );
                        state = St::Code;
                    } else {
                        buf.push(ch);
                    }
                }
                St::Template => {
                    if ch == '\n' {
                        line += 1;
                        buf.push(ch);
                    } else if ch == '\\' && i + 1 < n {
                        i += 1;
                    } else if ch == '`' {
                        emit_string(&mut result, &mut buf, str_line, line, Kind::String);
                        state = St::Code;
                    } else {
                        buf.push(ch);
                    }
                }
            }
            i += 1;
        }

        // Handle case where the file ends without a newline.
        if state == St::Line {
            let text = buf.trim().to_string();
            if !text.is_empty() {
                result.push(Comment {
                    text,
                    line: comment_line,
                    end_line: line,
                    is_block: false,
                    kind: Kind::Comment,
                });
            }
        }
        if state == St::DQ || state == St::SQ || state == St::Template {
            emit_string(
                &mut result,
                &mut buf,
                str_line,
                line,
                kind_for_import(is_import_context(&line_pre)),
            );
        }

        Ok(result)
    }
}

fn emit_string(
    result: &mut Vec<Comment>,
    buf: &mut String,
    str_line: i64,
    end_line: i64,
    kind: Kind,
) {
    let val = std::mem::take(buf);
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
