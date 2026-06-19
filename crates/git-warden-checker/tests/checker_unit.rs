//! Pure unit test port of the Go checker (no external git repository required).
//! Corresponds to conventional_test.go / msg_test.go / fix_test.go / msg_language_test.go.
//!
//! Integration tests that require a git repository fixture (append-only tree comparison,
//! cache_dir, protected/binary/encoding staged, comment language diff, etc.) are ported
//! in `tests/integration.rs` using chdir serialization.

use git_warden_checker::{check_msg, fix_msg};
use git_warden_config::Config;

fn conventional_config() -> Config {
    let mut cfg = Config::default();
    cfg.commit_message.conventional_commit.enabled = Some(true);
    cfg
}

fn all_checks_config() -> Config {
    let mut cfg = Config::default();
    cfg.commit_message.no_ai_coauthor = Some(true);
    cfg.commit_message.no_unicode_spaces = Some(true);
    cfg.commit_message.no_ambiguous_chars = Some(true);
    cfg.commit_message.no_bad_runes = Some(true);
    cfg.commit_message.locale = "ko".to_string();
    cfg
}

fn check(cfg: &Config, msg: &str) -> Vec<String> {
    check_msg(cfg, msg.as_bytes())
}

// ── conventional commits ──

#[test]
fn conventional_valid_formats() {
    let cfg = conventional_config();
    for msg in [
        "feat: add new login page\n",
        "fix: correct null pointer in auth\n",
        "feat(auth): add OAuth2 support\n",
        "feat!: remove deprecated API\n",
        "feat(api)!: remove v1 endpoints\n",
        "feat: add something\n\nThis is the body.\n\nBREAKING CHANGE: old API removed\n",
    ] {
        assert!(check(&cfg, msg).is_empty(), "should be valid: {msg:?}");
    }
}

#[test]
fn conventional_all_default_types() {
    let cfg = conventional_config();
    for typ in git_warden_config::DEFAULT_CONVENTIONAL_TYPES {
        let msg = format!("{typ}: some description\n");
        assert!(check(&cfg, &msg).is_empty(), "type {typ} should be valid");
    }
}

#[test]
fn conventional_invalid_formats() {
    let cfg = conventional_config();
    for msg in [
        "feat add something\n",
        "feat: \n",
        "feat:add something\n",
        "just a plain commit message\n",
    ] {
        assert!(!check(&cfg, msg).is_empty(), "should be invalid: {msg:?}");
    }
}

#[test]
fn conventional_invalid_type_mentions_type() {
    let cfg = conventional_config();
    let errs = check(&cfg, "unknown: add something\n");
    assert!(!errs.is_empty());
    assert!(errs[0].contains("unknown"));
}

#[test]
fn conventional_skip_rules() {
    let cfg = conventional_config();
    for msg in [
        "Merge branch 'feature' into main\n",
        "Revert \"feat: add something\"\n",
        "fixup! feat: add something\n",
        "squash! feat: add something\n",
    ] {
        assert!(check(&cfg, msg).is_empty(), "should be skipped: {msg:?}");
    }
}

#[test]
fn conventional_disabled_by_default() {
    let cfg = Config::default();
    assert!(check(&cfg, "just a plain commit message\n").is_empty());
}

#[test]
fn conventional_custom_types() {
    let mut cfg = conventional_config();
    cfg.commit_message.conventional_commit.types = vec!["task".into(), "hotfix".into()];
    assert!(check(&cfg, "task: do something\n").is_empty());
    assert!(!check(&cfg, "feat: standard type should now fail\n").is_empty());
}

#[test]
fn conventional_require_scope() {
    let mut cfg = conventional_config();
    cfg.commit_message.conventional_commit.require_scope = Some(true);
    assert!(!check(&cfg, "feat: add something\n").is_empty());
    assert!(check(&cfg, "feat(auth): add something\n").is_empty());
}

#[test]
fn conventional_localized_types() {
    let mut ko = conventional_config();
    ko.commit_message.conventional_commit.locale = "ko".into();
    assert!(check(&ko, "기능: 새로운 로그인 페이지 추가\n").is_empty());
    assert!(check(&ko, "수정: 인증 널 포인터 수정\n").is_empty());
    assert!(check(&ko, "feat: add something\n").is_empty());
    assert!(check(&ko, "기능(인증): OAuth2 지원 추가\n").is_empty());

    let mut ja = conventional_config();
    ja.commit_message.conventional_commit.locale = "ja".into();
    assert!(check(&ja, "機能: 新しいログインページを追加\n").is_empty());

    let mut zh = conventional_config();
    zh.commit_message.conventional_commit.locale = "zh".into();
    assert!(check(&zh, "功能: 添加新的登录页面\n").is_empty());
}

#[test]
fn conventional_custom_type_aliases() {
    let mut cfg = conventional_config();
    cfg.commit_message
        .conventional_commit
        .type_aliases
        .insert("fonctionnalite".into(), "feat".into());
    cfg.commit_message
        .conventional_commit
        .type_aliases
        .insert("correction".into(), "fix".into());
    assert!(check(&cfg, "fonctionnalite: ajouter une page\n").is_empty());
    assert!(!check(&cfg, "unknown_alias: something\n").is_empty());
}

