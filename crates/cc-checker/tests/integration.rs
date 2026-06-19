//! cc-checker integration tests: diff checks against a real staged git repository.
//! Ported from Go `internal/checker/{append_only,protected,cache_dir,binary,custom,diff_integration}_test.go`.
//!
//! Checker functions run git from cwd, so chdir is required → serialized via a global Mutex.
//! The gitdiff content cache and spec are reset for each test.
//! lint/encoding parser message wording may differ from Go — only detection is verified.

use std::path::Path;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use cc_config::{apply_defaults, Config, CustomRule};
use cc_gitdiff as gd;
use tempfile::TempDir;

static CWD_LOCK: Mutex<()> = Mutex::new(());

fn git(dir: &Path, gitconfig: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", gitconfig)
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "commit.gpgsign")
        .env("GIT_CONFIG_VALUE_0", "false")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

struct Repo<'a> {
    dir: &'a Path,
    gitconfig: std::path::PathBuf,
}

impl Repo<'_> {
    fn write(&self, rel: &str, content: &[u8]) {
        let full = self.dir.join(rel);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }
    fn add(&self, rel: &str) {
        git(self.dir, &self.gitconfig, &["add", rel]);
    }
    fn commit(&self, msg: &str) {
        git(self.dir, &self.gitconfig, &["commit", "-m", msg]);
    }
    fn rm(&self, rel: &str) {
        git(self.dir, &self.gitconfig, &["rm", rel]);
    }
}

fn with_repo<F: FnOnce(&Repo)>(f: F) {
    let _guard = CWD_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let gitconfig = tmp.path().join("empty.gitconfig");
    std::fs::write(&gitconfig, "# empty\n").unwrap();
    let repo = Repo {
        dir: tmp.path(),
        gitconfig: gitconfig.clone(),
    };

    git(repo.dir, &gitconfig, &["init"]);
    git(repo.dir, &gitconfig, &["config", "user.email", "t@t.com"]);
    git(repo.dir, &gitconfig, &["config", "user.name", "Test"]);
    repo.write("README.md", b"init\n");
    repo.add("README.md");
    repo.commit("chore: init");

    let orig = std::env::current_dir().unwrap();
    std::env::set_current_dir(repo.dir).unwrap();
    std::env::set_var("GIT_CONFIG_GLOBAL", &gitconfig);
    gd::reset_staged_content_cache();
    gd::set_spec(gd::Spec::default());

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&repo)));

    std::env::set_current_dir(&orig).unwrap();
    gd::reset_staged_content_cache();
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

fn base_cfg() -> Config {
    let mut cfg = Config::default();
    apply_defaults(&mut cfg);
    cfg
}

fn no_cancel() -> AtomicBool {
    AtomicBool::new(false)
}

// ── append-only ──

#[test]
fn append_only_new_file_allowed() {
    with_repo(|r| {
        r.write("migrations/001_init.sql", b"CREATE TABLE a (id INT);\n");
        r.add("migrations/001_init.sql");
        r.commit("feat: 001");
        // Adding a new file with a higher number → allowed.
        r.write("migrations/002_next.sql", b"CREATE TABLE b (id INT);\n");
        r.add("migrations/002_next.sql");

        let mut cfg = base_cfg();
        cfg.append_only.enabled = true;
        cfg.append_only.paths = vec!["migrations/**".into()];
        let diffs = gd::get_staged_diff().unwrap();
        let errs = cc_checker::check_append_only(&cfg, &diffs, &no_cancel()).unwrap();
        assert!(errs.is_empty(), "expected no violation, got {errs:?}");
    });
}

#[test]
fn append_only_modify_existing_blocked() {
    with_repo(|r| {
        r.write("migrations/001_init.sql", b"CREATE TABLE a (id INT);\n");
        r.add("migrations/001_init.sql");
        r.commit("feat: 001");
        // Modifying existing content (changing the beginning) → blocked.
        r.write(
            "migrations/001_init.sql",
            b"CREATE TABLE changed (id INT);\n",
        );
        r.add("migrations/001_init.sql");

        let mut cfg = base_cfg();
        cfg.append_only.enabled = true;
        cfg.append_only.paths = vec!["migrations/**".into()];
        let diffs = gd::get_staged_diff().unwrap();
        let errs = cc_checker::check_append_only(&cfg, &diffs, &no_cancel()).unwrap();
        assert!(!errs.is_empty(), "expected modify violation");
    });
}

#[test]
fn append_only_append_at_end_allowed() {
    with_repo(|r| {
        r.write("migrations/001.sql", b"line1\n");
        r.add("migrations/001.sql");
        r.commit("feat: 001");
        // Appending only at the end → allowed.
        r.write("migrations/001.sql", b"line1\nline2\n");
        r.add("migrations/001.sql");

        let mut cfg = base_cfg();
        cfg.append_only.enabled = true;
        cfg.append_only.paths = vec!["migrations/**".into()];
        let diffs = gd::get_staged_diff().unwrap();
        let errs = cc_checker::check_append_only(&cfg, &diffs, &no_cancel()).unwrap();
        assert!(errs.is_empty(), "expected append allowed, got {errs:?}");
    });
}

#[test]
fn append_only_delete_blocked() {
    with_repo(|r| {
        r.write("migrations/001.sql", b"x\n");
        r.add("migrations/001.sql");
        r.commit("feat: 001");
        r.rm("migrations/001.sql");

        let mut cfg = base_cfg();
        cfg.append_only.enabled = true;
        cfg.append_only.paths = vec!["migrations/**".into()];
        let diffs = gd::get_staged_diff().unwrap();
        let errs = cc_checker::check_append_only(&cfg, &diffs, &no_cancel()).unwrap();
        assert!(!errs.is_empty(), "expected delete violation");
    });
}

