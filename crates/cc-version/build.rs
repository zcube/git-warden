//! Captures the git commit hash and build timestamp at compile time and injects them as env vars.
//! Corresponds to Go's ldflags injection (`internal/version.Version/Commit/BuildTime`).
//! Falls back to default values (none/unknown) when git information is unavailable.

use std::process::Command;

fn main() {
    let commit = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "none".to_string());

    // Use SOURCE_DATE_EPOCH for reproducible builds if set; otherwise "unknown".
    let build_time = std::env::var("SOURCE_DATE_EPOCH").unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=CC_COMMIT={commit}");
    println!("cargo:rustc-env=CC_BUILD_TIME={build_time}");
    // Rebuild when git HEAD changes.
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
}
