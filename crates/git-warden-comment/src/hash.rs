//! Comment extraction for `#`-style languages (Ruby/Shell). Ruby supports =begin/=end blocks. Corresponds to Go `hash_parser.go`.

use crate::{Comment, Kind, Parser};

pub struct HashStyleParser {
    exts: Vec<&'static str>,
    ruby_blocks: bool,
}

impl HashStyleParser {
    pub fn new(exts: Vec<&'static str>, ruby_blocks: bool) -> Self {
        HashStyleParser { exts, ruby_blocks }
    }
}

impl Parser for HashStyleParser {
    fn supported_extensions(&self) -> Vec<&'static str> {
        self.exts.clone()
    }

    fn parse_file(&self, content: &str) -> Result<Vec<Comment>, String> {
        let mut result: Vec<Comment> = Vec::new();
        let lines: Vec<&str> = content.split('\n').collect();

        let mut in_block = false;
        let mut block_start = 0i64;
        let mut block_buf = String::new();

        for (line_num, raw_line) in lines.iter().enumerate() {
            let line_no = (line_num + 1) as i64;
            let trimmed = raw_line.trim();

            // =begin/=end block comments (Ruby only).
            if self.ruby_blocks {
                if !in_block && trimmed == "=begin" {
                    in_block = true;
                    block_start = line_no;
                    block_buf.clear();
                    continue;
                }
                if in_block {
                    if trimmed == "=end" {
                        result.push(Comment {
                            text: block_buf.trim().to_string(),
                            line: block_start,
                            end_line: line_no,
                            is_block: true,
                            kind: Kind::Comment,
                        });
                        in_block = false;
                        block_buf.clear();
                    } else {
                        if !block_buf.is_empty() {
                            block_buf.push('\n');
                        }
                        block_buf.push_str(raw_line);
                    }
                    continue;
                }
            }

            // `#` line comment (excluding shebang #! line).
            if trimmed.starts_with('#') && (line_no != 1 || !trimmed.starts_with("#!")) {
                let text = trimmed
                    .strip_prefix('#')
                    .unwrap_or(trimmed)
                    .trim()
                    .to_string();
                if !text.is_empty() {
                    result.push(Comment {
                        text,
                        line: line_no,
                        end_line: line_no,
                        is_block: false,
                        kind: Kind::Comment,
                    });
                }
            }
        }

        // Handle case where the file ends without a closing =end.
        if in_block && !block_buf.is_empty() {
            let text = block_buf.trim().to_string();
            if !text.is_empty() {
                result.push(Comment {
                    text,
                    line: block_start,
                    end_line: lines.len() as i64,
                    is_block: true,
                    kind: Kind::Comment,
                });
            }
        }

        Ok(result)
    }
}
