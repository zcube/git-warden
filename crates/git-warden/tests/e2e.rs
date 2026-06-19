//! e2e tests: runs the binary against a temporary git repository. Ported from Go `cmd/e2e_test.go`.
//! Safe to run in parallel because assert_cmd sets current_dir on child processes, avoiding chdir races.
//! HOME/XDG_CONFIG_HOME/GIT_WARDEN_GLOBAL_CONFIG are isolated to prevent interference from the developer's global config.

use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

use assert_cmd::cargo::cargo_bin;
use tempfile::TempDir;

fn fixture_gitconfig() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/empty.gitconfig")
}

struct TestRepo {
    _dir: TempDir,
    _home: TempDir,
    path: PathBuf,
    home: PathBuf,
    xdg: PathBuf,
}

impl TestRepo {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let path = dir.path().to_path_buf();
        let home_path = home.path().to_path_buf();
        let xdg = home_path.join("xdg-config");
        let r = TestRepo {
            _dir: dir,
            _home: home,
            path,
            home: home_path,
            xdg,
        };
        r.git(&["init"]);
        r.git(&["config", "user.email", "test@git-warden.test"]);
        r.git(&["config", "user.name", "E2E Test"]);
        r
    }

    fn git(&self, args: &[&str]) -> String {
        let out = StdCommand::new("git")
            .args(args)
            .current_dir(&self.path)
            .env("GIT_CONFIG_GLOBAL", fixture_gitconfig())
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "commit.gpgsign")
            .env("GIT_CONFIG_VALUE_0", "false")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).to_string()
    }

    fn write(&self, rel: &str, content: &str) {
        let full = self.path.join(rel);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }

    fn write_bytes(&self, rel: &str, content: &[u8]) {
        let full = self.path.join(rel);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }

    fn stage(&self, rel: &str, content: &str) {
        self.write(rel, content);
        self.git(&["add", rel]);
    }

    fn commit(&self, msg: &str) {
        self.git(&["commit", "-m", msg]);
    }

    fn write_config(&self, content: &str) {
        self.write(".git-warden.yml", content);
    }

    fn write_global_config(&self, content: &str) {
        let p = self.xdg.join("git-warden").join("config.yml");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    /// Runs the binary from the repository root. Returns (combined output, exit code).
    fn run(&self, args: &[&str]) -> (String, i32) {
        let out = StdCommand::new(cargo_bin("git-warden"))
            .args(args)
            .current_dir(&self.path)
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", &self.xdg)
            .env("GIT_WARDEN_GLOBAL_CONFIG", "")
            .env("LANG", "C")
            .output()
            .unwrap();
        let mut combined = String::from_utf8_lossy(&out.stdout).to_string();
        combined.push_str(&String::from_utf8_lossy(&out.stderr));
        (combined, out.status.code().unwrap_or(-1))
    }

    fn run_lang(&self, lang: &str, args: &[&str]) -> (String, i32) {
        let out = StdCommand::new(cargo_bin("git-warden"))
            .args(args)
            .current_dir(&self.path)
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", &self.xdg)
            .env("GIT_WARDEN_GLOBAL_CONFIG", "")
            .env("LANG", lang)
            .output()
            .unwrap();
        let mut combined = String::from_utf8_lossy(&out.stdout).to_string();
        combined.push_str(&String::from_utf8_lossy(&out.stderr));
        (combined, out.status.code().unwrap_or(-1))
    }
}

// ── diff: comment language ──

#[test]
fn diff_korean_comment_exit0() {
    let r = TestRepo::new();
    r.stage(
        "main.go",
        "package main\n\n// 한국어 주석입니다\nfunc main() {}\n",
    );
    let (_, code) = r.run(&["diff"]);
    assert_eq!(code, 0);
}

#[test]
fn diff_english_comment_exit1() {
    let r = TestRepo::new();
    r.stage(
        "main.go",
        "package main\n\n// This comment is written in English only\nfunc main() {}\n",
    );
    let (out, code) = r.run(&["diff"]);
    assert_eq!(code, 1, "output: {out}");
}

#[test]
fn diff_no_staged_files_exit0() {
    let r = TestRepo::new();
    let (_, code) = r.run(&["diff"]);
    assert_eq!(code, 0);
}

#[test]
fn diff_unsupported_extension_exit0() {
    let r = TestRepo::new();
    r.stage(
        "notes.txt",
        "This is all English content in a plain text file.\n",
    );
    let (_, code) = r.run(&["diff"]);
    assert_eq!(code, 0);
}

#[test]
fn diff_disabled_exit0() {
    let r = TestRepo::new();
    r.write_config("comment_language:\n  enabled: false\n");
    r.stage(
        "main.go",
        "package main\n\n// This English comment should be ignored when disabled\nfunc main() {}\n",
    );
    let (_, code) = r.run(&["diff"]);
    assert_eq!(code, 0);
}

