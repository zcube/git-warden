# AGENTS.md

Guide for agents/contributors working in this repository. For a user-facing overview see the
[README](README.en.md).

## Project overview

A CLI tool that enforces commit message and source-code policies via Git hooks. Designed to work
with [lefthook](https://github.com/evilmartians/lefthook), husky, and similar Git hook managers.

## Development workflow

```bash
cargo build                  # build
cargo test                   # unit + integration tests
cargo fmt --all              # format
cargo clippy --all-targets --all-features -- -D warnings  # lint (zero warnings)
git-warden run               # run all checks against the working tree
```

- **lefthook** (install: `lefthook install`)
  - pre-commit: `cargo fmt` (auto-format + re-stage), `clippy -D warnings`, `git-warden diff`
  - commit-msg: `git-warden msg` (language + policy check)
  - prepare-commit-msg: `git-warden prepare-msg`
  - pre-push: `cargo test`
- **CI** (`.github/workflows/ci.yml`): fmt --check, clippy -D warnings, build/test. Runs on pushes
  to main and PRs.

## Commit conventions

- **Conventional Commits** type prefix required: `feat|fix|ci|chore|test|docs|refactor|perf|style|build|revert`.
- Commit messages must be written in **English** (enforced by `git-warden` via the commit-msg hook).
- **No AI co-author trailers** (e.g. Co-Authored-By), no arrows (`→`), no emoji.
- For throwaway test repos, `git commit --no-verify` is fine.

## Workspace layout

| Crate | Role |
|---|---|
| `crates/git-warden` | CLI binary — argument parsing, TUI, version metadata |
| `crates/git-warden-core` | Reusable library — all checker, config, and analysis logic |

The split criterion: code that can be meaningfully reused by other applications goes into
`git-warden-core`; everything else stays in the binary crate.

## Versioning and releases

### Source of truth

`Cargo.toml` (`[workspace.package] version`) is the single source of truth.
Use **cargo-release** to bump the version; do not edit it by hand.

```bash
cargo release minor   # 0.0.x → 0.1.0: updates Cargo.toml, commits, tags v0.1.0
```

### Release procedure

1. Ensure `main` is green.
2. Run `cargo release minor` (or `patch` / `major`) locally. This:
   - Updates `[workspace.package] version` in `Cargo.toml`
   - Commits `"chore: release v0.1.0"`
   - Creates annotated tag `v0.1.0`
   - Pushes commit + tag
3. The tag push triggers `.github/workflows/release-draft.yml`:
   - Builds 6 cross-compiled targets, signs with cosign, generates changelog with git-cliff.
4. Trigger `.github/workflows/release-publish.yml` (manual dispatch, input: tag):
   - Publishes `git-warden-core` then `git-warden` to crates.io via `cargo workspaces publish`.
   - Updates the Homebrew tap formula.

## Recommended gitversion-rs integration pattern

When integrating gitversion-rs into a Rust project, use the following pattern. This is also how
git-warden itself should be built when gitversion-rs is available.

### Tool roles

| Tool | Responsibility |
|---|---|
| **cargo-release** | Version number management — bumps `Cargo.toml`, commits, tags |
| **gitversion-rs `--exec`** | Build-time metadata injection — `PreReleaseTag`, `InformationalVersion` |

### What `gitversion-rs --exec` provides

Running `gitversion-rs --exec "cargo build"` sets these env vars in the child process:

```
CARGO_PKG_VERSION_PRE   = PreReleaseTag   # "" on release tag, "5" on untagged dev commit
GitVersion_InformationalVersion           # "0.1.0+Branch.main.Sha.abc1234"
CARGO_PKG_VERSION       = SemVer          # overrides Cargo.toml value at build time
```

`CARGO_PKG_VERSION_PRE` is the key variable: it carries the `PreReleaseTag` without any code
change or dirty state. Release builds get `""`, dev builds get commit distance, pre-release
branches get the configured tag (e.g. `"alpha.1"`).

### build.rs pattern

```rust
fn main() {
    // gitversion-rs --exec sets this; fall back to CARGO_PKG_VERSION when git is absent.
    let info = std::env::var("GitVersion_InformationalVersion")
        .unwrap_or_else(|_| std::env::var("CARGO_PKG_VERSION").unwrap_or_default());

    println!("cargo:rustc-env=APP_INFO_VERSION={info}");
    println!("cargo:rerun-if-env-changed=GitVersion_InformationalVersion");
}
```

In source files, `CARGO_PKG_VERSION_PRE` is available without a build.rs:

```rust
const PRE: &str = env!("CARGO_PKG_VERSION_PRE");  // "" / "5" / "alpha.1"
```

### Build command

```bash
gitversion-rs --exec "cargo build --release"
```

No `--allow-dirty` and no separate injection step needed. `cargo-release` has already written the
correct version into `Cargo.toml`; gitversion-rs adds the git-derived metadata on top.

### Fallback (no git / crates.io install)

When `GitVersion_*` vars are absent, `build.rs` falls back to `CARGO_PKG_VERSION` from
`Cargo.toml`. Because `cargo-release` stamped the correct version before publishing, the installed
binary always reports the right version.
