//! cc-gitdiff integration tests: get_staged_diff/get_staged_content/get_content_at against a real git repository.
//! Ported from Go `internal/gitdiff/gitops_test.go`.
//!
//! The functions under test run git against the cwd, so chdir is required → serialised via a global Mutex.
//! GIT_CONFIG_GLOBAL is set to an empty config to block global hooks/settings on developer machines.

use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

use git_warden_gitdiff as gd;
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

/// Creates a temporary git repository and runs the closure inside it (serialised cwd + cache/spec reset).
fn with_repo<F: FnOnce(&Path)>(f: F) {
    let _guard = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    let gitconfig = dir.path().join("empty.gitconfig");
    std::fs::write(&gitconfig, "# empty\n").unwrap();

    git(dir.path(), &gitconfig, &["init"]);
    git(dir.path(), &gitconfig, &["config", "user.email", "t@t.com"]);
    git(dir.path(), &gitconfig, &["config", "user.name", "Test"]);
    std::fs::write(dir.path().join("README.md"), "init\n").unwrap();
    git(dir.path(), &gitconfig, &["add", "README.md"]);
    git(dir.path(), &gitconfig, &["commit", "-m", "chore: init"]);

    let orig = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    // Apply GIT_CONFIG_GLOBAL to the process environment so library-internal git calls inherit it.
    std::env::set_var("GIT_CONFIG_GLOBAL", &gitconfig);
    gd::reset_staged_content_cache();
    gd::set_spec(gd::Spec::default());

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(dir.path())));

    std::env::set_current_dir(&orig).unwrap();
    gd::reset_staged_content_cache();
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

fn stage_file(dir: &Path, name: &str, content: &str) {
    let full = dir.join(name);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(&full, content).unwrap();
    let out = Command::new("git")
        .args(["add", name])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git add: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn get_staged_diff_no_changes() {
    with_repo(|_dir| {
        let diffs = gd::get_staged_diff().unwrap();
        assert_eq!(diffs.len(), 0);
    });
}

#[test]
fn get_staged_diff_with_staged_file() {
    with_repo(|dir| {
        stage_file(
            dir,
            "main.go",
            "package main\n\n// Hello world\nfunc main() {}\n",
        );
        let diffs = gd::get_staged_diff().unwrap();
        assert!(
            diffs.iter().any(|d| d.path == "main.go"),
            "diffs: {diffs:?}"
        );
    });
}

#[test]
fn get_staged_content_existing_file() {
    with_repo(|dir| {
        let content = "package main\n\n// staged content\n";
        stage_file(dir, "staged.go", content);
        let got = gd::get_staged_content("staged.go").unwrap();
        assert_eq!(got, content);
    });
}

#[test]
fn get_staged_content_nonexistent_file() {
    with_repo(|_dir| {
        assert!(gd::get_staged_content("nonexistent_file_xyz.go").is_err());
    });
}

// ── ParseDiff edge cases (pure, no git required) ──

#[test]
fn parse_diff_hunk_header_no_plus() {
    let diff = "diff --git a/foo.go b/foo.go\n+++ b/foo.go\n@@ -1,3 @@\n+package main\n";
    assert_eq!(gd::parse_diff(diff).len(), 1);
}

#[test]
fn parse_diff_hunk_header_non_numeric() {
    let diff = "diff --git a/foo.go b/foo.go\n+++ b/foo.go\n@@ -1,3 +abc,3 @@\n+package main\n";
    assert_eq!(gd::parse_diff(diff).len(), 1);
}

#[test]
fn parse_diff_before_first_header() {
    let diff = "some preamble line\nanother line\ndiff --git a/foo.go b/foo.go\n+++ b/foo.go\n@@ -0,0 +1 @@\n+package main\n";
    assert_eq!(gd::parse_diff(diff).len(), 1);
}

#[test]
fn parse_diff_rename_metadata() {
    let diff = "diff --git a/old.go b/new.go\nrename from old.go\nrename to new.go\n+++ b/new.go\n@@ -1 +1 @@\n package main\n";
    let diffs = gd::parse_diff(diff);
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].path, "new.go");
}

#[test]
fn parse_diff_binary_file() {
    let diff = "diff --git a/img.png b/img.png\nindex abc..def 100644\nBinary files a/img.png and b/img.png differ\n";
    assert_eq!(gd::parse_diff(diff).len(), 1);
}