#[test]
fn diff_locale_ko_english_fails() {
    let r = TestRepo::new();
    r.write_config("comment_language:\n  locale: ko\n");
    r.stage(
        "main.go",
        "package main\n\n// English comment should fail with locale=ko\nfunc main() {}\n",
    );
    let (out, code) = r.run(&["diff"]);
    assert_eq!(code, 1, "output: {out}");
}

#[test]
fn diff_locale_en_english_passes() {
    let r = TestRepo::new();
    r.write_config("comment_language:\n  locale: en\n");
    r.stage(
        "main.go",
        "package main\n\n// This English comment should pass with locale=en\nfunc main() {}\n",
    );
    let (out, code) = r.run(&["diff"]);
    assert_eq!(code, 0, "output: {out}");
}

#[test]
fn diff_ignore_files() {
    let r = TestRepo::new();
    r.write_config("comment_language:\n  ignore_files:\n    - \"generated/**\"\n");
    r.stage(
        "generated/gen.go",
        "package gen\n\n// This English comment is in an ignored path\nfunc Gen() {}\n",
    );
    let (out, code) = r.run(&["diff"]);
    assert_eq!(code, 0, "output: {out}");
}

#[test]
fn diff_locale_ja_japanese_passes_korean_english_fail() {
    let r = TestRepo::new();
    r.write_config("comment_language:\n  locale: ja\n");
    r.stage(
        "a.go",
        "package main\n\n// これは日本語のコメントです\nfunc main() {}\n",
    );
    assert_eq!(r.run(&["diff"]).1, 0);

    let r2 = TestRepo::new();
    r2.write_config("comment_language:\n  locale: ja\n");
    r2.stage(
        "a.go",
        "package main\n\n// 이것은 한국어 주석입니다\nfunc main() {}\n",
    );
    assert_eq!(r2.run(&["diff"]).1, 1);
}

#[test]
fn diff_locale_zh_chinese_passes_korean_fails() {
    let r = TestRepo::new();
    r.write_config("comment_language:\n  locale: zh\n");
    r.stage(
        "a.go",
        "package main\n\n// 这是一个中文注释内容示例\nfunc main() {}\n",
    );
    assert_eq!(r.run(&["diff"]).1, 0);

    let r2 = TestRepo::new();
    r2.write_config("comment_language:\n  locale: zh\n");
    r2.stage(
        "a.go",
        "package main\n\n// 이것은 한국어 주석입니다\nfunc main() {}\n",
    );
    assert_eq!(r2.run(&["diff"]).1, 1);
}

#[test]
fn diff_output_contains_file_path() {
    let r = TestRepo::new();
    r.stage(
        "src/app.go",
        "package main\n\n// English comment that should be flagged here\nfunc main() {}\n",
    );
    let (out, code) = r.run(&["diff"]);
    assert_eq!(code, 1);
    assert!(out.contains("src/app.go"), "output: {out}");
}

// ── file_languages ──

#[test]
fn file_languages_any_allows_english() {
    let r = TestRepo::new();
    r.write_config("comment_language:\n  locale: ko\n  file_languages:\n    - pattern: \"locales/**\"\n      locale: any\n");
    r.stage(
        "locales/en.go",
        "package locales\n\n// English text allowed by any\nfunc X() {}\n",
    );
    assert_eq!(r.run(&["diff"]).1, 0);
}

#[test]
fn file_languages_english_override() {
    let r = TestRepo::new();
    r.write_config("comment_language:\n  locale: ko\n  file_languages:\n    - pattern: \"i18n/**\"\n      locale: english\n");
    r.stage(
        "i18n/en.go",
        "package i18n\n\n// This English comment passes via override\nfunc X() {}\n",
    );
    assert_eq!(r.run(&["diff"]).1, 0);
}

// ── directives ──

#[test]
fn directive_ignore() {
    let r = TestRepo::new();
    r.stage("main.go", "package main\n\n// git-warden:ignore\n// This English comment is ignored\nfunc main() {}\n");
    assert_eq!(r.run(&["diff"]).1, 0);
}

#[test]
fn directive_disable_enable() {
    let r = TestRepo::new();
    r.stage("main.go", "package main\n\n// git-warden:disable\n// English here is fine\n// git-warden:enable\nfunc main() {}\n");
    assert_eq!(r.run(&["diff"]).1, 0);
}

#[test]
fn directive_file_lang_english() {
    let r = TestRepo::new();
    r.stage("main.go", "package main\n\n// git-warden:file-lang=english\n// This whole file allows English\nfunc main() {}\n");
    assert_eq!(r.run(&["diff"]).1, 0);
}

