//! Check step registry. Corresponds to Go `cmd/steps.go`.
//! Holds the shared check definition list for run/diff commands, the --only filter, forced enable via --only, and Step construction.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::progress::Step;
use git_warden_core::config::Config;
use git_warden_core::gitdiff::FileDiff;

/// Check step definition. Corresponds to Go `checkStepDef`.
pub struct StepDef {
    pub name_key: &'static str,
    pub category: &'static str,
    pub has_run: bool,
    pub has_diff: bool,
}

/// Full check step registry. Corresponds to Go `checkStepDefs` (same order).
pub const STEP_DEFS: &[StepDef] = &[
    StepDef {
        name_key: "step.binary_detection",
        category: "binary",
        has_run: true,
        has_diff: true,
    },
    StepDef {
        name_key: "step.encoding_check",
        category: "encoding",
        has_run: true,
        has_diff: true,
    },
    StepDef {
        name_key: "step.unicode_check",
        category: "unicode",
        has_run: true,
        has_diff: true,
    },
    StepDef {
        name_key: "step.lint_check",
        category: "lint",
        has_run: true,
        has_diff: true,
    },
    StepDef {
        name_key: "step.editorconfig_check",
        category: "editorconfig",
        has_run: true,
        has_diff: true,
    },
    StepDef {
        name_key: "step.comment_language_check",
        category: "comment_language",
        has_run: true,
        has_diff: true,
    },
    StepDef {
        name_key: "step.custom_rules_check",
        category: "custom_rules",
        has_run: false,
        has_diff: true,
    },
    StepDef {
        name_key: "step.protected_paths_check",
        category: "protected_paths",
        has_run: false,
        has_diff: true,
    },
    StepDef {
        name_key: "step.append_only_check",
        category: "append_only",
        has_run: false,
        has_diff: true,
    },
    StepDef {
        name_key: "step.cache_dir_check",
        category: "cache_dir",
        has_run: true,
        has_diff: true,
    },
];

/// Returns available step definitions for the given command type (run/diff). Corresponds to Go `stepDefsFor`.
/// When only is non-empty, retains only the specified categories (preserving registry order).
pub fn step_defs_for(diff_mode: bool, only: &[String]) -> Result<Vec<&'static StepDef>, String> {
    let available: Vec<&StepDef> = STEP_DEFS
        .iter()
        .filter(|d| if diff_mode { d.has_diff } else { d.has_run })
        .collect();
    let categories: Vec<&str> = available.iter().map(|d| d.category).collect();

    if only.is_empty() {
        return Ok(available);
    }

    let mut requested = std::collections::HashSet::new();
    for cat in only {
        let cat = cat.trim();
        if cat.is_empty() {
            continue;
        }
        if !categories.contains(&cat) {
            return Err(git_warden_core::t!(
                "flag.only_invalid",
                Category = cat,
                Valid = categories.join(", ")
            ));
        }
        requested.insert(cat.to_string());
    }

    Ok(available
        .into_iter()
        .filter(|d| requested.contains(d.category))
        .collect())
}

/// Returns a cloned Config with enabled forcibly set for checks selected via --only. Corresponds to Go `cfgWithOnlyEnabled`.
pub fn cfg_with_only_enabled(cfg: &Config, defs: &[&StepDef]) -> Config {
    let mut c = cfg.clone();
    for def in defs {
        match def.category {
            "binary" => c.binary_file.enabled = Some(true),
            "encoding" => {
                c.encoding.enabled = Some(true);
                c.encoding.require_utf8 = Some(true);
            }
            "unicode" => c.encoding.enabled = Some(true),
            "lint" => c.lint.enabled = Some(true),
            "editorconfig" => c.editorconfig.enabled = Some(true),
            "comment_language" => c.comment_language.enabled = Some(true),
            "cache_dir" => c.cache_dir.enabled = Some(true),
            "protected_paths" => c.protected_paths.enabled = true,
            "append_only" => c.append_only.enabled = true,
            _ => {} // custom_rules: no enabled toggle
        }
    }
    c
}

