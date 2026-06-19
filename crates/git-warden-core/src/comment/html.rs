//! Extracts `<!-- -->` block comments from HTML/SVG. Corresponds to Go `html_parser.go`.

use super::{clean_block_comment, Comment, Kind, Parser};

pub struct HtmlParser;

impl Parser for HtmlParser {
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec![".html", ".htm", ".svg"]
    }

    fn parse_file(&self, content: &str) -> Result<Vec<Comment>, String> {
        let runes: Vec<char> = content.chars().collect();
        let n = runes.len();
        let mut result: Vec<Comment> = Vec::new();
        let mut line = 1i64;
        let mut i = 0;

        while i < n {
            let ch = runes[i];
            if ch == '\n' {
                line += 1;
                i += 1;
                continue;
            }
            // Detect <!-- start.
            if ch == '<'
                && i + 3 < n
                && runes[i + 1] == '!'
                && runes[i + 2] == '-'
                && runes[i + 3] == '-'
            {
                let comment_line = line;
                i += 4;
                let mut buf = String::new();
                while i < n {
                    if runes[i] == '-' && i + 2 < n && runes[i + 1] == '-' && runes[i + 2] == '>' {
                        let text = clean_block_comment(&buf);
                        result.push(Comment {
                            text,
                            line: comment_line,
                            end_line: line,
                            is_block: true,
                            kind: Kind::Comment,
                        });
                        i += 3;
                        break;
                    }
                    if runes[i] == '\n' {
                        line += 1;
                    }
                    buf.push(runes[i]);
                    i += 1;
                }
                continue;
            }
            i += 1;
        }

        Ok(result)
    }
}
