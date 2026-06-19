//! ratatui inline-viewport progress spinner. Corresponds to Go's bubbletea TUI (`runTUI`).
//! Decorative only; used exclusively on a TTY. The caller falls back to text output on init failure.

use super::{CancelFlag, Options, RunResult, Step, StepResult};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{Terminal, TerminalOptions, Viewport};

enum Msg {
    Done { idx: usize, errs: Vec<String> },
    Fatal { idx: usize, err: String },
    Finished,
}

// Spinner frames (similar to bubbletea spinner.Dot).
const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Runs steps via the TUI.
/// Returns `Ok(inner)` on success or fatal error; returns `Err(steps)` on TUI init failure (original steps for fallback).
pub(super) fn run(
    steps: Vec<Step>,
    opts: Options,
    cancel: CancelFlag,
) -> Result<Result<RunResult, String>, Vec<Step>> {
    // Capture step metadata (name/category) before transferring steps to the worker.
    let meta: Vec<(String, String)> = steps
        .iter()
        .map(|s| (s.name.clone(), s.category.clone()))
        .collect();
    let n = steps.len();

    // Initialize terminal (return original steps on failure → fallback).
    if enable_raw_mode().is_err() {
        return Err(steps);
    }
    let backend = CrosstermBackend::new(std::io::stderr());
    let height = (n as u16).saturating_add(1);
    let mut terminal = match Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(height),
        },
    ) {
        Ok(t) => t,
        Err(_) => {
            let _ = disable_raw_mode();
            return Err(steps);
        }
    };

    // worker: runs steps sequentially and sends results over the channel.
    let (tx, rx) = mpsc::channel::<Msg>();
    let worker_cancel = cancel.clone();
    let worker = std::thread::spawn(move || {
        for (idx, s) in steps.into_iter().enumerate() {
            if worker_cancel.load(Ordering::SeqCst) {
                break;
            }
            match (s.func)(worker_cancel.clone()) {
                Ok(errs) => {
                    if tx.send(Msg::Done { idx, errs }).is_err() {
                        return;
                    }
                }
                Err(err) => {
                    let _ = tx.send(Msg::Fatal { idx, err });
                    return;
                }
            }
        }
        let _ = tx.send(Msg::Finished);
    });

    // State
    let mut results: Vec<Option<StepResult>> = vec![None; n];
    let mut all_errors: Vec<String> = Vec::new();
    let mut current = 0usize;
    let mut frame = 0usize;
    let mut fatal: Option<String> = None;
    let mut done = false;

    while !done {
        // Process channel messages (non-blocking).
        loop {
            match rx.try_recv() {
                Ok(Msg::Done { idx, errs }) => {
                    all_errors.extend(errs.iter().cloned());
                    results[idx] = Some(StepResult {
                        name: meta[idx].0.clone(),
                        category: meta[idx].1.clone(),
                        errors: errs,
                        failed: false,
                    });
                    current = idx + 1;
                }
                Ok(Msg::Fatal { idx, err }) => {
                    results[idx] = Some(StepResult {
                        name: meta[idx].0.clone(),
                        category: meta[idx].1.clone(),
                        errors: Vec::new(),
                        failed: true,
                    });
                    fatal = Some(err);
                    done = true;
                }
                Ok(Msg::Finished) => {
                    done = true;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    done = true;
                    break;
                }
            }
        }

        // Render.
        let _ = terminal.draw(|f| {
            let mut lines: Vec<Line> = Vec::new();
            for (i, r) in results.iter().enumerate() {
                if i >= current && !done {
                    break;
                }
                let Some(r) = r else { continue };
                let (icon, color) = if r.failed {
                    ("✗", Color::Red)
                } else if !r.errors.is_empty() {
                    ("!", Color::Yellow)
                } else {
                    ("✓", Color::Green)
                };
                let mut spans = vec![
                    Span::raw("  "),
                    Span::styled(icon.to_string(), Style::default().fg(color)),
                    Span::raw(format!(" {}", r.name)),
                ];
                if !r.errors.is_empty() {
                    spans.push(Span::styled(
                        format!(" ({} issues)", r.errors.len()),
                        Style::default().fg(Color::Yellow),
                    ));
                }
                lines.push(Line::from(spans));
            }
            if !done && current < meta.len() {
                let sp = SPINNER[frame % SPINNER.len()];
                let style = if opts.no_color {
                    Style::default()
                } else {
                    Style::default().fg(Color::Magenta)
                };
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(sp.to_string(), style),
                    Span::raw(format!(" {}", meta[current].0)),
                ]));
            }
            f.render_widget(Paragraph::new(lines), f.area());
        });
        frame += 1;

        // Handle Ctrl-C (80ms polling).
        if event::poll(Duration::from_millis(80)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    cancel.store(true, Ordering::SeqCst);
                    fatal = Some("interrupted".to_string());
                    done = true;
                }
            }
        }
    }

    let _ = worker.join();
    let _ = disable_raw_mode();
    // Move the cursor below the inline area with a newline so the final output is preserved.
    let _ = terminal.backend_mut();
    use std::io::Write;
    let _ = writeln!(std::io::stderr());

    if let Some(err) = fatal {
        return Ok(Err(err));
    }

    // Fill remaining step slots (not received due to cancellation etc.) with metadata.
    let steps_out: Vec<StepResult> = (0..n)
        .map(|i| {
            results[i].take().unwrap_or(StepResult {
                name: meta[i].0.clone(),
                category: meta[i].1.clone(),
                errors: Vec::new(),
                failed: false,
            })
        })
        .collect();

    Ok(Ok(RunResult {
        all_errors,
        steps: steps_out,
    }))
}
