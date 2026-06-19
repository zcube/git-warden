//! cc-gitdiff: Execute git diff and parse unified diff output. Corresponds to Go `internal/gitdiff`.

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, RwLock};

/// Special ref value pointing to the working tree. Corresponds to Go `RefWorktree`.
pub const REF_WORKTREE: &str = "worktree";

/// Specifies the git diff comparison target. Corresponds to Go `Spec`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Spec {
    pub from: String,
    pub to: String,
}

impl Spec {
    /// Returns true if this is the default staged mode (HEAD ↔ index).
    pub fn is_default(&self) -> bool {
        self.from.is_empty() && self.to.is_empty()
    }

    /// Returns true if the comparison target is the working tree.
    pub fn is_worktree(&self) -> bool {
        self.to == REF_WORKTREE || self.to == "working-tree" || self.to == "wt"
    }
}

// Package-level global spec. Set once from the cmd entry point via set_spec.
static CURRENT_SPEC: Lazy<RwLock<Spec>> = Lazy::new(|| RwLock::new(Spec::default()));

/// Sets the current spec (called once from the cmd entry point). Corresponds to Go `SetSpec`.
pub fn set_spec(s: Spec) {
    *CURRENT_SPEC.write().unwrap() = s;
}

/// Returns the currently configured spec. Corresponds to Go `CurrentSpec`.
pub fn current_spec() -> Spec {
    CURRENT_SPEC.read().unwrap().clone()
}

/// Splits an "A..B" or "A...B" range into (from, to). Returns None if the input is not a range.
/// "A...B" uses merge-base(A,B) as `from`. Corresponds to Go `ParseRange`.
pub fn parse_range(s: &str) -> Option<(String, String)> {
    if let Some(i) = s.find("...") {
        let left = &s[..i];
        let right = &s[i + 3..];
        let base = match merge_base(left, right) {
            Ok(b) if !b.is_empty() => b,
            // On merge-base failure, use left as-is (best-effort).
            _ => left.to_string(),
        };
        return Some((base, right.to_string()));
    }
    if let Some(i) = s.find("..") {
        return Some((s[..i].to_string(), s[i + 2..].to_string()));
    }
    None
}

