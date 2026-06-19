//! Comment, string, and heredoc extraction for HCL (Terraform, etc.). Corresponds to Go `hcl_parser.go`.
//!
//! The original Go implementation uses the hashicorp/hcl v2 hclsyntax tokenizer; this Rust port
//! implements an equivalent state machine directly: `#`/`//` line comments, `/* */` block comments,
//! `"..."` strings (tracking interpolation `${}`/`%{}` depth, `$${`/`%%{` literal escapes),
//! and heredocs (`<<LABEL`/`<<-LABEL`).

use crate::{clean_block_comment, Comment, Kind, Parser};

pub struct HclParser;

impl Parser for HclParser {
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec![".hcl", ".tf", ".tfvars"]
    }

    fn parse_file(&self, content: &str) -> Result<Vec<Comment>, String> {
        let runes: Vec<char> = content.chars().collect();
        let n = runes.len();
        let mut s = Scanner {
            runes,
            n,
            i: 0,
            line: 1,
            result: Vec::new(),
        };
        s.run();
        Ok(s.result)
    }
}

struct Scanner {
    runes: Vec<char>,
    n: usize,
    i: usize,
    line: i64,
    result: Vec<Comment>,
}

impl Scanner {
    fn peek(&self, off: usize) -> char {
        if self.i + off < self.n {
            self.runes[self.i + off]
        } else {
            '\0'
        }
    }

    fn run(&mut self) {
        while self.i < self.n {
            let c = self.runes[self.i];
            if c == '\n' {
                self.line += 1;
                self.i += 1;
            } else if c == '#' {
                self.line_comment(1);
            } else if c == '/' && self.peek(1) == '/' {
                self.line_comment(2);
            } else if c == '/' && self.peek(1) == '*' {
                self.block_comment();
            } else if c == '"' {
                self.top_string();
            } else if c == '<' && self.peek(1) == '<' {
                if !self.heredoc() {
                    self.i += 1;
                }
            } else {
                self.i += 1;
            }
        }
    }

    // `#` (marker_len=1) or `//` (marker_len=2) line comment. Does not consume the newline.
    fn line_comment(&mut self, marker_len: usize) {
        let start_line = self.line;
        self.i += marker_len;
        let mut buf = String::new();
        while self.i < self.n && self.runes[self.i] != '\n' {
            buf.push(self.runes[self.i]);
            self.i += 1;
        }
        let text = buf.trim().to_string();
        if !text.is_empty() {
            self.result.push(Comment {
                text,
                line: start_line,
                end_line: start_line,
                is_block: false,
                kind: Kind::Comment,
            });
        }
    }

    fn block_comment(&mut self) {
        let start_line = self.line;
        self.i += 2; // /*
        let mut buf = String::new();
        while self.i < self.n {
            if self.runes[self.i] == '*' && self.peek(1) == '/' {
                self.i += 2;
                break;
            }
            if self.runes[self.i] == '\n' {
                self.line += 1;
            }
            buf.push(self.runes[self.i]);
            self.i += 1;
        }
        self.result.push(Comment {
            text: clean_block_comment(&buf),
            line: start_line,
            end_line: self.line,
            is_block: true,
            kind: Kind::Comment,
        });
    }

    // Top-level string: extract body and emit as KindString.
    fn top_string(&mut self) {
        let str_line = self.line;
        let (body_start, body_end, _) = self.scan_string();
        let raw: String = self.runes[body_start..body_end].iter().collect();
        let text = unescape_template_literal(&raw);
        if !text.is_empty() {
            self.result.push(Comment {
                text,
                line: str_line,
                end_line: self.line,
                is_block: false,
                kind: Kind::String,
            });
        }
    }

