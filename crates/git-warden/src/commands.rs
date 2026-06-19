//! All subcommand handlers. Corresponds to Go `cmd/*.go`.

use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use git_warden_config::Config;

use crate::output::{guide_enabled, print_commit_message_guide, run_steps_and_report};
use crate::steps::{cfg_with_only_enabled, diff_steps, run_steps, step_defs_for};
use crate::{CmdError, Globals};

fn new_cancel() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

/// Resolves the project config file path. When using the default, accepts both .yaml and .yml. Corresponds to Go `resolveConfigFilePath`.
pub fn resolve_config_file_path(path: &str) -> String {
    if path == ".git-warden.yaml" || path == ".git-warden.yml" {
        for candidate in [".git-warden.yaml", ".git-warden.yml"] {
            if Path::new(candidate).exists() {
                return candidate.to_string();
            }
        }
    }
    path.to_string()
}

/// Returns true if --require-config is set and the config file is absent. Corresponds to Go `requireConfigSkip`.
fn require_config_skip(g: &Globals) -> bool {
    if !g.require_config {
        return false;
    }
    !Path::new(&resolve_config_file_path(&g.config_file)).exists()
}

fn load_cfg(g: &Globals) -> Result<Config, CmdError> {
    git_warden_config::load(&resolve_config_file_path(&g.config_file))
        .map_err(|e| CmdError::Msg(format!("failed to load config: {e}")))
}

// ── run ──
pub fn cmd_run(g: &Globals, format: &str, only: &[String]) -> Result<(), CmdError> {
    if require_config_skip(g) {
        return Ok(());
    }
    let defs = step_defs_for(false, only).map_err(CmdError::Msg)?;
    let mut cfg = load_cfg(g)?;
    if !cfg.is_enabled() {
        return Ok(());
    }
    if !only.is_empty() {
        cfg = cfg_with_only_enabled(&cfg, &defs);
    }
    let files = git_warden_checker::get_tracked_files()
        .map_err(|e| CmdError::Msg(format!("failed to list tracked files: {e}")))?;
    let cfg = Arc::new(cfg);
    let with_guide = guide_enabled(g, &cfg);
    let steps = run_steps(&defs, cfg.clone(), Arc::new(files));
    run_steps_and_report(g, steps, format, with_guide, new_cancel())
}

// ── diff ──
pub fn cmd_diff(
    g: &Globals,
    format: &str,
    staged: bool,
    only: &[String],
    args: &[String],
) -> Result<(), CmdError> {
    if require_config_skip(g) {
        return Ok(());
    }
    let defs = step_defs_for(true, only).map_err(CmdError::Msg)?;
    let spec = git_warden_gitdiff::spec_from_args(args, staged).map_err(CmdError::Msg)?;
    git_warden_gitdiff::set_spec(spec);

    let mut cfg = load_cfg(g)?;
    if !cfg.is_enabled() {
        return Ok(());
    }
    if !only.is_empty() {
        cfg = cfg_with_only_enabled(&cfg, &defs);
    }
    let diffs = git_warden_gitdiff::get_staged_diff().map_err(CmdError::Msg)?;
    let cfg = Arc::new(cfg);
    let with_guide = guide_enabled(g, &cfg);
    let steps = diff_steps(&defs, cfg.clone(), Arc::new(diffs));
    run_steps_and_report(g, steps, format, with_guide, new_cancel())
}

// ── msg ──
pub fn cmd_msg(g: &Globals, msg_file: &str, fix: bool) -> Result<(), CmdError> {
    if require_config_skip(g) {
        return Ok(());
    }
    let cfg = load_cfg(g)?;
    if !cfg.is_enabled() {
        return Ok(());
    }
    let mut content = std::fs::read(msg_file)
        .map_err(|e| CmdError::Msg(format!("failed to read commit message file: {e}")))?;

    if fix {
        let result = git_warden_checker::fix_msg(&cfg, &content);
        if result.needs_fixing() {
            std::fs::write(msg_file, &result.fixed)
                .map_err(|e| CmdError::Msg(format!("failed to write fixed commit message: {e}")))?;
            content = result.fixed.into_bytes();
        }
    }

    let errs = git_warden_checker::check_msg(&cfg, &content);
    if !errs.is_empty() {
        for e in &errs {
            eprintln!("{e}");
        }
        if guide_enabled(g, &cfg) {
            print_commit_message_guide();
        }
        return Err(CmdError::Silent);
    }
    Ok(())
}