// ── msg ──

#[test]
fn msg_clean_exit0() {
    let r = TestRepo::new();
    r.write("m.txt", "기능 추가\n");
    assert_eq!(r.run(&["msg", "m.txt"]).1, 0);
}

#[test]
fn msg_coauthor_ai_exit1() {
    let r = TestRepo::new();
    r.write(
        "m.txt",
        "기능 추가\n\nCo-authored-by: Claude <noreply@anthropic.com>\n",
    );
    let (out, code) = r.run(&["msg", "m.txt"]);
    // noreply@anthropic.com matches the built-in AI block pattern.
    assert_eq!(code, 1, "output: {out}");
}

#[test]
fn msg_coauthor_human_exit0() {
    let r = TestRepo::new();
    r.write(
        "m.txt",
        "기능 추가\n\nCo-authored-by: Jane <jane@example.com>\n",
    );
    assert_eq!(r.run(&["msg", "m.txt"]).1, 0);
}

#[test]
fn msg_invisible_char_exit1() {
    let r = TestRepo::new();
    r.write("m.txt", "기능\u{00A0}추가\n");
    assert_eq!(r.run(&["msg", "m.txt"]).1, 1);
}

#[test]
fn msg_bom_exit0() {
    let r = TestRepo::new();
    r.write_bytes(
        "m.txt",
        &[&[0xEF, 0xBB, 0xBF][..], "기능 추가\n".as_bytes()].concat(),
    );
    assert_eq!(r.run(&["msg", "m.txt"]).1, 0);
}

#[test]
fn msg_bad_rune_exit1() {
    let r = TestRepo::new();
    r.write_bytes("m.txt", &[b'f', b'i', b'x', b':', b' ', 0xFF, 0xFE, b'\n']);
    assert_eq!(r.run(&["msg", "m.txt"]).1, 1);
}

#[test]
fn msg_language_check_enabled_korean_fails() {
    let r = TestRepo::new();
    r.write_config("commit_message:\n  language_check:\n    enabled: true\n    locale: ko\n");
    r.write("m.txt", "This commit subject is English\n");
    let (out, code) = r.run(&["msg", "m.txt"]);
    assert_eq!(code, 1, "output: {out}");
}

#[test]
fn msg_fix_removes_ai_coauthor() {
    let r = TestRepo::new();
    r.write(
        "m.txt",
        "기능 추가\n\nCo-authored-by: Claude <noreply@anthropic.com>\n",
    );
    let (_, code) = r.run(&["msg", "--fix", "m.txt"]);
    assert_eq!(code, 0);
    let content = std::fs::read_to_string(r.path.join("m.txt")).unwrap();
    assert!(
        !content.to_lowercase().contains("co-authored-by"),
        "still has trailer: {content}"
    );
}

// ── fix ──

#[test]
fn fix_no_staged_files() {
    let r = TestRepo::new();
    let (out, code) = r.run(&["fix"]);
    assert_eq!(code, 0, "output: {out}");
}

#[test]
fn fix_dry_run_invisible_char() {
    let r = TestRepo::new();
    r.stage("init.go", "package main\n");
    r.commit("chore: 초기화");
    // no_invisible_chars must be enabled for fix to target NBSP (default: false).
    r.write_config("encoding:\n  enabled: true\n  no_invisible_chars: true\n");
    r.stage("hello.go", "package main\n// comment with\u{00A0}nbsp\n");
    let (out, code) = r.run(&["fix", "--dry-run"]);
    assert_eq!(code, 0, "output: {out}");
    assert!(out.contains("hello.go"), "output: {out}");
    // dry-run must not modify the file.
    assert!(std::fs::read_to_string(r.path.join("hello.go"))
        .unwrap()
        .contains('\u{00A0}'));
}

// ── global / project config ──

#[test]
fn diff_global_config_xdg_no_project_config() {
    let r = TestRepo::new();
    // Global config allows English → English comment passes.
    r.write_global_config("comment_language:\n  locale: en\n");
    r.stage(
        "main.go",
        "package main\n\n// English comment allowed by global config\nfunc main() {}\n",
    );
    let (out, code) = r.run(&["diff"]);
    assert_eq!(code, 0, "output: {out}");
}

#[test]
fn diff_project_config_overrides_global() {
    let r = TestRepo::new();
    r.write_global_config("comment_language:\n  locale: en\n");
    r.write_config("comment_language:\n  locale: ko\n");
    r.stage(
        "main.go",
        "package main\n\n// English comment should fail with project locale ko\nfunc main() {}\n",
    );
    let (out, code) = r.run(&["diff"]);
    assert_eq!(code, 1, "output: {out}");
}

