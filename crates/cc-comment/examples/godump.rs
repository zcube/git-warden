//! Diff-testing helper: parses Go source from stdin with GoParser and prints in the same format as the oracle (goracle).
use std::io::Read;

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace('\r', "\\r")
}

fn main() {
    let mut content = String::new();
    std::io::stdin().read_to_string(&mut content).unwrap();
    let p = cc_comment::GoParser;
    let comments = cc_comment::Parser::parse_file(&p, &content).unwrap();
    for c in &comments {
        let kind = match c.kind {
            cc_comment::Kind::Comment => "comment",
            cc_comment::Kind::String => "string",
            cc_comment::Kind::Import => "import",
        };
        println!(
            "{}|{}|{}|{}|{}",
            kind,
            c.line,
            c.end_line,
            c.is_block,
            esc(&c.text)
        );
    }
}