// ── protected paths ──

#[test]
fn protected_paths_modify_blocked() {
    with_repo(|r| {
        r.write("generated/api.go", b"package gen\n");
        r.add("generated/api.go");
        r.commit("feat: gen");
        r.write("generated/api.go", b"package gen\n// changed\n");
        r.add("generated/api.go");

        let mut cfg = base_cfg();
        cfg.protected_paths.enabled = true;
        cfg.protected_paths.paths = vec!["generated/**".into()];
        let diffs = gd::get_staged_diff().unwrap();
        let errs = cc_checker::check_protected_paths(&cfg, &diffs, &no_cancel()).unwrap();
        assert!(!errs.is_empty(), "expected protected path violation");
    });
}

#[test]
fn protected_paths_unmatched_allowed() {
    with_repo(|r| {
        r.write("src/app.go", b"package main\n");
        r.add("src/app.go");

        let mut cfg = base_cfg();
        cfg.protected_paths.enabled = true;
        cfg.protected_paths.paths = vec!["generated/**".into()];
        let diffs = gd::get_staged_diff().unwrap();
        let errs = cc_checker::check_protected_paths(&cfg, &diffs, &no_cancel()).unwrap();
        assert!(
            errs.is_empty(),
            "unmatched path should be allowed, got {errs:?}"
        );
    });
}

// ── binary ──

#[test]
fn binary_file_blocked_by_default() {
    with_repo(|r| {
        // Contains a NUL byte → git numstat identifies it as binary.
        r.write("blob.bin", &[0u8, 1, 2, 3, 0, 255, 0]);
        r.add("blob.bin");

        let mut cfg = base_cfg();
        cfg.binary_file.enabled = Some(true);
        cfg.binary_file.default_policy = "block".into();
        let errs = cc_checker::check_binary_files(&cfg, &no_cancel()).unwrap();
        assert!(!errs.is_empty(), "binary file should be blocked");
    });
}

#[test]
fn binary_file_image_allowed_by_builtin() {
    with_repo(|r| {
        // PNG signature + NUL → binary, but .png is allowed by built-in policy.
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        png.extend_from_slice(&[0u8, 1, 2, 3]);
        r.write("logo.png", &png);
        r.add("logo.png");

        let mut cfg = base_cfg();
        cfg.binary_file.enabled = Some(true);
        cfg.binary_file.default_policy = "block".into();
        let errs = cc_checker::check_binary_files(&cfg, &no_cancel()).unwrap();
        assert!(errs.is_empty(), "png should be allowed, got {errs:?}");
    });
}

// ── cache dir ──

#[test]
fn cache_dir_staged_blocked() {
    with_repo(|r| {
        r.write("package.json", b"{}\n");
        r.add("package.json");
        r.write("node_modules/lib/index.js", b"module.exports = 1;\n");
        r.add("node_modules/lib/index.js");

        let mut cfg = base_cfg();
        cfg.cache_dir.enabled = Some(true);
        let diffs = gd::get_staged_diff().unwrap();
        let errs = cc_checker::check_cache_dir_staged(&cfg, &diffs, &no_cancel()).unwrap();
        assert!(
            !errs.is_empty(),
            "node_modules staged file should be blocked"
        );
    });
}

// ── custom rules (diff) ──

#[test]
fn custom_rules_diff_forbidden() {
    with_repo(|r| {
        r.write("app.js", b"function f(){ console.log('x'); }\n");
        r.add("app.js");

        let mut cfg = base_cfg();
        cfg.custom_rules.diff = vec![CustomRule {
            name: "no_console".into(),
            pattern: r"console\.log".into(),
            message: "console.log not allowed".into(),
            required: false,
        }];
        let diffs = gd::get_staged_diff().unwrap();
        let errs = cc_checker::check_diff_custom_rules(&cfg, &diffs, &no_cancel()).unwrap();
        assert!(!errs.is_empty(), "console.log should be flagged");
    });
}

// ── comment language (diff) ──

#[test]
fn comment_language_diff_english_flagged_korean_ok() {
    with_repo(|r| {
        r.write(
            "main.go",
            b"package main\n\n// This is an English comment to flag\nfunc main() {}\n",
        );
        r.add("main.go");

        let mut cfg = base_cfg();
        cfg.comment_language.enabled = Some(true);
        cfg.comment_language.locale = "korean".into();
        let diffs = gd::get_staged_diff().unwrap();
        let errs = cc_checker::check_diff(&cfg, &diffs, &no_cancel()).unwrap();
        assert!(!errs.is_empty(), "english comment should be flagged");
    });

    with_repo(|r| {
        r.write(
            "main.go",
            "package main\n\n// 한국어 주석입니다 정상\nfunc main() {}\n".as_bytes(),
        );
        r.add("main.go");

        let mut cfg = base_cfg();
        cfg.comment_language.enabled = Some(true);
        cfg.comment_language.locale = "korean".into();
        let diffs = gd::get_staged_diff().unwrap();
        let errs = cc_checker::check_diff(&cfg, &diffs, &no_cancel()).unwrap();
        assert!(errs.is_empty(), "korean comment should pass, got {errs:?}");
    });
}

// ── encoding (diff): detection only ──

#[test]
fn encoding_non_utf8_flagged() {
    with_repo(|r| {
        // Latin-1 bytes (invalid UTF-8).
        r.write("latin.txt", &[0xC4, 0xD6, 0xDC, b'\n']);
        r.add("latin.txt");

        let mut cfg = base_cfg();
        cfg.encoding.enabled = Some(true);
        cfg.encoding.require_utf8 = Some(true);
        let errs = cc_checker::check_encoding(&cfg, &no_cancel()).unwrap();
        assert!(!errs.is_empty(), "non-utf8 file should be flagged");
    });
}
