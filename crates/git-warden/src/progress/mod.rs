//! cc-progress: check step execution, progress display, and result reporting. Corresponds to Go `internal/progress`.
//!
//! - When not a TTY or when quiet, steps run sequentially with a text fallback (`run_plain`) — test/CI/hook path.
//! - When a TTY is detected, steps run with a ratatui inline-viewport spinner (corresponds to Go's bubbletea TUI; decorative only).
//! - `RunResult`, `summary`, and `format_json` are equivalent to Go.
//!
//! Cancellation is modelled as an `Arc<AtomicBool>` (true = cancelled), corresponding to Go's `context.Context`.

use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Cancellation flag for check step functions. True means cancelled.
pub type CancelFlag = Arc<AtomicBool>;

/// Step execution function: receives a cancel flag and returns a list of violation messages or a fatal error.
pub type StepFn = Box<dyn FnOnce(CancelFlag) -> Result<Vec<String>, String> + Send>;

/// A check step to execute. Corresponds to Go `Step`.
pub struct Step {
    pub name: String,
    pub category: String,
    pub func: StepFn,
}

/// Result of a single step execution. Corresponds to Go `StepResult`.
#[derive(Debug, Clone, Default)]
pub struct StepResult {
    pub name: String,
    pub category: String,
    pub errors: Vec<String>,
    pub failed: bool,
}

/// Return value of RunWithProgress. Corresponds to Go `RunResult`.
#[derive(Debug, Clone, Default)]
pub struct RunResult {
    pub all_errors: Vec<String>,
    pub steps: Vec<StepResult>,
}

/// Execution options. Corresponds to Go `Options`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub quiet: bool,
    pub no_color: bool,
}

/// Runs steps sequentially and returns the result. On cancellation, skips remaining steps and returns Err. Corresponds to Go `RunWithProgress`.
pub fn run_with_progress(
    steps: Vec<Step>,
    opts: Options,
    cancel: CancelFlag,
) -> Result<RunResult, String> {
    if opts.quiet {
        return run_plain(steps, cancel, None);
    }
    if !std::io::stderr().is_terminal() {
        return run_plain(steps, cancel, Some(Output::Stderr));
    }
    run_tui(steps, opts, cancel)
}

enum Output {
    Stderr,
}

/// Runs steps sequentially with text progress output (or no output). Corresponds to Go `runPlain`/`runPlainSilent`.
fn run_plain(
    steps: Vec<Step>,
    cancel: CancelFlag,
    out: Option<Output>,
) -> Result<RunResult, String> {
    let mut result = RunResult {
        all_errors: Vec::new(),
        steps: Vec::with_capacity(steps.len()),
    };
    // Pre-fill result slots (preserves step metadata even on cancellation).
    for s in &steps {
        result.steps.push(StepResult {
            name: s.name.clone(),
            category: s.category.clone(),
            errors: Vec::new(),
            failed: false,
        });
    }
    for (i, s) in steps.into_iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            return Err("interrupted".to_string());
        }
        if matches!(out, Some(Output::Stderr)) {
            let _ = writeln!(std::io::stderr(), "  {} ...", s.name);
        }
        match (s.func)(cancel.clone()) {
            Ok(errs) => {
                result.all_errors.extend(errs.iter().cloned());
                result.steps[i].errors = errs;
            }
            Err(e) => {
                result.steps[i].failed = true;
                return Err(e);
            }
        }
    }
    Ok(result)
}

/// Returns the total violation count and a "category(count)" summary string. Corresponds to Go `Summary`.
pub fn summary(steps: &[StepResult]) -> (usize, String) {
    let mut total = 0;
    let mut parts = Vec::new();
    for s in steps {
        let n = s.errors.len();
        if n > 0 {
            total += n;
            let cat = if s.category.is_empty() {
                &s.name
            } else {
                &s.category
            };
            parts.push(format!("{cat}({n})"));
        }
    }
    (total, parts.join(", "))
}

// --- JSON output ---

#[derive(serde::Serialize)]
struct JsonViolation {
    #[serde(skip_serializing_if = "str::is_empty")]
    file: String,
    #[serde(skip_serializing_if = "is_zero")]
    line: i64,
    message: String,
    check: String,
}

fn is_zero(n: &i64) -> bool {
    *n == 0
}

#[derive(serde::Serialize)]
struct JsonSummary {
    total: usize,
    by_check: std::collections::BTreeMap<String, usize>,
}

#[derive(serde::Serialize)]
struct JsonOutput {
    status: String,
    violations: Vec<JsonViolation>,
    summary: JsonSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    guides: Option<std::collections::BTreeMap<String, String>>,
}

