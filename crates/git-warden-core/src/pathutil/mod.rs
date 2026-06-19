//! cc-pathutil: glob path matching. Corresponds to Go `internal/pathutil`.
//!
//! Faithfully reproduces Go's `filepath.Match` (Unix semantics: `*` does not cross `/`,
//! `?`, `[...]`, `\` escape) and layers `**` doublestar matching on top.

const SEP: char = '/';

/// Returns true if `path` matches any of the given glob patterns. Corresponds to Go `MatchesAny`.
/// OR of three strategies: full-path match, base-name match, and doublestar glob match.
pub fn matches_any(path: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        // Full-path match
        if fnmatch(pattern, path) {
            return true;
        }
        // Base-name match
        if fnmatch(pattern, base(path)) {
            return true;
        }
        // Doublestar glob match
        if match_double_star_glob(path, pattern) {
            return true;
        }
    }
    false
}

/// Returns true if `path` matches the glob pattern under doublestar ("**") semantics. Corresponds to Go `MatchPath`.
/// Unlike `matches_any`, compares the full path segment-by-segment without base-name-only matching.
pub fn match_path(path: &str, pattern: &str) -> bool {
    match_double_star_glob(path, pattern)
}

/// Splits a pattern containing `**` on `/` and matches path segments sequentially.
fn match_double_star_glob(path: &str, pattern: &str) -> bool {
    let path_parts = split_path(path);
    let pat_parts = split_path(pattern);
    match_parts(&path_parts, &pat_parts)
}

/// Splits a path on `/` and removes empty segments. Corresponds to Go `splitPath`.
fn split_path(p: &str) -> Vec<&str> {
    p.split('/').filter(|s| !s.is_empty()).collect()
}

fn match_parts(path_parts: &[&str], pat_parts: &[&str]) -> bool {
    if pat_parts.is_empty() {
        return path_parts.is_empty();
    }
    if pat_parts[0] == "**" {
        // ** can match zero or more path segments
        for i in 0..=path_parts.len() {
            if match_parts(&path_parts[i..], &pat_parts[1..]) {
                return true;
            }
        }
        return false;
    }
    if path_parts.is_empty() {
        return false;
    }
    if !fnmatch(pat_parts[0], path_parts[0]) {
        return false;
    }
    match_parts(&path_parts[1..], &pat_parts[1..])
}

/// Last segment (base name) of a path. Simplified reproduction of Go's `filepath.Base` behaviour.
/// For this use case, returning the last non-empty segment (or the original path if none) is sufficient.
fn base(path: &str) -> &str {
    if path.is_empty() {
        return ".";
    }
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return "/";
    }
    match trimmed.rfind('/') {
        Some(idx) => &trimmed[idx + 1..],
        None => trimmed,
    }
}

// ---------------------------------------------------------------------------
// Faithful port of Go's path/filepath.Match (Unix: Separator = '/').
// Returns true only when the pattern matches the entire name. Invalid patterns return false
// (Go callers ignore the error).
// ---------------------------------------------------------------------------

/// Equivalent to Go `filepath.Match(pattern, name)`. Errors are absorbed as false.
pub fn fnmatch(pattern: &str, name: &str) -> bool {
    let pat: Vec<char> = pattern.chars().collect();
    let nm: Vec<char> = name.chars().collect();
    let mut p: &[char] = &pat;
    let mut n: &[char] = &nm;

    'pattern: while !p.is_empty() {
        let (star, chunk, rest) = scan_chunk(p);
        p = rest;
        if star && chunk.is_empty() {
            // A trailing * matches the rest only when name contains no '/'
            return !n.contains(&SEP);
        }
        // Try matching at the current position
        match match_chunk(chunk, n) {
            Ok((t, true)) if t.is_empty() || !p.is_empty() => {
                n = t;
                continue;
            }
            Err(()) => return false,
            _ => {}
        }
        if star {
            // Try matching by skipping i+1 characters (never cross '/')
            let mut i = 0;
            while i < n.len() && n[i] != SEP {
                match match_chunk(chunk, &n[i + 1..]) {
                    Ok((t, true)) => {
                        if p.is_empty() && !t.is_empty() {
                            i += 1;
                            continue;
                        }
                        n = t;
                        continue 'pattern;
                    }
                    Err(()) => return false,
                    _ => {}
                }
                i += 1;
            }
        }
        return false;
    }
    n.is_empty()
}

/// Splits out the next chunk (the part containing no asterisks). Consumes leading asterisks and
/// returns the chunk up to the next asterisk and the remainder as `rest`. Corresponds to Go `scanChunk`.
fn scan_chunk(pattern: &[char]) -> (bool, &[char], &[char]) {
    let mut p = pattern;
    let mut star = false;
    while !p.is_empty() && p[0] == '*' {
        p = &p[1..];
        star = true;
    }
    let mut in_range = false;
    let mut i = 0;
    while i < p.len() {
        match p[i] {
            '\\' => {
                // Escape: protect the next character (Unix)
                if i + 1 < p.len() {
                    i += 1;
                }
            }
            '[' => in_range = true,
            ']' => in_range = false,
            '*' if !in_range => {
                break;
            }
            _ => {}
        }
        i += 1;
    }
    (star, &p[..i], &p[i..])
}

