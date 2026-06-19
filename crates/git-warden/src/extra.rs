//! clean / analyze / init commands. Corresponds to Go `cmd/clean.go`, `analyze.go`, `init.go`.

use std::path::Path;
use std::process::Command;

use crate::{CmdError, Globals};

// ── clean ──
pub fn cmd_clean(g: &Globals, yes: bool) -> Result<(), CmdError> {
    let cwd = std::env::current_dir().map_err(|e| CmdError::Msg(e.to_string()))?;
    let repo_root = match git_warden_core::cachedir::find_repo_root(&cwd) {
        Ok(r) => r,
        Err(_) => {
            eprintln!(
                "{}",
                git_warden_core::t!("cmd.clean.not_in_repo", Path = cwd.display())
            );
            return Err(CmdError::Silent);
        }
    };

    if !g.quiet {
        eprintln!(
            "{}",
            git_warden_core::t!("cmd.clean.scanning", Path = repo_root.display())
        );
    }

    let dirs = git_warden_core::cachedir::find_cache_dirs_in_repo(&repo_root);
    if dirs.is_empty() {
        println!("{}", git_warden_core::t!("cmd.clean.no_cache_dirs"));
        return Ok(());
    }

    struct Entry {
        abs: std::path::PathBuf,
        rel: String,
        size: i64,
        tracked: usize,
    }
    let mut entries = Vec::new();
    let mut total = 0i64;
    for d in &dirs {
        let rel = d
            .strip_prefix(&repo_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| d.to_string_lossy().to_string());
        let size = git_warden_core::cachedir::get_dir_size(d);
        let tracked = git_warden_core::cachedir::list_tracked_entries(&repo_root, d)
            .map(|v| v.len())
            .unwrap_or(0);
        entries.push(Entry {
            abs: d.clone(),
            rel,
            size,
            tracked,
        });
        total += size;
    }
    entries.sort_by_key(|e| std::cmp::Reverse(e.size));

    println!(
        "{}",
        git_warden_core::t!(
            "cmd.clean.found_header",
            Count = entries.len(),
            Size = git_warden_core::cachedir::format_bytes(total)
        )
    );
    for e in &entries {
        let tracked = if e.tracked > 0 {
            git_warden_core::t!("cmd.clean.entry_tracked", Count = e.tracked)
        } else {
            String::new()
        };
        println!(
            "{}",
            git_warden_core::t!(
                "cmd.clean.entry",
                Path = e.rel,
                Size = git_warden_core::cachedir::format_bytes(e.size),
                Tracked = tracked
            )
        );
    }

    if !yes {
        println!("{}", git_warden_core::t!("cmd.clean.dry_run_hint"));
        return Ok(());
    }

    let mut freed = 0i64;
    let mut cleaned = 0;
    for e in &entries {
        let untracked = match git_warden_core::cachedir::list_untracked_entries(&repo_root, &e.abs)
        {
            Ok(u) => u,
            Err(_) => continue,
        };
        for p in &untracked {
            let size = git_warden_core::cachedir::get_dir_size(p);
            if std::fs::remove_dir_all(p).is_ok() || std::fs::remove_file(p).is_ok() {
                freed += size;
            }
        }
        cleaned += 1;
    }
    println!(
        "{}",
        git_warden_core::t!(
            "cmd.clean.cleaned_summary",
            Size = git_warden_core::cachedir::format_bytes(freed),
            Count = cleaned
        )
    );
    Ok(())
}

// ── analyze ──
struct LintConfigCheck {
    language: &'static str,
    config_files: &'static [&'static str],
}