    // Assumes self.i points to the opening `"`. Advances to the closing quote (or newline)
    // and returns (body_start, body_end, newline_terminated). Skips interpolation/escapes.
    fn scan_string(&mut self) -> (usize, usize, bool) {
        self.i += 1; // consume opening quote
        let body_start = self.i;
        while self.i < self.n {
            let c = self.runes[self.i];
            if c == '\\' {
                self.i += 2; // escape: skip backslash + next char (kept as-is in body)
                continue;
            }
            if c == '\n' {
                // Malformed input: terminate gracefully (newline not consumed; main loop handles it).
                return (body_start, self.i, true);
            }
            if c == '"' {
                let end = self.i;
                self.i += 1;
                return (body_start, end, false);
            }
            if c == '$' || c == '%' {
                // $${ / %%{ literal escapes.
                if self.peek(1) == c && self.peek(2) == '{' {
                    self.i += 3;
                    continue;
                }
                if self.peek(1) == '{' {
                    self.skip_interpolation();
                    continue;
                }
                self.i += 1;
                continue;
            }
            self.i += 1;
        }
        (body_start, self.i, false)
    }

    // Assumes self.i points to the `$`/`%` of `${`/`%{`. Advances to the closing `}`.
    fn skip_interpolation(&mut self) {
        self.i += 2; // consume marker
        let mut depth = 1;
        while self.i < self.n {
            let c = self.runes[self.i];
            if c == '\n' {
                self.line += 1;
                self.i += 1;
            } else if c == '"' {
                self.scan_string();
            } else if c == '{' {
                depth += 1;
                self.i += 1;
            } else if c == '}' {
                depth -= 1;
                self.i += 1;
                if depth == 0 {
                    return;
                }
            } else if c == '\\' {
                self.i += 2;
            } else {
                self.i += 1;
            }
        }
    }

    // Processes a heredoc (`<<LABEL`/`<<-LABEL`). Returns true and updates self.i/self.line on success.
    fn heredoc(&mut self) -> bool {
        let heredoc_line = self.line;
        let mut j = self.i + 2; // consume <<
        if j < self.n && self.runes[j] == '-' {
            j += 1;
        }
        let label_start = j;
        while j < self.n && (self.runes[j].is_alphanumeric() || self.runes[j] == '_') {
            j += 1;
        }
        if j == label_start {
            return false; // no label → not a heredoc
        }
        let label: String = self.runes[label_start..j].iter().collect();
        // Skip the rest of the opening line (before the newline).
        while j < self.n && self.runes[j] != '\n' {
            j += 1;
        }
        if j >= self.n {
            return false; // no newline after label → not a heredoc
        }
        // Collect body lines.
        let mut body_lines: Vec<String> = Vec::new();
        let mut k = j + 1; // start of first body line
        let mut cur_line = heredoc_line + 1;
        loop {
            // Read one line.
            let mut line_end = k;
            while line_end < self.n && self.runes[line_end] != '\n' {
                line_end += 1;
            }
            let line_str: String = self.runes[k..line_end].iter().collect();
            if line_str.trim() == label {
                // Closing line.
                let body = body_lines.join("\n");
                self.emit_string(body, heredoc_line, cur_line);
                self.i = line_end; // closing line's newline (or EOF)
                self.line = cur_line;
                return true;
            }
            body_lines.push(line_str);
            if line_end >= self.n {
                // Reached EOF without closing → best-effort.
                let body = body_lines.join("\n");
                self.emit_string(body, heredoc_line, cur_line);
                self.i = self.n;
                self.line = cur_line;
                return true;
            }
            k = line_end + 1;
            cur_line += 1;
        }
    }

    fn emit_string(&mut self, text: String, line: i64, end_line: i64) {
        if !text.is_empty() {
            self.result.push(Comment {
                text,
                line,
                end_line,
                is_block: false,
                kind: Kind::String,
            });
        }
    }
}

/// Unescapes template literal escapes inside quoted strings (`$${` → `${`, `%%{` → `%{`).
fn unescape_template_literal(s: &str) -> String {
    s.replace("$${", "${").replace("%%{", "%{")
}