// ── push ──
pub fn cmd_push(g: &Globals, range: &str) -> Result<(), CmdError> {
    if require_config_skip(g) {
        return Ok(());
    }
    let cfg = load_cfg(g)?;
    if !cfg.is_enabled() {
        return Ok(());
    }

    let commit_ranges: Vec<String> = if !range.is_empty() {
        vec![range.to_string()]
    } else {
        // Only read stdin when it is piped/file (not a terminal).
        if std::io::stdin().is_terminal_compat() {
            Vec::new()
        } else {
            let mut buf = String::new();
            let _ = std::io::stdin().read_to_string(&mut buf);
            parse_push_ranges(&buf)
        }
    };

    if commit_ranges.is_empty() {
        return Ok(());
    }

    let mut all_errs = Vec::new();
    for r in &commit_ranges {
        match list_push_commit_hashes(r) {
            Ok(hashes) => {
                for hash in hashes {
                    match get_push_commit_message(&hash) {
                        Ok(msg) => {
                            for e in git_warden_checker::check_msg(&cfg, msg.as_bytes()) {
                                all_errs.push(format!("[{}] {}", &hash[..7.min(hash.len())], e));
                            }
                        }
                        Err(err) => eprintln!(
                            "{}",
                            git_warden_i18n::t!(
                                "cmd.push.warn_msg_failed",
                                Hash = &hash[..7.min(hash.len())],
                                Error = err
                            )
                        ),
                    }
                }
            }
            Err(err) => eprintln!(
                "{}",
                git_warden_i18n::t!("cmd.push.warn_list_failed", Range = r, Error = err)
            ),
        }
    }

    if !all_errs.is_empty() {
        for e in &all_errs {
            eprintln!("{e}");
        }
        if guide_enabled(g, &cfg) {
            print_commit_message_guide();
        }
        return Err(CmdError::Silent);
    }
    Ok(())
}

fn parse_push_ranges(input: &str) -> Vec<String> {
    let mut ranges = Vec::new();
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 4 {
            continue;
        }
        let local_sha = parts[1];
        let remote_sha = parts[3];
        if is_push_zero_sha(local_sha) {
            continue;
        }
        if is_push_zero_sha(remote_sha) {
            let base = find_push_remote_base();
            if base.is_empty() {
                eprintln!(
                    "{}",
                    git_warden_i18n::t!("cmd.push.warn_no_base", Ref = parts[0])
                );
                continue;
            }
            ranges.push(format!("{base}..{local_sha}"));
        } else {
            ranges.push(format!("{remote_sha}..{local_sha}"));
        }
    }
    ranges
}

fn is_push_zero_sha(sha: &str) -> bool {
    sha.len() == 40 && sha.bytes().all(|c| c == b'0')
}

fn find_push_remote_base() -> String {
    if let Ok(out) = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .output()
    {
        if out.status.success() {
            return String::from_utf8_lossy(&out.stdout).trim().to_string();
        }
    }
    if let Ok(out) = Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
        .output()
    {
        if out.status.success() {
            let r = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if let Some(after) = r.strip_prefix("refs/remotes/") {
                return after.to_string();
            }
        }
    }
    for branch in ["origin/main", "origin/master"] {
        if Command::new("git")
            .args(["rev-parse", "--verify", branch])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return branch.to_string();
        }
    }
    String::new()
}

fn list_push_commit_hashes(commit_range: &str) -> Result<Vec<String>, String> {
    let out = Command::new("git")
        .args(["log", "--format=%H", commit_range])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

fn get_push_commit_message(hash: &str) -> Result<String, String> {
    let out = Command::new("git")
        .args(["show", "-s", "--format=%B", hash])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .trim_end_matches('\n')
        .to_string())
}