#[test]
fn diff_enabled_false_exit0() {
    let r = TestRepo::new();
    r.write_config("enabled: false\ncomment_language:\n  locale: ko\n");
    r.stage(
        "main.go",
        "package main\n\n// English comment ignored due to enabled false\nfunc main() {}\n",
    );
    assert_eq!(r.run(&["diff"]).1, 0);
}

#[test]
fn diff_require_config_no_config_exit0() {
    let r = TestRepo::new();
    r.stage(
        "main.go",
        "package main\n\n// English comment but no config and require-config\nfunc main() {}\n",
    );
    let (_, code) = r.run(&["--require-config", "diff"]);
    assert_eq!(code, 0);
}

#[test]
fn diff_require_config_with_config_checks() {
    let r = TestRepo::new();
    r.write_config("comment_language:\n  locale: ko\n");
    r.stage(
        "main.go",
        "package main\n\n// English comment should fail since config exists\nfunc main() {}\n",
    );
    let (out, code) = r.run(&["--require-config", "diff"]);
    assert_eq!(code, 1, "output: {out}");
}

// ── other commands ──

#[test]
fn version_outputs() {
    let r = TestRepo::new();
    let (out, code) = r.run(&["version"]);
    assert_eq!(code, 0);
    assert!(out.contains("git-warden"), "output: {out}");
}

#[test]
fn init_creates_config() {
    let r = TestRepo::new();
    let (_, code) = r.run(&["init"]);
    assert_eq!(code, 0);
    assert!(r.path.join(".git-warden.yaml").exists());
    // Running init again without --force should fail.
    let (_, code2) = r.run(&["init"]);
    assert_eq!(code2, 1);
    // --force overwrites.
    assert_eq!(r.run(&["init", "--force"]).1, 0);
}

#[test]
fn analyze_runs() {
    let r = TestRepo::new();
    r.stage("main.go", "package main\nfunc main() {}\n");
    r.commit("init");
    let (out, code) = r.run(&["analyze"]);
    assert_eq!(code, 0);
    assert!(out.contains("Go"), "output: {out}");
}

#[test]
fn validate_clean_config() {
    let r = TestRepo::new();
    r.write_config("comment_language:\n  locale: ko\n");
    let (_, code) = r.run(&["validate"]);
    assert_eq!(code, 0);
}

#[test]
fn run_command_lint_error() {
    let r = TestRepo::new();
    r.stage("bad.json", "{\"a\": 1,}\n");
    r.commit("add bad json");
    let (out, code) = r.run(&["run", "--only", "lint"]);
    assert_eq!(code, 1, "output: {out}");
    assert!(out.contains("bad.json"), "output: {out}");
}

#[test]
fn run_format_json() {
    let r = TestRepo::new();
    r.stage("bad.json", "{\"a\": 1,}\n");
    r.commit("add bad json");
    let (out, code) = r.run(&["run", "--only", "lint", "--format", "json"]);
    assert_eq!(code, 1);
    assert!(out.contains("\"status\": \"fail\""), "output: {out}");
    assert!(out.contains("\"check\": \"lint\""), "output: {out}");
}

#[test]
fn only_invalid_category_errors() {
    let r = TestRepo::new();
    let (out, code) = r.run(&["run", "--only", "nonexistent"]);
    assert_eq!(code, 1, "output: {out}");
}

#[test]
fn prepare_msg_adds_hint() {
    let r = TestRepo::new();
    r.write_config("commit_message:\n  conventional_commit:\n    enabled: true\n");
    r.write("COMMIT_EDITMSG", "");
    let (_, code) = r.run(&["prepare-msg", "COMMIT_EDITMSG"]);
    assert_eq!(code, 0);
    let content = std::fs::read_to_string(r.path.join("COMMIT_EDITMSG")).unwrap();
    assert!(content.contains('#'), "hint not added: {content}");
}

#[test]
fn migrate_already_current() {
    let r = TestRepo::new();
    r.write_config("comment_language:\n  locale: ko\n  allowed_words:\n    - API\n");
    let (out, code) = r.run(&["migrate"]);
    assert_eq!(code, 0, "output: {out}");
}

// ── locale-specific message output (Korean) ──

#[test]
fn diff_korean_locale_message() {
    let r = TestRepo::new();
    r.write_config("comment_language:\n  locale: ko\n");
    r.stage(
        "main.go",
        "package main\n\n// English comment for korean message check\nfunc main() {}\n",
    );
    let (out, code) = r.run_lang("ko_KR.UTF-8", &["diff"]);
    assert_eq!(code, 1);
    // Korean violation message must be present.
    assert!(out.contains("작성되어야 합니다"), "output: {out}");
}