const KNOWN_LINT_CONFIGS: &[LintConfigCheck] = &[
    LintConfigCheck {
        language: "Go",
        config_files: &[
            ".golangci.yml",
            ".golangci.yaml",
            ".golangci.toml",
            ".golangci.json",
        ],
    },
    LintConfigCheck {
        language: "TypeScript/JavaScript",
        config_files: &[
            ".eslintrc",
            ".eslintrc.json",
            ".eslintrc.yml",
            ".eslintrc.yaml",
            ".eslintrc.js",
            ".eslintrc.cjs",
            "eslint.config.js",
            "eslint.config.mjs",
            "eslint.config.cjs",
            "eslint.config.ts",
            "biome.json",
            "biome.jsonc",
            ".prettierrc",
            ".prettierrc.json",
            ".prettierrc.yml",
        ],
    },
    LintConfigCheck {
        language: "Python",
        config_files: &[
            ".pylintrc",
            ".flake8",
            ".ruff.toml",
            "ruff.toml",
            "pyproject.toml",
            "setup.cfg",
            ".pyre_configuration",
        ],
    },
    LintConfigCheck {
        language: "Java",
        config_files: &["checkstyle.xml", "pmd.xml", ".checkstyle", "spotbugs.xml"],
    },
    LintConfigCheck {
        language: "Kotlin",
        config_files: &["detekt.yml", "detekt.yaml", ".detekt.yml"],
    },
    LintConfigCheck {
        language: "Rust",
        config_files: &["rustfmt.toml", ".rustfmt.toml", "clippy.toml"],
    },
    LintConfigCheck {
        language: "C/C++",
        config_files: &[".clang-format", ".clang-tidy"],
    },
    LintConfigCheck {
        language: "Swift",
        config_files: &[".swiftlint.yml", ".swiftformat"],
    },
    LintConfigCheck {
        language: "C#",
        config_files: &[".editorconfig", "omnisharp.json", "Directory.Build.props"],
    },
];

fn extension_to_language(ext: &str) -> Option<&'static str> {
    Some(match ext {
        ".go" => "Go",
        ".ts" | ".tsx" | ".js" | ".jsx" | ".mjs" | ".cjs" => "TypeScript/JavaScript",
        ".py" => "Python",
        ".java" => "Java",
        ".kt" | ".kts" => "Kotlin",
        ".rs" => "Rust",
        ".c" | ".cpp" | ".cc" | ".h" | ".hpp" => "C/C++",
        ".swift" => "Swift",
        ".cs" => "C#",
        ".rb" => "Ruby",
        ".php" => "PHP",
        ".yaml" | ".yml" => "YAML",
        ".json" => "JSON",
        ".xml" => "XML",
        ".html" => "HTML",
        ".css" | ".scss" => "CSS",
        ".sh" | ".bash" => "Shell",
        ".md" => "Markdown",
        _ => return None,
    })
}

fn ext_of(path: &str) -> String {
    match path.rfind('.') {
        Some(i) if i > path.rfind('/').map(|s| s + 1).unwrap_or(0) => path[i..].to_lowercase(),
        _ => String::new(),
    }
}

pub fn cmd_analyze(_g: &Globals) -> Result<(), CmdError> {
    let files =
        analyze_tracked_files().map_err(|e| CmdError::Msg(format!("failed to list files: {e}")))?;

    // Count files per language.
    let mut counts: std::collections::HashMap<&'static str, i64> = std::collections::HashMap::new();
    for f in &files {
        if let Some(lang) = extension_to_language(&ext_of(f)) {
            *counts.entry(lang).or_insert(0) += 1;
        }
    }
    let mut langs: Vec<(&'static str, i64)> = counts.into_iter().collect();
    // Sort by count descending, then name ascending for deterministic output.
    langs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));

    println!("{}", git_warden_core::t!("analyze.header"));
    println!();
    println!("{}", git_warden_core::t!("analyze.detected_languages"));
    if langs.is_empty() {
        println!("{}", git_warden_core::t!("analyze.no_languages"));
    }
    for (name, count) in &langs {
        println!(
            "{}",
            git_warden_core::t!("analyze.lang_entry", Name = name, Count = count)
        );
    }
    println!();

    let data_for_prog = ["YAML", "JSON", "XML", "HTML", "CSS", "Markdown", "Shell"];
    let programming: Vec<&(&str, i64)> = langs
        .iter()
        .filter(|(n, _)| !data_for_prog.contains(n))
        .collect();

    println!("{}", git_warden_core::t!("analyze.lint_config_status"));
    if programming.is_empty() {
        println!("{}", git_warden_core::t!("analyze.no_programming_langs"));
    }
    for (name, _) in &programming {
        match check_lint_config(name) {
            Some(config) => println!(
                "{}",
                git_warden_core::t!("analyze.lint_found", Language = name, Config = config)
            ),
            None => println!(
                "{}",
                git_warden_core::t!("analyze.lint_not_found", Language = name)
            ),
        }
    }
    println!();

    println!("{}", git_warden_core::t!("analyze.project_config"));
    for f in [
        ".editorconfig",
        ".git-warden.yml",
        ".git-warden.yaml",
        ".gitattributes",
        ".gitignore",
    ] {
        check_and_report(f);
    }
    println!();

    let data_types = ["YAML", "JSON", "XML"];
    let data_langs: Vec<&(&str, i64)> = langs
        .iter()
        .filter(|(n, _)| data_types.contains(n))
        .collect();
    if !data_langs.is_empty() {
        println!("{}", git_warden_core::t!("analyze.data_files"));
        for (name, count) in &data_langs {
            println!(
                "{}",
                git_warden_core::t!("analyze.lang_entry", Name = name, Count = count)
            );
        }
        println!();
    }
    Ok(())
}

