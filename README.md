[English](./README.md) | [한국어](./README.ko.md) | [日本語](./README.ja.md) | [中文](./README.zh.md)

# git-warden

A CLI tool that automatically enforces policies on Git commit messages and source code.
Works with [lefthook](https://github.com/evilmartians/lefthook), husky, or any Git hook manager.

## Features

| Check | Description |
|---|---|
| **Comment language** | Verify comments are written in the required language (Korean/English/Japanese/Chinese) |
| **Allowed words** | Register technical terms and proper nouns to prevent false positives |
| **Co-authored-by** | Block AI co-author trailers (with email allow-list support) |
| **Unicode spaces** | Block invisible/non-standard Unicode space characters (NBSP, EM SPACE, ZWSP, BiDi, etc.) |
| **Ambiguous chars** | Block Unicode characters that look like ASCII (e.g., Cyrillic A vs Latin A) |
| **File Unicode check** | Detect invisible/ambiguous Unicode characters in source and markdown files |
| **Invalid UTF-8** | Block invalid byte sequences |
| **Emoji ban** | Block emojis in commit messages and comments (optional) |
| **Binary file policy** | Per-extension block / allow / lfs policy (images allowed by default, git-LFS verification) |
| **Encoding check** | Block non-UTF-8 encoded files (chardet-based) |
| **Data file lint** | YAML, JSON (with JSON5/JSONC support), XML syntax validation |
| **EditorConfig** | Validate files against .editorconfig rules |
| **Conventional Commits** | Enforce commit message format (optional) |
| **Append-only paths** | Block file deletion, content modification, and mid-file insertion (e.g. DB migrations) |
| **Cache / build dirs** | Block commits inside node_modules, dist, build, target, __pycache__, .venv, etc. |
| **clean command** | Remove untracked files inside cache/build dirs (tracked files preserved) |
| **Repository analysis** | Detect development languages and warn about missing lint configs |
| **Auto-fix** | Batch-fix unicode/encoding violations across git history |
| **Config migration** | Auto-detect old config versions and migrate to the latest schema |
| **Progress indicator** | ratatui TUI spinner (TTY-aware, plain text fallback) |

## Installation

### Binary download

Download the file for your platform from [GitHub Releases](https://github.com/zcube/git-warden/releases).

```bash
# Linux (amd64)
curl -L https://github.com/zcube/git-warden/releases/download/<TAG>/git-warden_<TAG>_x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv git-warden /usr/local/bin/

# Linux (arm64)
curl -L https://github.com/zcube/git-warden/releases/download/<TAG>/git-warden_<TAG>_aarch64-unknown-linux-gnu.tar.gz | tar xz
sudo mv git-warden /usr/local/bin/

# macOS (Apple Silicon)
curl -L https://github.com/zcube/git-warden/releases/download/<TAG>/git-warden_<TAG>_aarch64-apple-darwin.tar.gz | tar xz
sudo mv git-warden /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/zcube/git-warden/releases/download/<TAG>/git-warden_<TAG>_x86_64-apple-darwin.tar.gz | tar xz
sudo mv git-warden /usr/local/bin/
```

Replace `<TAG>` with the latest release tag (e.g. `v0.1.0`). Check the [Releases](https://github.com/zcube/git-warden/releases) page for the latest version.

### Build from source

Requires Rust 1.88+.

```bash
cargo install --git https://github.com/zcube/git-warden --bin git-warden
```

Verify with `git-warden version`.

## Git Hook Integration (lefthook)

### 1. Install lefthook

```bash
# macOS
brew install lefthook

# npm
npm install --save-dev lefthook

# go install
go install github.com/evilmartians/lefthook@latest
```

### 2. Install git-warden

Download from [GitHub Releases](https://github.com/zcube/git-warden/releases) or build from source.

### 3. Create lefthook.yml

Create `lefthook.yml` in your project root:

```yaml
pre-commit:
  commands:
    git-warden:
      run: git-warden diff

commit-msg:
  commands:
    message-policy:
      run: git-warden msg {1}
```

### 4. Install hooks

```bash
lefthook install
```

Checks run automatically on every `git commit`.

### Optional hooks (add only what you need)

#### Auto-fix before checking (fix)

`fix` re-stages the files it modifies via `git add`. Replace the `pre-commit` block with the one below (lefthook runs commands in name order, so `auto-fix` runs before `git-warden`):

```yaml
pre-commit:
  commands:
    auto-fix:
      run: git-warden fix
      stage_fixed: true
    git-warden:
      run: git-warden diff
```

#### Prevent merge bypass (pre-merge-commit)

Merge commits do not trigger the pre-commit hook. Register the check here too:

```yaml
pre-merge-commit:
  commands:
    git-warden:
      run: git-warden diff
```

#### Commit message policy hints (prepare-commit-msg)

Shows active policy hints as `#` comments in the commit message editor. Use `{0}` (not `{1}`):

```yaml
prepare-commit-msg:
  commands:
    policy-hint:
      run: git-warden prepare-msg {0}
```

#### Check commit messages before push (pre-push)

```yaml
pre-push:
  commands:
    check-commits:
      run: git-warden push
```

### 5. Check all existing files (initial adoption)

When adopting git-warden in an existing repository, run a one-time check over all tracked files:

```bash
git-warden run
```

To fix violations automatically:

```bash
git-warden fix --dry-run   # preview
git-warden fix             # apply
```

### husky (Node.js projects)

```bash
npx husky init
```

`.husky/pre-commit`:
```bash
#!/bin/sh
git-warden diff
```

`.husky/commit-msg`:
```bash
#!/bin/sh
git-warden msg "$1"
```

### Git 2.54+ config-based hooks (no hook manager)

```bash
# Base: check staged changes (pre-commit)
git config set hook.git-warden-diff.command "git-warden diff"
git config set --append hook.git-warden-diff.event pre-commit

# Base: check commit messages (commit-msg)
git config set hook.git-warden-msg.command "git-warden msg"
git config set --append hook.git-warden-msg.event commit-msg

# Optional: check before push (pre-push)
git config set hook.git-warden-push.command "git-warden push"
git config set --append hook.git-warden-push.event pre-push

# Optional: check merge commits (pre-merge-commit)
git config set hook.git-warden-merge.command "git-warden diff"
git config set --append hook.git-warden-merge.event pre-merge-commit

# Optional: policy hints in commit message editor (prepare-commit-msg)
git config set hook.git-warden-prepare.command "git-warden prepare-msg"
git config set --append hook.git-warden-prepare.event prepare-commit-msg
```

Add `--global` to apply to every repository. Verify with `git hook list pre-commit`.

### Other hook integrations

#### git am workflow

```bash
git config set hook.git-warden-am-msg.command "git-warden msg"
git config set --append hook.git-warden-am-msg.event applypatch-msg

git config set hook.git-warden-am-diff.command "git-warden diff"
git config set --append hook.git-warden-am-diff.event pre-applypatch
```

#### Server-side enforcement (update hook)

```bash
#!/bin/sh
# hooks/update — arguments: <refname> <old> <new>
exec git-warden push --range "$2..$3"
```

New branches (old is all zeros) print a warning and are skipped.

## Global Installation

### Global hooks + global config

```bash
git config set --global hook.git-warden-diff.command "git-warden diff"
git config set --global --append hook.git-warden-diff.event pre-commit
git config set --global hook.git-warden-msg.command "git-warden msg"
git config set --global --append hook.git-warden-msg.event commit-msg
```

Global config file resolution order (first found is used):

| Order | Location |
|---|---|
| 1 | `$GIT_WARDEN_GLOBAL_CONFIG` (explicit; ignored with warning if file missing) |
| 2 | `$XDG_CONFIG_HOME/git-warden/config.yaml` (`config.yml` also supported) |
| 3 | OS config dir — Linux `~/.config/git-warden/config.yaml`, macOS `~/Library/Application Support/git-warden/config.yaml`, Windows `%AppData%\git-warden\config.yaml` |
| 4 | `$HOME/.config/git-warden/config.yaml` (`config.yml` also supported) |
| 5 | `~/.git-warden.yml` (legacy) |

```yaml
# Global config example
# macOS: ~/Library/Application Support/git-warden/config.yml
# Linux: ~/.config/git-warden/config.yml
commit_message:
  no_ai_coauthor: true
  no_unicode_spaces: true
  no_ambiguous_chars: true
  no_bad_runes: true
  no_emoji: true
  locale: en

  conventional_commit:
    enabled: true
    locale: en

  language_check:
    enabled: true
    locale: en
```

### Per-directory policies (gitdir include)

```yaml
# ~/.config/git-warden/config.yaml
include:
  - path: ~/.config/git-warden/base.yml
  - path: ~/.config/git-warden/work.yml
    gitdir: ~/work/
comment_language:
  locale: en
```

- Precedence: main body > later includes > earlier includes.
- `gitdir`: `~` expands to home directory; trailing `/` matches the entire subtree.
- Available in both global and project configs. Nested includes and remote preset includes are ignored for security.

### Per-repository control (override · opt-out · opt-in)

**override** — a repository `.git-warden.yml` or `.git-warden.yaml` overrides the global config entirely.

**opt-out** — disable all checks in a specific repository:

```yaml
enabled: false
```

**opt-in** — only check repositories that have a project config file:

```bash
git config set --global hook.git-warden-diff.command "git-warden diff --require-config"
git config set --global hook.git-warden-msg.command "git-warden msg --require-config"
```

## Configuration

Create `.git-warden.yml` in your project root. Run `git-warden init` to generate a default config.
Use `.git-warden.schema.json` for IDE autocompletion in VS Code.

```yaml
# yaml-language-server: $schema=./.git-warden.schema.json

comment_language:
  enabled: true
  required_language: english  # korean | english | japanese | chinese | any
  min_length: 5
  check_mode: diff            # diff | full
  no_emoji: false
  extensions:
    - .go
    - .ts
    - .py
    - .tf

  allowed_words:
    - TypeScript
    - JavaScript
    - API
  # allowed_words_file: .git-warden-words.txt
  # allowed_words_url: https://example.com/allowed-words.txt
  # allowed_words_cache:
  #   enabled: true
  #   ttl: 24h

binary_file:
  enabled: true
  # default_policy: block
  # rules:
  #   - extensions: [.psd, .ai]
  #     policy: lfs
  # ignore_files:
  #   - "**/*.png"

lint:
  enabled: true
  yaml:
    enabled: true
  json:
    enabled: true
  xml:
    enabled: true

encoding:
  enabled: true
  require_utf8: true
  # no_invisible_chars: true
  # no_ambiguous_chars: true

editorconfig:
  enabled: true

commit_message:
  no_ai_coauthor: true
  no_unicode_spaces: true
  no_ambiguous_chars: true
  no_bad_runes: true
  no_emoji: false
  locale: en
  conventional_commit:
    enabled: false
  language_check:
    enabled: false
    required_language: english

append_only:
  enabled: false
  # paths:
  #   - "migrations/**"

# protected_paths:
#   enabled: true
#   paths:
#     - "legacy/**"

cache_dir:
  enabled: true
  # ignore_dirs:
  #   - vendor

# guide:
#   enabled: false
```

### Binary file policy

| Policy | Behaviour |
|---|---|
| `block` | Reject (default) |
| `allow` | Accept |
| `lfs` | Accept only when tracked by git LFS (checks `filter=lfs` in `.gitattributes`) |

Built-in image extensions (`.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.bmp`, `.ico`, `.tiff`, `.tif`, `.heic`, `.heif`, `.avif`) default to **`allow`** when no rule matches.

```yaml
binary_file:
  enabled: true
  default_policy: block
  rules:
    - extensions: [.png, .jpg, .jpeg, .gif, .webp]
      policy: lfs
    - extensions: [.psd, .ai, .sketch]
      policy: lfs
    - extensions: [.mp4, .mov, .webm]
      policy: lfs
  ignore_files:
    - "assets/icons/**"
```

Resolution order: `rules` match → built-in image (`allow`) → `default_policy` (or `block`).

### Data file lint

`.jsonc` files are always checked in JSON5 mode. Use `# git-warden: skip-lint` to disable linting for a file.

```yaml
lint:
  enabled: true
  yaml:
    enabled: true
    comment_filter: true   # skip files containing "# git-warden: skip-lint"
  json:
    enabled: true
    comment_filter: true   # strip // and /* */ comments before validating
  xml:
    enabled: true
```

### Append-only paths

```yaml
append_only:
  enabled: true
  paths:
    - "migrations/**"
    - "db/migrations/**"
  # filename_order: none   # disable numeric filename order check
```

Allowed: adding new files (sorting after existing files), appending at the end of a file.
Blocked: deleting files, modifying existing lines, inserting content in the middle.

### protected_paths

Full freeze — blocks every staged change (add, modify, delete):

```yaml
protected_paths:
  enabled: true
  paths:
    - "legacy/**"
```

| Check | Allowed changes |
|---|---|
| `append_only` | Adding new files, appending at end of existing files |
| `protected_paths` | None (full freeze) |

### Build artifact / cache directories

```yaml
cache_dir:
  enabled: true
  ignore_dirs:
    - vendor
```

Supported directories: `node_modules`, `dist`, `out`, `build`, `target`, `vendor`, `.gradle`, `.next`, `.nuxt`, `.output`, `.svelte-kit`, `.yarn`, `.bun`, `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, `.turbo`, `.parcel-cache`, `.venv` (+pyvenv), `.tox`, `.nox`, `.embuild`, `.dart_tool`.

#### clean command

```bash
git-warden clean         # list untracked files (dry-run)
git-warden clean --yes   # delete them
```

Tracked files are never deleted.

### Allowed words dictionary

```yaml
comment_language:
  allowed_words:
    - TypeScript
    - API
    - URL
  allowed_words_file: .git-warden-words.txt
  allowed_words_url: https://example.com/allowed-words.txt
  allowed_words_cache:
    enabled: true
    ttl: 24h
```

All three sources are merged.

### Per-file language rules

```yaml
comment_language:
  required_language: english
  file_languages:
    - pattern: "locales/**"
      language: any
    - pattern: "i18n/**"
      language: english
    - pattern: "locale/ja/**"
      language: ja
```

### In-source directives

```go
// git-warden:ignore
// This comment is intentional (next comment only)

// git-warden:file-lang=english

// git-warden:disable:lang=english
// Intentional English block
// git-warden:enable
```

| Directive | Description |
|---|---|
| `git-warden:ignore` | Skip the next comment only |
| `git-warden:disable` | Disable checking from this line |
| `git-warden:disable:lang=<L>` | Disable and use language L for this region |
| `git-warden:enable` | Re-enable checking |
| `git-warden:lang=<L>` | Switch required language from this point |
| `git-warden:file-lang=<L>` | Set required language for the entire file |

`<L>`: `korean` `english` `japanese` `chinese` `any` (or `ko` `en` `ja` `zh`)

### Remediation guide

When a check fails, a per-category fix guide is printed after the violation list. Disable:

```yaml
guide:
  enabled: false
```

Use `--no-guide` flag to disable regardless of config. With `--format json`, guides appear as `"guides": {"<category>": "<text>"}`.

## Commands

```
git-warden init          Generate default config file (.git-warden.yml)
git-warden diff          Check staged diff (comments/encoding/lint/binary/unicode)
git-warden run           Check all tracked files for policy compliance
git-warden msg <file>    Check commit message file
git-warden prepare-msg   For prepare-commit-msg hook: show policy hints in the editor
git-warden fix           Auto-fix git history (supports --dry-run)
git-warden migrate       Migrate config file to the latest schema
git-warden analyze       Analyze repository (language detection, lint config check)
git-warden clean         Remove untracked files inside cache/build directories
git-warden version       Print version info
```

### diff command

```bash
git-warden diff                    # default: staged (pre-commit)
git-warden diff --staged           # explicit (alias: --cached)
git-warden diff HEAD               # HEAD ↔ working tree
git-warden diff origin/main        # origin/main ↔ working tree
git-warden diff A B                # A ↔ B
git-warden diff A..B               # range
git-warden diff A...B              # merge-base(A,B) ↔ B
```

`--only` flag runs specific checks only (even if disabled in config):

```bash
git-warden diff --only comment_language
git-warden diff --only lint,encoding
```

Categories: `binary` `encoding` `unicode` `lint` `editorconfig` `comment_language` `cache_dir` (diff-only: `custom_rules` `append_only` `protected_paths`)

CI usage:

```yaml
# GitHub Actions
- run: git-warden diff ${{ github.event.pull_request.base.sha }}..HEAD

# GitLab CI
- git-warden diff ${CI_MERGE_REQUEST_DIFF_BASE_SHA}..HEAD
```

### init command

```bash
git-warden init             # auto-detect locale
git-warden init --lang en   # specific locale
git-warden init --force     # overwrite existing
```

### run command

```bash
git-warden run              # check all tracked files
git-warden run --only lint  # specific checks only
```

### prepare-msg command

Shows active policy hints as `#` comments in the commit message editor (no-op for `-m`/merge/squash/amend).

```bash
git-warden prepare-msg .git/COMMIT_EDITMSG
```

### fix command

```bash
git-warden fix --dry-run
git-warden fix --range HEAD~5..HEAD
git-warden fix --mine --dry-run
```

### migrate command

```bash
git-warden migrate
git-warden migrate --dry-run
```

Migrates old config files to the latest schema. Comments and formatting are preserved.

### analyze command

```bash
git-warden analyze
```

Detects development languages and warns when lint config files (`.golangci.yml`, `.eslintrc.*`, `pyproject.toml`, etc.) are missing. Also checks for `.editorconfig`, `.gitattributes`, `.gitignore`.

## Supported Languages

| Language | Extensions |
|---|---|
| Go | `.go` |
| TypeScript | `.ts` `.tsx` |
| JavaScript | `.js` `.jsx` `.mjs` `.cjs` |
| Java | `.java` |
| Kotlin | `.kt` `.kts` |
| Python | `.py` |
| C / C++ | `.c` `.h` `.cpp` `.cc` `.hpp` |
| C# | `.cs` |
| Swift | `.swift` |
| Rust | `.rs` |
| Dockerfile | `Dockerfile` `Dockerfile.*` `*.dockerfile` |
| Markdown | `.md` `.markdown` |
| HCL (Terraform) | `.hcl` `.tf` `.tfvars` |

## i18n Support

CLI output is available in Korean (ko), English (en), Japanese (ja), Chinese (zh).

Set via `GIT_WARDEN_LANG`, `LC_ALL`, `LC_MESSAGES`, `LANG` environment variables, or the `locale` config field.

## License

MIT
