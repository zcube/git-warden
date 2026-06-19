//! cc-version: provides version information injected at build time. Corresponds to Go `internal/version`.

/// Release version. Uses the Cargo package version (corresponds to ldflags Version).
pub fn version() -> &'static str {
    let v = env!("CARGO_PKG_VERSION");
    if v.is_empty() {
        "dev"
    } else {
        v
    }
}

/// Git commit hash at build time. Returns "none" if unavailable.
pub fn commit() -> &'static str {
    env!("CC_COMMIT")
}

/// Build timestamp. Returns "unknown" if unavailable.
pub fn build_time() -> &'static str {
    env!("CC_BUILD_TIME")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert!(!version().is_empty());
        assert!(!commit().is_empty());
        assert!(!build_time().is_empty());
    }
}
