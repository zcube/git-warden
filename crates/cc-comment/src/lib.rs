//! cc-comment: Extracts comments and string literals from source code. Corresponds to Go `internal/comment`.

mod cstyle;
mod dockerfile;
mod go;
mod hash;
mod hcl;
mod html;
mod markdown;
mod python;
mod registry;

pub use cstyle::CStyleParser;
pub use dockerfile::DockerfileParser;
pub use go::GoParser;
pub use hash::HashStyleParser;
pub use hcl::HclParser;
pub use html::HtmlParser;
pub use markdown::MarkdownParser;
pub use python::PythonParser;
pub use registry::{extensions_for_languages, get_parser, is_dockerfile_path};

/// Kind of extracted item. Corresponds to Go `Kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A comment.
    Comment,
    /// A string literal.
    String,
    /// An import/include path — always excluded from language checks.
    Import,
}

/// A comment or string literal extracted from source code. Corresponds to Go `Comment`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub text: String,
    pub line: i64,
    pub end_line: i64,
    pub is_block: bool,
    pub kind: Kind,
}

/// Parser that extracts comments from source code. Corresponds to Go `Parser` interface.
pub trait Parser {
    fn parse_file(&self, content: &str) -> Result<Vec<Comment>, String>;
    fn supported_extensions(&self) -> Vec<&'static str>;
}

/// Strips leading asterisks and whitespace from each line of a block comment body (JavaDoc/JSDoc style). Corresponds to Go `cleanBlockComment`.
pub(crate) fn clean_block_comment(raw: &str) -> String {
    let mut cleaned: Vec<&str> = Vec::new();
    for line in raw.split('\n') {
        let mut l = line.trim();
        l = l.strip_prefix('*').unwrap_or(l);
        l = l.trim();
        if !l.is_empty() {
            cleaned.push(l);
        }
    }
    cleaned.join("\n")
}