// ── prepare-msg ──
pub fn cmd_prepare_msg(g: &Globals, msg_file: &str, source: Option<&str>) -> Result<(), CmdError> {
    if matches!(source, Some("message" | "merge" | "squash" | "commit")) {
        return Ok(());
    }
    let cfg = load_cfg(g)?;
    let hint = prepare_msg_hint(&cfg);
    if hint.is_empty() {
        return Ok(());
    }
    let content = std::fs::read_to_string(msg_file)
        .map_err(|e| CmdError::Msg(format!("failed to read commit message file: {e}")))?;
    if content.contains(&prepare_msg_header_line()) {
        return Ok(());
    }
    let mut out = content;
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&hint);
    std::fs::write(msg_file, out)
        .map_err(|e| CmdError::Msg(format!("failed to write commit message file: {e}")))?;
    Ok(())
}

fn prepare_msg_header_line() -> String {
    format!("# {}", git_warden_i18n::t!("cmd.prepare_msg.hint_header"))
}

fn prepare_msg_hint(cfg: &Config) -> String {
    let cm = &cfg.commit_message;
    if !cm.is_enabled() {
        return String::new();
    }
    let mut lines = Vec::new();
    if cm.conventional_commit.is_enabled() {
        lines.push(git_warden_i18n::t!("cmd.prepare_msg.hint_format"));
        lines.push(git_warden_i18n::t!(
            "cmd.prepare_msg.hint_types",
            Types = cm.conventional_commit.get_all_allowed_types().join(", ")
        ));
    }
    if cm.language_check.is_enabled() {
        let lang = cm.language_check.get_locale();
        if lang != git_warden_langdetect::ANY {
            lines.push(git_warden_i18n::t!(
                "cmd.prepare_msg.hint_language",
                Language = language_display_name(&lang)
            ));
        }
    }
    if cm.is_no_ai_coauthor() {
        lines.push(git_warden_i18n::t!("cmd.prepare_msg.hint_coauthor"));
    }
    if lines.is_empty() {
        return String::new();
    }
    let mut b = String::new();
    b.push_str(&prepare_msg_header_line());
    b.push('\n');
    for l in lines {
        b.push_str("# ");
        b.push_str(&l);
        b.push('\n');
    }
    b
}

fn language_display_name(lang: &str) -> String {
    let key = format!("lang.{lang}");
    let name = git_warden_i18n::translate(&key, &[]);
    if name == format!("[{key}]") {
        lang.to_string()
    } else {
        name
    }
}

// ── fix ──
pub fn cmd_fix(g: &Globals, dry_run: bool) -> Result<(), CmdError> {
    let cfg = load_cfg(g)?;
    let files = get_staged_files_for_fix().map_err(CmdError::Msg)?;
    if files.is_empty() {
        println!("{}", git_warden_i18n::t!("cmd.fix.no_staged_files"));
        return Ok(());
    }
    let mut fixed_count = 0;
    for path in &files {
        let content = match std::fs::read(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "{}",
                    git_warden_i18n::t!("cmd.fix.warn_read_failed", Path = path, Error = e)
                );
                continue;
            }
        };
        if git_warden_encoding::is_binary(&content) {
            continue;
        }
        let result = git_warden_checker::fix_file_content(&cfg, &content);
        if !result.needs_fixing() {
            continue;
        }
        println!("{path}:");
        for ch in &result.changes {
            println!("  - {ch}");
        }
        if !dry_run {
            let perm = std::fs::metadata(path).map(|m| m.permissions()).ok();
            std::fs::write(path, &result.fixed)
                .map_err(|e| CmdError::Msg(format!("writing {path}: {e}")))?;
            if let Some(perm) = perm {
                let _ = std::fs::set_permissions(path, perm);
            }
            run_git_add(path).map_err(|e| CmdError::Msg(format!("git add {path}: {e}")))?;
        }
        fixed_count += 1;
    }

    if fixed_count == 0 {
        println!("{}", git_warden_i18n::t!("cmd.fix.no_issues"));
    } else if dry_run {
        println!(
            "{}",
            git_warden_i18n::t!("cmd.fix.dry_run_summary", Count = fixed_count)
        );
    } else {
        println!(
            "{}",
            git_warden_i18n::t!("cmd.fix.fixed_summary", Count = fixed_count)
        );
    }
    Ok(())
}