// ── co-author ──

#[test]
fn coauthor_ai_detected() {
    let cfg = all_checks_config();
    assert!(!check(
        &cfg,
        "feat: add new feature\n\nCo-authored-by: Claude Sonnet 4.6 <noreply@anthropic.com>\n"
    )
    .is_empty());
    assert!(!check(
        &cfg,
        "fix: bug\n\nCo-authored-by: GitHub Copilot <github-copilot[bot]@users.noreply.github.com>\n"
    )
    .is_empty());
    // Case-insensitive.
    assert!(!check(
        &cfg,
        "fix: bug\n\nco-authored-by: Claude <noreply@anthropic.com>\n"
    )
    .is_empty());
}

#[test]
fn coauthor_human_allowed() {
    let cfg = all_checks_config();
    let errs = check(
        &cfg,
        "feat: add feature\n\nCo-authored-by: Alice <alice@myteam.com>\n",
    );
    assert!(errs
        .iter()
        .all(|e| !e.contains("co-author") && !e.contains("AI")));
}

#[test]
fn coauthor_custom_remove() {
    let mut cfg = all_checks_config();
    cfg.commit_message.coauthor_remove_emails = vec!["*@myai.internal".into()];
    assert!(!check(
        &cfg,
        "feat: add\n\nCo-authored-by: InternalBot <bot@myai.internal>\n"
    )
    .is_empty());
}

#[test]
fn coauthor_disabled() {
    let mut cfg = all_checks_config();
    cfg.commit_message.no_ai_coauthor = Some(false);
    let errs = check(&cfg, "feat: ok\n\nCo-authored-by: Bot <bot@example.com>\n");
    assert!(errs.iter().all(|e| !e.contains("co-author")));
}

#[test]
fn msg_clean() {
    let cfg = all_checks_config();
    assert!(check(&cfg, "feat: normal commit message\n\nBody text here.\n").is_empty());
}

// ── invisible / ambiguous / bad rune ──

#[test]
fn invisible_chars_detected() {
    let cfg = all_checks_config();
    assert!(!check(&cfg, "feat: hello\u{00A0}world").is_empty()); // NBSP
    assert!(!check(&cfg, "feat: hello\u{200B}world").is_empty()); // ZWSP
    assert!(!check(&cfg, "feat: hello\u{202E}world").is_empty()); // BiDi RLO
}

#[test]
fn bom_allowed() {
    let cfg = all_checks_config();
    let errs = check(&cfg, "\u{FEFF}feat: starts with BOM");
    assert!(errs
        .iter()
        .all(|e| !e.contains("비가시") && !e.to_lowercase().contains("invisible")));
}

#[test]
fn ambiguous_chars_detected() {
    let cfg = all_checks_config();
    assert!(!check(&cfg, "feat: \u{0410}mbiguous").is_empty()); // Cyrillic А
    assert!(!check(&cfg, "feat: c\u{043E}mmit").is_empty()); // Cyrillic о
}

#[test]
fn korean_not_ambiguous() {
    let cfg = all_checks_config();
    assert!(check(&cfg, "feat: 새로운 기능 추가").is_empty());
}

#[test]
fn bad_rune_detected() {
    let cfg = all_checks_config();
    // Contains an invalid UTF-8 byte 0x80.
    let mut bytes = b"feat: bad".to_vec();
    bytes.push(0x80);
    bytes.extend_from_slice(b"rune");
    assert!(!check_msg(&cfg, &bytes).is_empty());
}

// ── fix ──

#[test]
fn fix_removes_ai_coauthor() {
    let cfg = all_checks_config();
    let msg =
        b"feat: add feature\n\nSome body.\n\nCo-authored-by: Claude <noreply@anthropic.com>\n";
    let r = fix_msg(&cfg, msg);
    assert!(r.needs_fixing());
    assert!(!r.fixed.contains("Co-authored-by"));
    assert!(!r.changes.is_empty());
}

#[test]
fn fix_keeps_human_coauthor() {
    let cfg = all_checks_config();
    let msg = b"feat: add feature\n\nCo-authored-by: Alice <alice@myteam.com>\n";
    let r = fix_msg(&cfg, msg);
    assert!(!r.needs_fixing());
    assert!(r.fixed.contains("Co-authored-by: Alice"));
}

#[test]
fn fix_replaces_ambiguous() {
    let cfg = all_checks_config();
    let r = fix_msg(&cfg, "feat: \u{0410}dd feature".as_bytes());
    assert!(r.needs_fixing());
    assert!(!r.fixed.contains('\u{0410}'));
    assert!(r.fixed.contains('A'));
}

#[test]
fn fix_replaces_invisible_space() {
    let cfg = all_checks_config();
    let r = fix_msg(&cfg, "feat: hello\u{00A0}world".as_bytes());
    assert!(r.needs_fixing());
    assert!(!r.fixed.contains('\u{00A0}'));
    assert!(r.fixed.contains("hello world"));
}