/// Matches `chunk` against the beginning of `s` and returns the remainder. Corresponds to Go `matchChunk`.
/// Returns Ok((rest, matched)) or Err(()) for an invalid pattern.
fn match_chunk<'a>(chunk: &[char], mut s: &'a [char]) -> Result<(&'a [char], bool), ()> {
    let mut failed = false;
    let mut c = chunk;
    while !c.is_empty() {
        if !failed && s.is_empty() {
            failed = true;
        }
        match c[0] {
            '[' => {
                // Character class
                let mut r = '\0';
                if !failed {
                    r = s[0];
                    s = &s[1..];
                }
                c = &c[1..];
                // Optional negation '^'
                let mut negated = false;
                if !c.is_empty() && c[0] == '^' {
                    negated = true;
                    c = &c[1..];
                }
                // Range parsing
                let mut matched = false;
                let mut nrange = 0;
                loop {
                    if !c.is_empty() && c[0] == ']' && nrange > 0 {
                        c = &c[1..];
                        break;
                    }
                    let (lo, nc) = get_esc(c)?;
                    c = nc;
                    let mut hi = lo;
                    if !c.is_empty() && c[0] == '-' {
                        let (h, nc2) = get_esc(&c[1..])?;
                        hi = h;
                        c = nc2;
                    }
                    if lo <= r && r <= hi {
                        matched = true;
                    }
                    nrange += 1;
                }
                if matched == negated {
                    failed = true;
                }
            }
            '?' => {
                if !failed {
                    if s[0] == SEP {
                        failed = true;
                    } else {
                        s = &s[1..];
                    }
                }
                c = &c[1..];
            }
            '\\' => {
                c = &c[1..];
                if c.is_empty() {
                    return Err(());
                }
                if !failed {
                    if c[0] != s[0] {
                        failed = true;
                    } else {
                        s = &s[1..];
                    }
                }
                c = &c[1..];
            }
            other => {
                if !failed {
                    if other != s[0] {
                        failed = true;
                    } else {
                        s = &s[1..];
                    }
                }
                c = &c[1..];
            }
        }
    }
    if failed {
        return Ok((s, false));
    }
    Ok((s, true))
}

/// Reads one (possibly escaped) character from inside a character class and returns the remainder. Corresponds to Go `getEsc`.
fn get_esc(chunk: &[char]) -> Result<(char, &[char]), ()> {
    if chunk.is_empty() || chunk[0] == '-' || chunk[0] == ']' {
        return Err(());
    }
    if chunk[0] == '\\' {
        let rest = &chunk[1..];
        if rest.is_empty() {
            return Err(());
        }
        return Ok((rest[0], &rest[1..]));
    }
    Ok((chunk[0], &chunk[1..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pats(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    // Ported from Go TestMatchesAny
    #[test]
    fn matches_any_cases() {
        let cases: &[(&str, &[&str], bool)] = &[
            ("main.go", &["main.go"], true),
            ("foo.pb.go", &["*.pb.go"], true),
            ("vendor/github.com/pkg/foo.go", &["vendor/**"], true),
            ("internal/generated/foo.go", &["**/generated/**"], true),
            ("internal/checker/diff.go", &["vendor/**", "*.pb.go"], false),
            ("any/path.go", &[], false),
            ("deep/dir/generated.go", &["generated.go"], true),
        ];
        for (path, p, want) in cases {
            assert_eq!(
                matches_any(path, &pats(p)),
                *want,
                "matches_any({path:?}, {p:?})"
            );
        }
    }

    // Ported from Go TestMatchPath
    #[test]
    fn match_path_cases() {
        let cases: &[(&str, &str, bool)] = &[
            ("/home/user/work/repo", "/home/user/work/**", true),
            ("/home/user/work", "/home/user/work/**", true),
            ("/home/user/work/team/sub/repo", "/home/user/work/**", true),
            ("/home/user/personal/repo", "/home/user/work/**", false),
            ("/home/user/deep/repo", "repo", false),
            ("/home/user/work", "/home/user/work", true),
        ];
        for (path, pat, want) in cases {
            assert_eq!(
                match_path(path, pat),
                *want,
                "match_path({path:?}, {pat:?})"
            );
        }
    }

    // Additional filepath.Match boundary behaviour
    #[test]
    fn fnmatch_star_does_not_cross_slash() {
        assert!(fnmatch("*.go", "main.go"));
        assert!(!fnmatch("*.go", "dir/main.go")); // * does not cross '/'
        assert!(fnmatch("a/*.go", "a/main.go"));
        assert!(!fnmatch("*", "a/b"));
        assert!(fnmatch("*", "abc"));
    }

    #[test]
    fn fnmatch_question_and_class() {
        assert!(fnmatch("?.go", "a.go"));
        assert!(!fnmatch("?.go", "/.go")); // ? does not match '/'
        assert!(fnmatch("[abc].go", "b.go"));
        assert!(!fnmatch("[abc].go", "d.go"));
        assert!(fnmatch("[a-z].go", "m.go"));
        assert!(fnmatch("[^0-9].go", "a.go"));
        assert!(!fnmatch("[^0-9].go", "5.go"));
    }

    #[test]
    fn fnmatch_escape() {
        assert!(fnmatch("\\*.go", "*.go"));
        assert!(!fnmatch("\\*.go", "a.go"));
    }

    #[test]
    fn doublestar_boundaries() {
        assert!(match_path("a/b/c", "a/**"));
        assert!(match_path("a", "a/**")); // ** matches zero segments
        assert!(match_path("a/b/c/d", "**/d"));
        assert!(match_path("x/generated/y/z", "**/generated/**"));
        assert!(!match_path("a/b", "a/c/**"));
        assert!(match_path("a/foo.go", "a/*.go"));
        assert!(!match_path("a/b/foo.go", "a/*.go")); // * within a segment matches only one segment
    }

    #[test]
    fn base_name() {
        assert_eq!(base("a/b/c.go"), "c.go");
        assert_eq!(base("c.go"), "c.go");
        assert_eq!(base("a/b/"), "b");
    }

    #[test]
    fn malformed_pattern_is_false() {
        // Unclosed class etc. returns false (error absorbed)
        assert!(!fnmatch("[a-", "a"));
    }
}
