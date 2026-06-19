//! cc-logger: warn/info/debug logging. Corresponds to Go `internal/logger` (charm/log).
//!
//! Default level is Warn; output goes to stderr. `set_quiet(true)` suppresses all output,
//! `set_no_color(true)` disables ANSI colour. Message format is `LEVEL msg key=val ...`.
//! Log output is diagnostic only and is separate from violation messages (check results).

use once_cell::sync::Lazy;
use std::io::{IsTerminal, Write};
use std::sync::RwLock;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
}

struct State {
    quiet: bool,
    no_color: bool,
    level: Level,
}

static STATE: Lazy<RwLock<State>> = Lazy::new(|| {
    RwLock::new(State {
        quiet: false,
        no_color: false,
        level: Level::Warn,
    })
});

/// When quiet=true, suppresses all log output.
pub fn set_quiet(quiet: bool) {
    STATE.write().unwrap().quiet = quiet;
}

/// When no_color=true, disables ANSI colour output.
pub fn set_no_color(no_color: bool) {
    STATE.write().unwrap().no_color = no_color;
}

/// Sets the minimum log level.
pub fn set_level(level: Level) {
    STATE.write().unwrap().level = level;
}

fn log(level: Level, label: &str, color: &str, msg: &str, keyvals: &[(&str, String)]) {
    let st = STATE.read().unwrap();
    if st.quiet || level < st.level {
        return;
    }
    // Like charm log, auto-disable colour when stderr is not a TTY (pipe/redirect/CI).
    let use_color = !st.no_color && std::io::stderr().is_terminal();
    let mut line = String::new();
    if use_color {
        // Coloured label (visual style similar to charm log)
        line.push_str(color);
        line.push_str(label);
        line.push_str("\x1b[0m");
    } else {
        line.push_str(label);
    }
    line.push(' ');
    line.push_str(msg);
    for (k, v) in keyvals {
        line.push(' ');
        line.push_str(k);
        line.push('=');
        line.push_str(v);
    }
    let _ = writeln!(std::io::stderr(), "{line}");
}

/// Logs a warning message.
pub fn warn(msg: &str, keyvals: &[(&str, String)]) {
    log(Level::Warn, "WARN", "\x1b[33m", msg, keyvals);
}

/// Logs an informational message.
pub fn info(msg: &str, keyvals: &[(&str, String)]) {
    log(Level::Info, "INFO", "\x1b[36m", msg, keyvals);
}

/// Logs a debug message.
pub fn debug(msg: &str, keyvals: &[(&str, String)]) {
    log(Level::Debug, "DEBU", "\x1b[35m", msg, keyvals);
}

/// Logs an error message.
pub fn error(msg: &str, keyvals: &[(&str, String)]) {
    log(Level::Error, "ERRO", "\x1b[31m", msg, keyvals);
}

/// Convenience helper for warnings with no key-value pairs.
pub fn warn_msg(msg: &str) {
    warn(msg, &[]);
}
