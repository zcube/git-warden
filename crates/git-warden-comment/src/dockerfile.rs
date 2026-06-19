//! Extracts `#` line comments from Dockerfiles (no string literals). Corresponds to Go `dockerfile_parser.go`.

use crate::{Comment, Kind, Parser};

pub struct DockerfileParser;

impl Parser for DockerfileParser {
    fn supported_extensions(&self) -> Vec<&'static str> {
        // "dockerfile" is a special filename-pattern identifier, not an extension.
        vec!["dockerfile"]
    }

    fn parse_file(&self, content: &str) -> Result<Vec<Comment>, String> {
        let runes: Vec<char> = content.chars().collect();
        let mut result: Vec<Comment> = Vec::new();
        let mut line = 1i64;
        let mut buf = String::new();
        let mut in_comment = false;
        let mut comment_line = 0i64;

        for &ch in &runes {
            if in_comment {
                if ch == '\n' {
                    push_comment(&mut result, &buf, comment_line, line);
                    buf.clear();
                    in_comment = false;
                    line += 1;
                } else {
                    buf.push(ch);
                }
                continue;
            }
            match ch {
                '\n' => line += 1,
                '#' => {
                    in_comment = true;
                    comment_line = line;
                    buf.clear();
                }
                _ => {}
            }
        }

        if in_comment {
            push_comment(&mut result, &buf, comment_line, line);
        }

        Ok(result)
    }
}

fn push_comment(result: &mut Vec<Comment>, buf: &str, comment_line: i64, line: i64) {
    let text = buf.trim().to_string();
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