/// Builds Step list for the run command. Corresponds to Go `runSteps`.
pub fn run_steps(
    defs: &[&'static StepDef],
    cfg: Arc<Config>,
    files: Arc<Vec<String>>,
) -> Vec<Step> {
    let mut steps = Vec::new();
    for def in defs {
        if !def.has_run {
            continue;
        }
        let cfg = cfg.clone();
        let files = files.clone();
        let category = def.category;
        let func = Box::new(
            move |cancel: Arc<AtomicBool>| -> Result<Vec<String>, String> {
                run_dispatch(category, &cfg, &files, &cancel)
            },
        );
        steps.push(Step {
            name: git_warden_core::t!(def.name_key),
            category: def.category.to_string(),
            func,
        });
    }
    steps
}

/// Builds Step list for the diff command. Corresponds to Go `diffSteps`.
pub fn diff_steps(
    defs: &[&'static StepDef],
    cfg: Arc<Config>,
    diffs: Arc<Vec<FileDiff>>,
) -> Vec<Step> {
    let mut steps = Vec::new();
    for def in defs {
        if !def.has_diff {
            continue;
        }
        let cfg = cfg.clone();
        let diffs = diffs.clone();
        let category = def.category;
        let func = Box::new(
            move |cancel: Arc<AtomicBool>| -> Result<Vec<String>, String> {
                diff_dispatch(category, &cfg, &diffs, &cancel)
            },
        );
        steps.push(Step {
            name: git_warden_core::t!(def.name_key),
            category: def.category.to_string(),
            func,
        });
    }
    steps
}

fn run_dispatch(
    category: &str,
    cfg: &Config,
    files: &[String],
    cancel: &AtomicBool,
) -> Result<Vec<String>, String> {
    match category {
        "binary" => git_warden_core::checker::run_binary_files(cfg, files, cancel),
        "encoding" => git_warden_core::checker::run_encoding(cfg, files, cancel),
        "unicode" => git_warden_core::checker::run_unicode(cfg, files, cancel),
        "lint" => git_warden_core::checker::run_lint(cfg, files, cancel),
        "editorconfig" => git_warden_core::checker::run_editorconfig(cfg, files, cancel),
        "comment_language" => git_warden_core::checker::run_comment_language(cfg, files, cancel),
        // Cache directories are checked by directory scan rather than tracked file list.
        "cache_dir" => git_warden_core::checker::check_cache_dir_committed(cfg, cancel),
        _ => Ok(Vec::new()),
    }
}

fn diff_dispatch(
    category: &str,
    cfg: &Config,
    diffs: &[FileDiff],
    cancel: &AtomicBool,
) -> Result<Vec<String>, String> {
    match category {
        // Binary detection uses numstat, so diffs are not needed.
        "binary" => git_warden_core::checker::check_binary_files(cfg, cancel),
        "encoding" => git_warden_core::checker::check_encoding(cfg, cancel),
        "unicode" => git_warden_core::checker::check_unicode(cfg, cancel),
        "lint" => git_warden_core::checker::check_lint(cfg, cancel),
        "editorconfig" => git_warden_core::checker::check_editorconfig(cfg, cancel),
        "comment_language" => git_warden_core::checker::check_diff(cfg, diffs, cancel),
        "custom_rules" => git_warden_core::checker::check_diff_custom_rules(cfg, diffs, cancel),
        "protected_paths" => git_warden_core::checker::check_protected_paths(cfg, diffs, cancel),
        "append_only" => git_warden_core::checker::check_append_only(cfg, diffs, cancel),
        "cache_dir" => git_warden_core::checker::check_cache_dir_staged(cfg, diffs, cancel),
        _ => Ok(Vec::new()),
    }
}