fn analyze_tracked_files() -> Result<Vec<String>, String> {
    let out = Command::new("git")
        .args(["ls-files", "-z"])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("git ls-files failed".into());
    }
    Ok(git_warden_core::gitdiff::split_null_separated(&out.stdout))
}

fn check_lint_config(language: &str) -> Option<String> {
    for lc in KNOWN_LINT_CONFIGS {
        if lc.language == language {
            for cf in lc.config_files {
                if Path::new(cf).exists() {
                    return Some(cf.to_string());
                }
            }
            return None;
        }
    }
    None
}

fn check_and_report(filename: &str) {
    if Path::new(filename).exists() {
        println!(
            "{}",
            git_warden_core::t!("analyze.file_found", File = filename)
        );
    } else {
        println!(
            "{}",
            git_warden_core::t!("analyze.file_not_found", File = filename)
        );
    }
}

// ── init ──
const CONFIG_TEMPLATE: &str = include_str!("../templates/config.yml.tmpl");

pub fn cmd_init(g: &Globals, force: bool, lang: &str) -> Result<(), CmdError> {
    let target = &g.config_file;
    if !force && Path::new(target).exists() {
        return Err(CmdError::Msg(git_warden_core::t!(
            "init.already_exists",
            Path = target
        )));
    }
    let content = default_config(lang);
    std::fs::write(target, content)
        .map_err(|e| CmdError::Msg(git_warden_core::t!("init.fail_write", Error = e)))?;
    println!("{}", git_warden_core::t!("init.created", Path = target));
    Ok(())
}

fn default_config(lang: &str) -> String {
    let lang = if lang.is_empty() {
        git_warden_core::i18n::detect_locale()
    } else {
        lang.to_lowercase()
    };
    let locale = if matches!(lang.as_str(), "ko" | "en" | "ja" | "zh") {
        lang.as_str()
    } else {
        "ko"
    };
    let example = match locale {
        "ko" => "기능",
        "ja" => "機能",
        "zh" => "功能",
        _ => "feat",
    };
    // Substitutes %s placeholders left-to-right, matching Go's fmt.Sprintf(template, locale, locale, locale, example, locale).
    let args = [locale, locale, locale, example, locale];
    sprintf_s(CONFIG_TEMPLATE, &args)
}

/// Substitutes `%s` placeholders in the template left-to-right with args (mirrors Go fmt.Sprintf %s behavior).
fn sprintf_s(template: &str, args: &[&str]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut idx = 0;
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 1 < bytes.len() && bytes[i + 1] == b's' {
            if idx < args.len() {
                out.push_str(args[idx]);
                idx += 1;
            }
            i += 2;
            continue;
        }
        let ch_len = utf8_len(bytes[i]);
        out.push_str(&template[i..i + ch_len]);
        i += ch_len;
    }
    out
}

fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else if b >> 3 == 0b11110 {
        4
    } else {
        1
    }
}
