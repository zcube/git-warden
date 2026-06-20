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

### Version management

- **Source of truth: `Cargo.toml`** (`[workspace.package] version`) — managed by `cargo-release`.
  Do not bump by hand.
- **Bump detection: `git-cliff`** — `git-cliff --bumped-version` derives the next semver from the
  conventional commits since the last tag. The `just` release recipes feed this value to
  `cargo-release` (pass an explicit level/version to override). Install locally with
  `brew install git-cliff` (or `cargo install git-cliff`).
- **Bump rules** (conventional-commit defaults, applied by git-cliff):
  - `feat:` → minor
  - `fix:` / `perf:` / other types → patch
  - `!` suffix or `BREAKING CHANGE:` → major

### Justfile commands

```bash
just version              # show current workspace version (Cargo.toml)
just next-version         # show the next version git-cliff derives from the commit log
just check                # dry-run: see what cargo-release would do (auto-detected bump)
just check minor          # dry-run for an explicit minor bump
just release-start        # create release branch from main, auto-bump, commit, tag, push
just release-start minor  # explicit minor bump
just release-start 0.3.0  # explicit version
just publish              # publish workspace to crates.io locally (manual fallback)
just gh-publish           # trigger release-publish.yml: publish GitHub release + crates.io + FF merge release->main
just release-retry        # reset a failed release: delete draft/tag/branch, recreate from latest main
                          # blocked if GitHub release is published or crates.io already has the version
```

### Release procedure

1. Ensure `main` is green.
2. Start the release from `main`:
   ```bash
   just release-start          # auto-detect bump from the commit log (or: minor / 0.3.0)
   ```
   This computes the next version with `git-cliff --bumped-version`, creates a `release` branch, and
   runs `cargo release <version> --workspace --execute --no-publish` which:
   - Switches to the new `release` branch
   - Updates `[workspace.package] version` in `Cargo.toml`
   - Commits `"chore: release 0.2.0"`
   - Creates annotated tag `v0.2.0`
   - Pushes commit + tag to origin
3. The tag push triggers `.github/workflows/release-draft.yml`:
   - Builds cross-compiled targets, signs with cosign, generates changelog with git-cliff
   - Creates a GitHub **draft** release (no crates.io publish at this stage)
4. Review the draft release, then publish:
   ```bash
   just gh-publish
   ```
   This triggers `.github/workflows/release-publish.yml` which:
   - Marks the GitHub release as published
   - Publishes `git-warden-core` then `git-warden` to crates.io
   - Updates the Homebrew tap formula
   - Fast-forward merges `release → main` and deletes the `release` branch
5. If CI failed at step 3 (e.g. workflow file was stale on the tag):
   ```bash
   just release-retry minor   # same level as step 2
   ```
   Deletes the draft release, tag, and `release` branch, then recreates from the latest `main`.
   Blocked if the release is already published or the version is on crates.io.

### Tool roles

| Tool | Responsibility |
|---|---|
| **git-cliff** | Derives the next semver from conventional commits (`--bumped-version`); generates `CHANGELOG.md` |
| **cargo-release** | Applies the version — bumps `Cargo.toml`, commits, tags, pushes |

The build embeds only the commit hash and build time (see `crates/git-warden/build.rs`), read
directly from git — no version-tool integration is required at build time. When git is unavailable
(e.g. a crates.io install), `build.rs` falls back to defaults and the binary reports
`CARGO_PKG_VERSION` from `Cargo.toml`, which `cargo-release` stamped before publishing.
