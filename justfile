# Show the version gitversion-rs computes for the current commit
version:
    gitversion-rs -v FullSemVer

# Dry-run: show what cargo-release would do without making changes
check level="patch":
    cargo release {{level}} --workspace

# Bump version, commit Cargo.toml, create annotated tag, push
# Usage: just bump        (patch)
#        just bump minor
#        just bump major
bump level="patch":
    cargo release {{level}} --workspace --execute

# Publish all workspace crates to crates.io
# Run after `just bump` has pushed the tag
publish:
    cargo release publish --workspace --execute