/// Serializes the result as JSON. Corresponds to Go `FormatJSON`.
/// Includes guides when non-empty (guide text is built by the caller using i18n and passed in).
pub fn format_json(
    result: &RunResult,
    guides: &std::collections::BTreeMap<String, String>,
) -> Result<String, String> {
    let mut out = JsonOutput {
        status: "pass".to_string(),
        violations: Vec::new(),
        summary: JsonSummary {
            total: 0,
            by_check: std::collections::BTreeMap::new(),
        },
        guides: if guides.is_empty() {
            None
        } else {
            Some(guides.clone())
        },
    };
    for s in &result.steps {
        for msg in &s.errors {
            out.violations.push(JsonViolation {
                file: String::new(),
                line: 0,
                message: msg.clone(),
                check: s.category.clone(),
            });
            *out.summary.by_check.entry(s.category.clone()).or_insert(0) += 1;
            out.summary.total += 1;
        }
    }
    if out.summary.total > 0 {
        out.status = "fail".to_string();
    }
    serde_json::to_string_pretty(&out).map_err(|e| e.to_string())
}

// --- ratatui inline TUI ---

mod tui;

/// Runs steps with a spinner on a TTY. Falls back to text output on setup failure. Corresponds to Go `runTUI`.
fn run_tui(steps: Vec<Step>, opts: Options, cancel: CancelFlag) -> Result<RunResult, String> {
    match tui::run(steps, opts, cancel.clone()) {
        Ok(r) => r,
        // Fall back to plain output on TUI setup failure (steps are returned by tui::run on error).
        Err(steps) => run_plain(steps, cancel, Some(Output::Stderr)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn step(name: &str, cat: &str, errs: Vec<&str>, fail: bool) -> Step {
        let errs: Vec<String> = errs.into_iter().map(String::from).collect();
        Step {
            name: name.to_string(),
            category: cat.to_string(),
            func: Box::new(move |_cancel| {
                if fail {
                    Err("fatal".to_string())
                } else {
                    Ok(errs)
                }
            }),
        }
    }

    #[test]
    fn run_plain_collects_errors() {
        let steps = vec![
            step("Binary", "binary", vec![], false),
            step(
                "Unicode",
                "unicode",
                vec!["a.txt: bad char", "b.txt: bad char"],
                false,
            ),
        ];
        let cancel = Arc::new(AtomicBool::new(false));
        let r = run_plain(steps, cancel, None).unwrap();
        assert_eq!(r.all_errors.len(), 2);
        assert_eq!(r.steps[1].errors.len(), 2);
        assert!(!r.steps[0].failed);
    }

    #[test]
    fn run_plain_fatal_stops() {
        let steps = vec![
            step("A", "a", vec![], true),
            step("B", "b", vec!["x"], false),
        ];
        let cancel = Arc::new(AtomicBool::new(false));
        let r = run_plain(steps, cancel, None);
        assert!(r.is_err());
    }

    #[test]
    fn summary_format() {
        let steps = vec![
            StepResult {
                name: "Binary".into(),
                category: "binary".into(),
                errors: vec![],
                failed: false,
            },
            StepResult {
                name: "Unicode".into(),
                category: "unicode".into(),
                errors: vec!["a".into(), "b".into()],
                failed: false,
            },
            StepResult {
                name: "Lint".into(),
                category: "lint".into(),
                errors: vec!["c".into()],
                failed: false,
            },
        ];
        let (total, s) = summary(&steps);
        assert_eq!(total, 3);
        assert_eq!(s, "unicode(2), lint(1)");
    }

    #[test]
    fn format_json_pass_and_fail() {
        let pass = RunResult {
            all_errors: vec![],
            steps: vec![StepResult {
                name: "Binary".into(),
                category: "binary".into(),
                errors: vec![],
                failed: false,
            }],
        };
        let j = format_json(&pass, &BTreeMap::new()).unwrap();
        assert!(j.contains("\"status\": \"pass\""));
        assert!(j.contains("\"total\": 0"));

        let fail = RunResult {
            all_errors: vec!["a.txt: oops".into()],
            steps: vec![StepResult {
                name: "Unicode".into(),
                category: "unicode".into(),
                errors: vec!["a.txt: oops".into()],
                failed: false,
            }],
        };
        let j = format_json(&fail, &BTreeMap::new()).unwrap();
        assert!(j.contains("\"status\": \"fail\""));
        assert!(j.contains("\"check\": \"unicode\""));
        assert!(j.contains("\"a.txt: oops\""));
        // file/line are omitted when empty.
        assert!(!j.contains("\"file\""));
        assert!(!j.contains("\"line\""));
    }

    #[test]
    fn format_json_with_guides() {
        let r = RunResult {
            all_errors: vec!["x".into()],
            steps: vec![StepResult {
                name: "L".into(),
                category: "lint".into(),
                errors: vec!["x".into()],
                failed: false,
            }],
        };
        let mut guides = BTreeMap::new();
        guides.insert("lint".to_string(), "fix your yaml".to_string());
        let j = format_json(&r, &guides).unwrap();
        assert!(j.contains("\"guides\""));
        assert!(j.contains("fix your yaml"));
    }
}
