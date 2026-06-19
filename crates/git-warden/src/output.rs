//! Check result reporting and improvement guides. Corresponds to Go `cmd/output.go`.

use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use cc_config::Config;
use cc_progress::{format_json, run_with_progress, summary, Options, Step, StepResult};

use crate::{CmdError, Globals};

/// Whether improvement guide output is enabled. Corresponds to Go `guideEnabled`.
pub fn guide_enabled(g: &Globals, cfg: &Config) -> bool {
    !g.no_guide && cfg.guide.is_enabled()
}

/// Guide text for a category. Returns empty string if the key (guide.<category>) is missing. Corresponds to Go `guideText`.
fn guide_text(category: &str) -> String {
    let key = format!("guide.{category}");
    let text = cc_i18n::translate(&key, &[]);
    if text == format!("[{key}]") {
        String::new()
    } else {
        text
    }
}

/// Collects per-category guide text for steps with violations (preserving step order, deduplicating). Corresponds to Go `failedGuides`.
fn failed_guides(steps: &[StepResult]) -> (Vec<String>, BTreeMap<String, String>) {
    let mut categories = Vec::new();
    let mut guides = BTreeMap::new();
    for s in steps {
        if s.errors.is_empty() || s.category.is_empty() {
            continue;
        }
        if guides.contains_key(&s.category) {
            continue;
        }
        let text = guide_text(&s.category);
        if text.is_empty() {
            continue;
        }
        categories.push(s.category.clone());
        guides.insert(s.category.clone(), text);
    }
    (categories, guides)
}

/// Prints guide header and per-category text to stderr. Corresponds to Go `printGuides`.
fn print_guides(categories: &[String], guides: &BTreeMap<String, String>) {
    if categories.is_empty() {
        return;
    }
    eprintln!();
    eprintln!("{}", cc_i18n::t!("guide.header"));
    for cat in categories {
        if let Some(text) = guides.get(cat) {
            eprintln!("  [{cat}] {text}");
        }
    }
}

/// Prints the commit_message guide once when a commit message violation occurs. Corresponds to Go `printCommitMessageGuide`.
pub fn print_commit_message_guide() {
    let cat = "commit_message";
    let text = guide_text(cat);
    if !text.is_empty() {
        let mut guides = BTreeMap::new();
        guides.insert(cat.to_string(), text);
        print_guides(&[cat.to_string()], &guides);
    }
}

/// Runs check steps and reports results in the requested format. Corresponds to Go `runStepsAndReport`.
pub fn run_steps_and_report(
    g: &Globals,
    steps: Vec<Step>,
    format: &str,
    with_guide: bool,
    cancel: Arc<AtomicBool>,
) -> Result<(), CmdError> {
    let opts = Options {
        quiet: g.quiet || format == "json",
        no_color: g.no_color,
    };
    let result = run_with_progress(steps, opts, cancel).map_err(CmdError::Msg)?;

    if format == "json" {
        let guides = if with_guide && !result.all_errors.is_empty() {
            failed_guides(&result.steps).1
        } else {
            BTreeMap::new()
        };
        let json = format_json(&result, &guides).map_err(CmdError::Msg)?;
        println!("{json}");
        if !result.all_errors.is_empty() {
            return Err(CmdError::Silent);
        }
        return Ok(());
    }

    // Text output
    for e in &result.all_errors {
        eprintln!("{e}");
    }
    if !result.all_errors.is_empty() {
        let (total, checks) = summary(&result.steps);
        if total > 0 {
            eprintln!(
                "{}",
                cc_i18n::t!("summary.violations", Count = total, Checks = checks)
            );
        }
        if with_guide {
            let (cats, guides) = failed_guides(&result.steps);
            print_guides(&cats, &guides);
        }
        return Err(CmdError::Silent);
    }

    Ok(())
}