#[test]
fn fix_removes_bad_rune() {
    let cfg = all_checks_config();
    let mut bytes = b"feat: bad".to_vec();
    bytes.push(0x80);
    bytes.extend_from_slice(b"rune");
    let r = fix_msg(&cfg, &bytes);
    assert!(r.needs_fixing());
    assert!(!r.fixed.contains('\u{80}'));
    assert!(r.fixed.contains("badrune"));
}

#[test]
fn fix_clean_no_changes() {
    let cfg = all_checks_config();
    let msg = "feat: normal commit\n\nBody text here.\n";
    let r = fix_msg(&cfg, msg.as_bytes());
    assert!(!r.needs_fixing());
    assert_eq!(r.fixed, r.original);
}

#[test]
fn fix_multiple_issues() {
    let cfg = all_checks_config();
    let msg = "feat: \u{0410}dd\n\nCo-authored-by: Copilot <github-copilot[bot]@users.noreply.github.com>\n";
    let r = fix_msg(&cfg, msg.as_bytes());
    assert!(r.needs_fixing());
    assert!(r.changes.len() >= 2);
    assert!(!r.fixed.contains("Co-authored-by"));
    assert!(!r.fixed.contains('\u{0410}'));
}

#[test]
fn fix_disabled_checks() {
    let mut cfg = Config::default();
    cfg.commit_message.no_ai_coauthor = Some(false);
    cfg.commit_message.no_unicode_spaces = Some(false);
    cfg.commit_message.no_ambiguous_chars = Some(false);
    cfg.commit_message.no_bad_runes = Some(false);
    let r = fix_msg(
        &cfg,
        "feat: \u{0410}dd\n\nCo-authored-by: Bot <x@y.com>\n".as_bytes(),
    );
    assert!(!r.needs_fixing());
}

// ── language check ──

#[test]
fn language_check_korean_required() {
    let mut cfg = Config::default();
    cfg.commit_message.language_check.enabled = Some(true);
    cfg.commit_message.language_check.locale = "ko".into();
    cfg.commit_message.language_check.min_length = 5;
    // English body violates the Korean-required rule.
    assert!(!check(&cfg, "Add a new feature to the system").is_empty());
    // Korean body passes.
    assert!(check(&cfg, "시스템에 새로운 기능을 추가합니다").is_empty());
}

#[test]
fn language_check_korean_with_english_technical_terms() {
    let mut cfg = Config::default();
    cfg.commit_message.language_check.enabled = Some(true);
    cfg.commit_message.language_check.locale = "ko".into();
    cfg.commit_message.language_check.min_length = 5;

    // Conventional commit prefix (ci:) + Korean content + English package names in parens.
    // dominant_language() would wrongly classify this as English; presence-based check passes.
    assert!(
        check(&cfg, "ci: 릴리즈 워크플로 추가 (gitversion-rs + git-cliff)").is_empty(),
        "Korean subject with English technical terms must pass Korean check"
    );

    // Body lines mixing Korean and English identifiers must also pass.
    let body = "ci: 릴리즈 워크플로 추가\n\n\
                - cliff.toml: git-cliff Conventional Commits 체인지로그 설정\n\
                - release.yml: 버전 계산, 체인지로그 생성 워크플로\n";
    assert!(
        check(&cfg, body).is_empty(),
        "Korean body lines with English identifiers must pass Korean check"
    );

    // Pure English subject must still fail.
    assert!(
        !check(&cfg, "ci: add release workflow for binaries").is_empty(),
        "English-only subject must fail Korean check"
    );

    // Pure English body line must fail.
    assert!(
        !check(
            &cfg,
            "ci: 릴리즈\n\nadd the release workflow for binary builds\n"
        )
        .is_empty(),
        "English-only body line must fail Korean check"
    );
}

#[test]
fn language_check_english_required_strict() {
    let mut cfg = Config::default();
    cfg.commit_message.language_check.enabled = Some(true);
    cfg.commit_message.language_check.locale = "en".into();
    cfg.commit_message.language_check.min_length = 5;

    // Pure English passes.
    assert!(
        check(&cfg, "feat: add release workflow for binaries").is_empty(),
        "English-only subject must pass English check"
    );

    // English subject with Korean must fail (no mixing allowed).
    assert!(
        !check(&cfg, "feat: 한국어가 포함된 커밋 메시지").is_empty(),
        "Korean in English-required subject must fail"
    );

    // English subject with Japanese must fail.
    assert!(
        !check(&cfg, "feat: add feature with 日本語 mixed in").is_empty(),
        "Japanese in English-required subject must fail"
    );

    // English subject with Chinese must fail.
    assert!(
        !check(&cfg, "feat: add 中文 content in message").is_empty(),
        "Chinese in English-required subject must fail"
    );

    // English commit with English body passes.
    assert!(
        check(
            &cfg,
            "feat: add release workflow\n\nThis adds binaries for all platforms.\n"
        )
        .is_empty(),
        "English body must pass English check"
    );

    // English commit with Korean body must fail.
    assert!(
        !check(
            &cfg,
            "feat: add release workflow\n\n릴리즈 워크플로를 추가합니다.\n"
        )
        .is_empty(),
        "Korean body in English-required commit must fail"
    );
}
