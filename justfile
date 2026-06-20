# Show the current workspace version (Cargo.toml)
version:
    @yq -p toml -oy '.workspace.package.version' Cargo.toml

# Show the next version git-cliff derives from conventional commits since the last tag
next-version:
    @git-cliff --bumped-version

# Build the workspace in release mode
build:
    cargo build --release --workspace

# Build and install to ~/.cargo/bin
install:
    cargo install --path crates/git-warden --locked

# Usage: just check  |  just check minor  |  just check 0.3.0
# Dry-run cargo-release; auto-detects the bump from the commit log unless a level/version is given.
check target="":
    #!/usr/bin/env bash
    set -euo pipefail
    TARGET="{{target}}"
    if [[ -z "${TARGET}" ]]; then
        TARGET=$(git-cliff --bumped-version)
        TARGET="${TARGET#v}"
    fi
    cargo release "${TARGET}" --workspace

# Publish all workspace crates to crates.io (manual fallback)
publish:
    cargo release publish --workspace --execute

# Trigger release-publish.yml: publish GitHub release, crates.io, FF merge release->main
gh-publish:
    #!/usr/bin/env bash
    set -euo pipefail
    VERSION=$(yq -p toml -oy '.workspace.package.version' Cargo.toml)
    gh workflow run release-publish.yml -f tag="v${VERSION}"

# Usage: just release-start  |  just release-start minor  |  just release-start 0.3.0
# Create release branch from main, auto-bump (unless given), commit, tag, push to origin.
release-start target="":
    #!/usr/bin/env bash
    set -euo pipefail
    CURRENT=$(git rev-parse --abbrev-ref HEAD)
    if [[ "${CURRENT}" != "main" ]]; then
        echo "Error: must be on main branch (currently on ${CURRENT})"
        exit 1
    fi
    if git show-ref --verify refs/heads/release >/dev/null 2>&1; then
        echo "Error: release branch already exists. Use 'just release-retry' to reset it."
        exit 1
    fi
    git pull --ff-only
    TARGET="{{target}}"
    if [[ -z "${TARGET}" ]]; then
        TARGET=$(git-cliff --bumped-version)
        TARGET="${TARGET#v}"
    fi
    git checkout -b release
    cargo release "${TARGET}" --workspace --execute --no-publish

# Usage: just release-retry  |  just release-retry minor
# Blocked if the GitHub release is published or the version is already on crates.io.
# Reset a failed release (delete draft/tag/branch, recreate from latest main); auto-bumps unless given.
release-retry target="":
    #!/usr/bin/env bash
    set -euo pipefail
    VERSION=$(yq -p toml -oy '.workspace.package.version' Cargo.toml)
    TAG="v${VERSION}"
    CRATE="git-warden"
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" -A "release-retry/1.0" \
        "https://crates.io/api/v1/crates/${CRATE}/${VERSION}")
    if [[ "${STATUS}" == "200" ]]; then
        echo "Error: ${CRATE} v${VERSION} is already on crates.io. Cannot retry."
        exit 1
    fi
    IS_DRAFT=$(gh release view "${TAG}" --json isDraft -q '.isDraft' 2>/dev/null || echo "none")
    if [[ "${IS_DRAFT}" == "false" ]]; then
        echo "Error: ${TAG} is already published. Cannot retry."
        exit 1
    fi
    if [[ "${IS_DRAFT}" == "true" ]]; then
        gh release delete "${TAG}" --yes --cleanup-tag
    fi
    git tag -d "${TAG}" 2>/dev/null || true
    git push origin --delete "${TAG}" 2>/dev/null || true
    git push origin --delete release 2>/dev/null || true
    git checkout main
    git pull --ff-only
    git branch -D release 2>/dev/null || true
    TARGET="{{target}}"
    if [[ -z "${TARGET}" ]]; then
        TARGET=$(git-cliff --bumped-version)
        TARGET="${TARGET#v}"
    fi
    git checkout -b release
    cargo release "${TARGET}" --workspace --execute --no-publish