/// Finds the common ancestor of two refs via git merge-base. Corresponds to Go `mergeBase`.
fn merge_base(a: &str, b: &str) -> Result<String, String> {
    let out = Command::new("git")
        .args(["merge-base", a, b])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("git merge-base failed".to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Constructs a Spec from git-diff-compatible positional arguments. Corresponds to Go `SpecFromArgs`.
pub fn spec_from_args(args: &[String], staged: bool) -> Result<Spec, String> {
    if staged {
        return match args.len() {
            0 => Ok(Spec::default()),
            1 => Ok(Spec {
                from: args[0].clone(),
                to: String::new(),
            }),
            _ => Err("--staged accepts 0 or 1 argument".to_string()),
        };
    }
    match args.len() {
        0 => Ok(Spec::default()),
        1 => {
            if let Some((from, to)) = parse_range(&args[0]) {
                Ok(Spec { from, to })
            } else {
                Ok(Spec {
                    from: args[0].clone(),
                    to: REF_WORKTREE.to_string(),
                })
            }
        }
        2 => Ok(Spec {
            from: args[0].clone(),
            to: args[1].clone(),
        }),
        _ => Err("too many arguments (maximum 2)".to_string()),
    }
}

/// Builds the git diff argument list for the given spec. Corresponds to Go `buildDiffArgs`.
fn build_diff_args(s: &Spec) -> Vec<String> {
    let mut args = vec!["diff".to_string()];
    if s.is_default() {
        args.push("--staged".to_string());
    } else if s.is_worktree() {
        if !s.from.is_empty() {
            args.push(s.from.clone());
        }
    } else {
        let from = if s.from.is_empty() {
            "HEAD"
        } else {
            s.from.as_str()
        };
        let to = if s.to.is_empty() {
            "HEAD"
        } else {
            s.to.as_str()
        };
        args.push(from.to_string());
        args.push(to.to_string());
    }
    args
}

/// File information from a staged diff. Corresponds to Go `FileDiff`.
#[derive(Debug, Clone, Default)]
pub struct FileDiff {
    pub path: String,
    /// Set of added line numbers in the new file (1-based).
    pub added_lines: std::collections::HashSet<i64>,
    pub is_deleted: bool,
    pub is_new: bool,
    pub has_removed_lines: bool,
    pub is_submodule: bool,
    pub is_symlink: bool,
}

/// Runs git diff based on current_spec and returns the parsed result. Corresponds to Go `GetStagedDiff`.
pub fn get_staged_diff() -> Result<Vec<FileDiff>, String> {
    let args = build_diff_args(&current_spec());
    // quotepath=false: prevent non-ASCII paths from being C-style quoted.
    let mut full = vec!["-c".to_string(), "core.quotepath=false".to_string()];
    full.extend(args.iter().cloned());
    let out = Command::new("git")
        .args(&full)
        .output()
        .map_err(|e| format!("git {} failed: {e}", args.join(" ")))?;
    // git diff exit code 1 simply means differences were found (not an error).
    if !out.status.success() && out.stdout.is_empty() {
        return Err(format!("git {} failed", args.join(" ")));
    }
    Ok(parse_diff(&String::from_utf8_lossy(&out.stdout)))
}

/// Splits git's -z output (NUL-delimited) into a list of paths. Corresponds to Go `SplitNullSeparated`.
pub fn split_null_separated(out: &[u8]) -> Vec<String> {
    let s = String::from_utf8_lossy(out);
    let raw = s.trim_end_matches('\u{0}');
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split('\u{0}').map(|p| p.to_string()).collect()
}

// Staged file content cache. Key: "cwd\0ref\0path".
static CONTENT_CACHE: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn content_cache_key(reference: &str, file_path: &str) -> String {
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    format!("{cwd}\u{0}{reference}\u{0}{file_path}")
}

/// Returns the file content at the current_spec.to revision. Corresponds to Go `GetStagedContent`.
pub fn get_staged_content(file_path: &str) -> Result<String, String> {
    get_content_at(&current_spec().to, file_path)
}

/// Returns the file content at a given ref. Corresponds to Go `GetContentAt`.
pub fn get_content_at(reference: &str, file_path: &str) -> Result<String, String> {
    let key = content_cache_key(reference, file_path);
    if let Some(v) = CONTENT_CACHE.lock().unwrap().get(&key) {
        return Ok(v.clone());
    }
    let result = match reference {
        "" | "index" | "staged" => {
            let out = Command::new("git")
                .args(["show", &format!(":{file_path}")])
                .output()
                .map_err(|e| format!("git show :{file_path}: {e}"))?;
            if !out.status.success() {
                return Err(format!("git show :{file_path}"));
            }
            String::from_utf8_lossy(&out.stdout).to_string()
        }
        REF_WORKTREE | "wt" | "working-tree" => std::fs::read(file_path)
            .map(|b| String::from_utf8_lossy(&b).to_string())
            .map_err(|e| format!("read {file_path}: {e}"))?,
        _ => {
            let out = Command::new("git")
                .args(["show", &format!("{reference}:{file_path}")])
                .output()
                .map_err(|e| format!("git show {reference}:{file_path}: {e}"))?;
            if !out.status.success() {
                return Err(format!("git show {reference}:{file_path}"));
            }
            String::from_utf8_lossy(&out.stdout).to_string()
        }
    };
    CONTENT_CACHE.lock().unwrap().insert(key, result.clone());
    Ok(result)
}

/// Clears the content cache (for tests). Corresponds to Go `ResetStagedContentCache`.
pub fn reset_staged_content_cache() {
    CONTENT_CACHE.lock().unwrap().clear();
}

/// Returns true if path has one of the given extensions. Corresponds to Go `HasExtension`.
/// "dockerfile" is a special identifier matching Dockerfile, Dockerfile.*, and *.dockerfile.
pub fn has_extension(path: &str, extensions: &[String]) -> bool {
    let base = Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let lower = base.to_lowercase();
    // filepath.Ext: from the last '.' to the end (empty string if none).
    let ext = match base.rfind('.') {
        Some(i) => &base[i..],
        None => "",
    };
    for e in extensions {
        if e == "dockerfile" {
            if lower == "dockerfile"
                || lower.starts_with("dockerfile.")
                || lower.ends_with(".dockerfile")
            {
                return true;
            }
            continue;
        }
        if ext.eq_ignore_ascii_case(e) {
            return true;
        }
    }
    false
}

/// Parses unified diff output and returns a list of FileDiff entries. Corresponds to Go `ParseDiff`.
pub fn parse_diff(diff: &str) -> Vec<FileDiff> {
    let mut result: Vec<FileDiff> = Vec::new();
    let mut current: Option<FileDiff> = None;
    let mut current_new_line: i64 = 0;

    for line in diff.split('\n') {
        // bufio.Scanner does not yield the final empty line, but empty lines have no
        // semantic impact in unified diff, so processing them here is harmless.
        if let Some(rest) = line.strip_prefix("diff --git ") {
            if let Some(cur) = current.take() {
                result.push(cur);
            }
            let mut fd = FileDiff::default();
            current_new_line = 0;
            fd.path = parse_diff_git_new_path_rest(rest);
            current = Some(fd);
            continue;
        }
        let Some(cur) = current.as_mut() else {
            continue;
        };

        if line.starts_with("+++ b/") || line.starts_with("+++ \"b/") {
            let after = line.strip_prefix("+++ ").unwrap();
            let unq = unquote_git_path(after);
            cur.path = unq.strip_prefix("b/").unwrap_or(&unq).to_string();
        } else if line == "+++ /dev/null" {
            cur.is_deleted = true;
        } else if line.starts_with("new file mode ")
            || line.starts_with("old mode ")
            || line.starts_with("new mode ")
        {
            if line.starts_with("new file mode ") {
                cur.is_new = true;
            }
            for prefix in ["new file mode ", "new mode ", "old mode "] {
                if let Some(rest) = line.strip_prefix(prefix) {
                    match rest.trim() {
                        "160000" => cur.is_submodule = true,
                        "120000" => cur.is_symlink = true,
                        _ => {}
                    }
                    break;
                }
            }
        } else if line.starts_with("--- ")
            || line.starts_with("index ")
            || line.starts_with("new file")
            || line.starts_with("deleted file")
            || line.starts_with("rename from")
            || line.starts_with("rename to")
            || line.starts_with("Binary ")
        {
            // metadata line, skip
        } else if line.starts_with("@@") {
            current_new_line = parse_hunk_header(line);
        } else if line.starts_with('+') {
            cur.added_lines.insert(current_new_line);
            current_new_line += 1;
        } else if line.starts_with('-') {
            cur.has_removed_lines = true;
        } else if line.starts_with(' ') {
            current_new_line += 1;
        }
    }

    if let Some(cur) = current.take() {
        result.push(cur);
    }
    result
}

/// Restores a git C-style quoted path to its original form. Corresponds to Go `unquoteGitPath` (using strconv.Unquote).
/// Returns the input unchanged if it is not quoted or unquoting fails.
pub fn unquote_git_path(p: &str) -> String {
    if p.len() < 2 || !p.starts_with('"') || !p.ends_with('"') {
        return p.to_string();
    }
    match unquote_c_style(&p[1..p.len() - 1]) {
        Some(s) => s,
        None => p.to_string(),
    }
}

/// Unescapes C-style escapes inside a double-quoted string byte-by-byte and decodes to UTF-8.
/// git quotes high-byte/control characters with octal (`\ooo`) and special characters with `\"`, `\\`, `\t`, etc.
fn unquote_c_style(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b != b'\\' {
            out.push(b);
            i += 1;
            continue;
        }
        // Escape sequence
        i += 1;
        if i >= bytes.len() {
            return None;
        }
        let c = bytes[i];
        match c {
            b'a' => {
                out.push(0x07);
                i += 1;
            }
            b'b' => {
                out.push(0x08);
                i += 1;
            }
            b'f' => {
                out.push(0x0c);
                i += 1;
            }
            b'n' => {
                out.push(b'\n');
                i += 1;
            }
            b'r' => {
                out.push(b'\r');
                i += 1;
            }
            b't' => {
                out.push(b'\t');
                i += 1;
            }
            b'v' => {
                out.push(0x0b);
                i += 1;
            }
            b'\\' => {
                out.push(b'\\');
                i += 1;
            }
            b'"' => {
                out.push(b'"');
                i += 1;
            }
            b'\'' => {
                out.push(b'\'');
                i += 1;
            }
            b'x' => {
                // \xHH
                if i + 2 >= bytes.len() {
                    return None;
                }
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
                let v = u8::from_str_radix(hex, 16).ok()?;
                out.push(v);
                i += 3;
            }
            b'0'..=b'7' => {
                // \ooo (exactly 1–3 octal digits; git uses 3)
                let mut j = i;
                let mut digits = 0;
                let mut val: u32 = 0;
                while j < bytes.len() && digits < 3 && (b'0'..=b'7').contains(&bytes[j]) {
                    val = val * 8 + (bytes[j] - b'0') as u32;
                    j += 1;
                    digits += 1;
                }
                if val > 0xff {
                    return None;
                }
                out.push(val as u8);
                i = j;
            }
            _ => return None,
        }
    }
    Some(String::from_utf8_lossy(&out).to_string())
}

/// Extracts the new path (b/ side) from the `diff --git ...` header (with prefix stripped).
/// Corresponds to Go `parseDiffGitNewPath`.
fn parse_diff_git_new_path_rest(rest: &str) -> String {
    // Quoted new path: starting from the ` "b/` sequence.
    if let Some(i) = rest.find(" \"b/") {
        let quoted = &rest[i + 1..];
        let p = unquote_git_path(quoted);
        if p != quoted {
            return p.strip_prefix("b/").unwrap_or(&p).to_string();
        }
    }
    // Unquoted new path: split on ` b/`.
    if let Some(i) = rest.find(" b/") {
        return rest[i + 3..].to_string();
    }
    String::new()
}

/// Parses the new-file start line number from `@@ -old[,count] +new[,count] @@`.
/// Corresponds to Go `parseHunkHeader`.
fn parse_hunk_header(line: &str) -> i64 {
    let Some(idx) = line.find('+') else {
        return 0;
    };
    let rest = &line[idx + 1..];
    let end = rest.find([',', ' ', '@', '\t']).unwrap_or(rest.len());
    rest[..end].parse::<i64>().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_is_default() {
        assert!(Spec::default().is_default());
        assert!(!Spec {
            from: "HEAD".into(),
            to: "".into()
        }
        .is_default());
        assert!(!Spec {
            from: "".into(),
            to: "worktree".into()
        }
        .is_default());
    }

    #[test]
    fn spec_is_worktree() {
        for (to, want) in [
            ("worktree", true),
            ("working-tree", true),
            ("wt", true),
            ("", false),
            ("HEAD", false),
            ("main", false),
        ] {
            let s = Spec {
                from: "".into(),
                to: to.into(),
            };
            assert_eq!(s.is_worktree(), want, "to={to}");
        }
    }

    #[test]
    fn build_diff_args_cases() {
        assert_eq!(build_diff_args(&Spec::default()), vec!["diff", "--staged"]);
        assert_eq!(
            build_diff_args(&Spec {
                from: "HEAD".into(),
                to: "worktree".into()
            }),
            vec!["diff", "HEAD"]
        );
        assert_eq!(
            build_diff_args(&Spec {
                from: "".into(),
                to: "worktree".into()
            }),
            vec!["diff"]
        );
        assert_eq!(
            build_diff_args(&Spec {
                from: "origin/main".into(),
                to: "".into()
            }),
            vec!["diff", "origin/main", "HEAD"]
        );
        assert_eq!(
            build_diff_args(&Spec {
                from: "A".into(),
                to: "B".into()
            }),
            vec!["diff", "A", "B"]
        );
    }

    #[test]
    fn parse_range_cases() {
        assert_eq!(
            parse_range("origin/main..HEAD"),
            Some(("origin/main".into(), "HEAD".into()))
        );
        assert_eq!(parse_range("HEAD"), None);
        assert_eq!(parse_range(""), None);
    }

    #[test]
    fn spec_from_args_cases() {
        assert!(spec_from_args(&[], false).unwrap().is_default());
        let s = spec_from_args(&["HEAD".into()], false).unwrap();
        assert_eq!(s.from, "HEAD");
        assert_eq!(s.to, REF_WORKTREE);
        let s = spec_from_args(&["main..feature".into()], false).unwrap();
        assert_eq!((s.from.as_str(), s.to.as_str()), ("main", "feature"));
        let s = spec_from_args(&["A".into(), "B".into()], false).unwrap();
        assert_eq!((s.from.as_str(), s.to.as_str()), ("A", "B"));
        assert!(spec_from_args(&[], true).unwrap().is_default());
        let s = spec_from_args(&["origin/main".into()], true).unwrap();
        assert_eq!(s.from, "origin/main");
        assert_eq!(s.to, "");
        assert!(spec_from_args(&["A".into(), "B".into()], true).is_err());
        assert!(spec_from_args(&["A".into(), "B".into(), "C".into()], false).is_err());
    }

    const SAMPLE_DIFF: &str = concat!(
        "diff --git a/foo.go b/foo.go\n",
        "index abc1234..def5678 100644\n",
        "--- a/foo.go\n",
        "+++ b/foo.go\n",
        "@@ -1,4 +1,6 @@\n",
        " package main\n",
        " \n",
        "+// 새로 추가된 주석\n",
        "+\n",
        " func old() {}\n",
        "+func newFunc() {}\n",
        "\n",
        "diff --git a/bar.go b/bar.go\n",
        "new file mode 100644\n",
        "index 0000000..aabbcc1\n",
        "--- /dev/null\n",
        "+++ b/bar.go\n",
        "@@ -0,0 +1,3 @@\n",
        "+package main\n",
        " \n",
        "+// 새 파일의 주석\n",
    );

    #[test]
    fn parse_diff_added_lines() {
        let diffs = parse_diff(SAMPLE_DIFF);
        assert_eq!(diffs.len(), 2);
        let foo = &diffs[0];
        assert_eq!(foo.path, "foo.go");
        for w in [3, 4, 6] {
            assert!(foo.added_lines.contains(&w), "line {w} should be added");
        }
        for c in [1, 2, 5] {
            assert!(!foo.added_lines.contains(&c), "line {c} should be context");
        }
        let bar = &diffs[1];
        assert_eq!(bar.path, "bar.go");
        assert!(bar.added_lines.contains(&1) && bar.added_lines.contains(&3));
    }

    #[test]
    fn parse_diff_deleted_file() {
        let diff = concat!(
            "diff --git a/old.go b/old.go\n",
            "deleted file mode 100644\n",
            "--- a/old.go\n",
            "+++ /dev/null\n",
            "@@ -1,3 +0,0 @@\n",
            "-package main\n",
            " \n",
            "-// removed\n",
        );
        let diffs = parse_diff(diff);
        assert_eq!(diffs.len(), 1);
        assert!(diffs[0].is_deleted);
    }

    #[test]
    fn parse_diff_submodule_and_symlink() {
        let sub = concat!(
            "diff --git a/vendor/mod b/vendor/mod\n",
            "new file mode 160000\n",
            "index 0000000..abc1234\n",
            "--- /dev/null\n",
            "+++ b/vendor/mod\n",
            "@@ -0,0 +1 @@\n",
            "+Subproject commit abc1234\n",
        );
        assert!(parse_diff(sub)[0].is_submodule);
        let link = concat!(
            "diff --git a/link b/link\n",
            "new file mode 120000\n",
            "index 0000000..abc1234\n",
            "--- /dev/null\n",
            "+++ b/link\n",
            "@@ -0,0 +1 @@\n",
            "+target/path\n",
        );
        assert!(parse_diff(link)[0].is_symlink);
    }

    #[test]
    fn parse_diff_quoted_path_with_quote() {
        let raw = concat!(
            "diff --git \"a/with\\\"quote.txt\" \"b/with\\\"quote.txt\"\n",
            "new file mode 100644\n",
            "index 0000000..abc1234\n",
            "--- /dev/null\n",
            "+++ \"b/with\\\"quote.txt\"\n",
            "@@ -0,0 +1 @@\n",
            "+x\n",
        );
        let diffs = parse_diff(raw);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "with\"quote.txt");
    }

    #[test]
    fn parse_diff_quoted_octal_korean() {
        let raw = concat!(
            "diff --git \"a/\\355\\225\\234\\352\\270\\200.md\" \"b/\\355\\225\\234\\352\\270\\200.md\"\n",
            "index abc1234..def5678 100644\n",
            "--- \"a/\\355\\225\\234\\352\\270\\200.md\"\n",
            "+++ \"b/\\355\\225\\234\\352\\270\\200.md\"\n",
            "@@ -1 +1,2 @@\n",
            " # 제목\n",
            "+본문\n",
        );
        let diffs = parse_diff(raw);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "한글.md");
        assert!(diffs[0].added_lines.contains(&2));
    }

    #[test]
    fn parse_diff_quoted_deleted_via_header() {
        let raw = concat!(
            "diff --git \"a/del\\\"eted.txt\" \"b/del\\\"eted.txt\"\n",
            "deleted file mode 100644\n",
            "index abc1234..0000000\n",
            "--- \"a/del\\\"eted.txt\"\n",
            "+++ /dev/null\n",
            "@@ -1 +0,0 @@\n",
            "-x\n",
        );
        let diffs = parse_diff(raw);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "del\"eted.txt");
        assert!(diffs[0].is_deleted);
    }

    #[test]
    fn has_extension_cases() {
        let exts: Vec<String> = vec![".go".into(), ".ts".into(), ".java".into()];
        for (p, want) in [
            ("main.go", true),
            ("src/app.ts", true),
            ("Service.java", true),
            ("readme.md", false),
            ("Makefile", false),
        ] {
            assert_eq!(has_extension(p, &exts), want, "path={p}");
        }
    }

    #[test]
    fn has_extension_dockerfile() {
        let exts: Vec<String> = vec!["dockerfile".into()];
        assert!(has_extension("Dockerfile", &exts));
        assert!(has_extension("Dockerfile.dev", &exts));
        assert!(has_extension("base.dockerfile", &exts));
        assert!(!has_extension("docker-compose.yml", &exts));
    }

    #[test]
    fn append_only_fields() {
        let new_file = concat!(
            "diff --git a/migrations/001.sql b/migrations/001.sql\n",
            "new file mode 100644\n",
            "index 0000000..abc1234\n",
            "--- /dev/null\n",
            "+++ b/migrations/001.sql\n",
            "@@ -0,0 +1,3 @@\n",
            "+CREATE TABLE users (\n",
            "+  id SERIAL PRIMARY KEY\n",
            "+);\n",
        );
        let f = &parse_diff(new_file)[0];
        assert!(f.is_new && !f.has_removed_lines);

        let modif = concat!(
            "diff --git a/migrations/001.sql b/migrations/001.sql\n",
            "index abc1234..def5678 100644\n",
            "--- a/migrations/001.sql\n",
            "+++ b/migrations/001.sql\n",
            "@@ -1,3 +1,3 @@\n",
            " CREATE TABLE users (\n",
            "-  id INT PRIMARY KEY\n",
            "+  id SERIAL PRIMARY KEY\n",
            " );\n",
        );
        assert!(parse_diff(modif)[0].has_removed_lines);
    }

    #[test]
    fn split_null_separated_cases() {
        assert_eq!(split_null_separated(b""), Vec::<String>::new());
        assert_eq!(
            split_null_separated(b"a\x00b\x00c\x00"),
            vec!["a", "b", "c"]
        );
        assert_eq!(split_null_separated(b"only"), vec!["only"]);
    }
}