fn get_staged_files_for_fix() -> Result<Vec<String>, String> {
    let out = Command::new("git")
        .args(["diff", "--staged", "--name-only", "--diff-filter=ACM", "-z"])
        .output()
        .map_err(|e| format!("git diff --staged: {e}"))?;
    Ok(git_warden_gitdiff::split_null_separated(&out.stdout))
}

fn run_git_add(path: &str) -> Result<(), String> {
    Command::new("git")
        .args(["add", path])
        .status()
        .map_err(|e| e.to_string())
        .and_then(|s| {
            if s.success() {
                Ok(())
            } else {
                Err("git add failed".into())
            }
        })
}

// ── version ──
pub fn cmd_version() {
    println!("git-warden {}", git_warden_version::version());
    println!(
        "{}",
        git_warden_i18n::t!("version.commit", Value = git_warden_version::commit())
    );
    println!(
        "{}",
        git_warden_i18n::t!(
            "version.build_time",
            Value = git_warden_version::build_time()
        )
    );
}

// ── validate ──
pub fn cmd_validate(g: &Globals) -> Result<(), CmdError> {
    let path = resolve_config_file_path(&g.config_file);
    let cfg = git_warden_config::load(&path).map_err(CmdError::Msg)?;
    let mut warnings = git_warden_config::validate(&cfg, &path);
    // Enhancement: also validate against JSON Schema to catch unknown (typo'd) fields and enum
    // violations that serde silently ignores.
    warnings.extend(git_warden_config::validate_schema_file(&path));
    if warnings.is_empty() {
        println!("{}", git_warden_i18n::t!("validate.config_valid"));
        return Ok(());
    }
    for w in &warnings {
        eprintln!("{w}");
    }
    Err(CmdError::Msg(git_warden_i18n::t!(
        "validate.warnings_found",
        Count = warnings.len()
    )))
}

// ── migrate ──
pub fn cmd_migrate(g: &Globals, dry_run: bool) -> Result<(), CmdError> {
    let path = resolve_config_file_path(&g.config_file);
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(CmdError::Msg(git_warden_i18n::t!(
                "migrate.file_not_found",
                Path = path
            )));
        }
        Err(e) => return Err(CmdError::Msg(e.to_string())),
    };
    let result = git_warden_config::schema::migrate(&data)
        .map_err(|e| CmdError::Msg(git_warden_i18n::t!("migrate.failed", Error = e)))?;

    if result.detected_version == git_warden_config::schema::Version::Current {
        println!(
            "{}",
            git_warden_i18n::t!("migrate.already_current", Path = path)
        );
        return Ok(());
    }
    println!(
        "{}",
        git_warden_i18n::t!(
            "migrate.detected_version",
            Version = result.detected_version.as_str()
        )
    );
    for desc in &result.applied {
        println!("{}", git_warden_i18n::t!("migrate.change", Desc = desc));
    }
    if dry_run {
        println!("{}", git_warden_i18n::t!("migrate.dry_run_header"));
        print!("{}", String::from_utf8_lossy(&result.data));
        return Ok(());
    }
    std::fs::write(&path, &result.data)
        .map_err(|e| CmdError::Msg(git_warden_i18n::t!("migrate.save_failed", Error = e)))?;
    println!("{}", git_warden_i18n::t!("migrate.saved", Path = path));
    Ok(())
}

// stdin TTY detection helper.
trait IsTerminalCompat {
    fn is_terminal_compat(&self) -> bool;
}
impl IsTerminalCompat for std::io::Stdin {
    fn is_terminal_compat(&self) -> bool {
        use std::io::IsTerminal;
        self.is_terminal()
    }
}
