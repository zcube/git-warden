//! Extracts HTML comments (`<!-- -->`) and link-reference comments (`[//]: # (...)`) from Markdown. Corresponds to Go `markdown_parser.go`.
//!
//! The original Go implementation identifies HTML blocks via the goldmark AST and parses
//! link-reference comments with a regex. This Rust port reproduces equivalent behaviour with a
//! line-based scanner that tracks code fences: heading/paragraph/code-block content is not
//! extracted; only block-level HTML comments and `[//]: #` comments are extracted.

use crate::{Comment, Kind, Parser};

pub struct MarkdownParser;

impl Parser for MarkdownParser {
    fn supported_extensions(&self) -> Vec<&'static str> {
        vec![".md", ".markdown"]
    }

    fn parse_file(&self, content: &str) -> Result<Vec<Comment>, String> {
        let lines: Vec<&str> = content.split('\n').collect();
        let mut result: Vec<Comment> = Vec::new();
        let mut in_fence = false;
        let mut i = 0usize;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            // Toggle code fence (``` or ~~~).
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                in_fence = !in_fence;
                i += 1;
                continue;
            }

            if !in_fence {
                if trimmed.starts_with("<!--") {
                    // Block-level HTML comment: collect until the line containing -->.
                    let start_line = (i + 1) as i64;
                    let mut raw = String::new();
                    let mut j = i;
                    loop {
                        raw.push_str(lines[j]);
                        if lines[j].contains("-->") {
                            break;
                        }
                        raw.push('\n');
                        j += 1;
                        if j >= lines.len() {
                            break;
                        }
                    }
                    let end_line = (j + 1) as i64;
                    if let Some(text) = extract_html_comment(&raw) {
                        if !text.is_empty() {
                            result.push(Comment {
                                text,
                                line: start_line,
                                end_line,
                                is_block: true,
                                kind: Kind::Comment,
                            });
                        }
                    }
                    i = j + 1;
                    continue;
                }
                if let Some(text) = parse_link_comment(trimmed) {
                    result.push(Comment {
                        text,
                        line: (i + 1) as i64,
                        end_line: (i + 1) as i64,
                        is_block: false,
                        kind: Kind::Comment,
                    });
                }
            }
            i += 1;
        }

        result.sort_by_key(|c| c.line);
        Ok(result)
    }
}

/// Extracts the comment body from `<!-- ... -->`. Corresponds to Go `mdExtractHTMLComment`.
fn extract_html_comment(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if !raw.starts_with("<!--") {
        return None;
    }
    let end = raw.find("-->")?;
    if end < 4 {
        return None;
    }
    Some(raw[4..end].trim().to_string())
}

/// Parses a Markdown `[//]: # (text)` / `"text"` / `'text'` comment. Corresponds to Go `mdLinkCommentRe`.
fn parse_link_comment(s: &str) -> Option<String> {
    let rest = s.strip_prefix("[//]: # ")?;
    let mut chars = rest.chars();
    let open = chars.next()?;
    let (close, body) = match open {
        '(' => (')', &rest[1..]),
        '"' => ('"', &rest[1..]),
        '\'' => ('\'', &rest[1..]),
        _ => return None,
    };
    let end = body.find(close)?;
    let inner = body[..end].trim().to_string();
    if inner.is_empty() {
        None
    } else {
        Some(inner)
    }
}
