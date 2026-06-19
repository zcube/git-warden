[한국어](./README.md) | [English](./README.en.md) | [日本語](./README.ja.md) | [中文](./README.zh.md)

# git-warden

Git 커밋 메시지와 소스 코드의 정책을 자동으로 검사하는 CLI 도구입니다.
[lefthook](https://github.com/evilmartians/lefthook) / husky 등 Git 훅 매니저와 함께 사용합니다.

## 기능

| 검사 항목 | 설명 |
|---|---|
| **주석 언어** | 지정된 언어(한국어/영어/일본어/중국어)로 작성된 주석인지 검사 |
| **허용 단어 사전** | 기술 용어·고유명사를 허용 단어로 등록하여 오탐 방지 |
| **Co-authored-by** | AI 기여 표시 트레일러 차단 (이메일 허용 목록 지원) |
| **비표준 유니코드 공백** | NBSP, EM SPACE, ZWSP, BiDi 제어문자 등 차단 |
| **모호한 유니코드 문자** | 키릴 А ↔ 라틴 A 등 시각적 혼동 문자 차단 |
| **파일 유니코드 검사** | 소스/마크다운 파일 내용에서 비가시·모호한 유니코드 문자 검사 |
| **잘못된 UTF-8** | 잘못된 바이트 시퀀스 차단 |
| **이모지 금지** | 커밋 메시지 및 주석에서 이모지 사용 차단 (선택적) |
| **바이너리 파일 정책** | 확장자별 block / allow / lfs 정책 (이미지 기본 허가, git LFS 검증 지원) |
| **인코딩 검사** | UTF-8이 아닌 파일 커밋 차단 (chardet 기반) |
| **데이터 파일 린트** | YAML, JSON (JSON5/JSONC 지원), XML 구문 검사 |
| **EditorConfig** | .editorconfig 규칙 준수 여부 검사 |
| **Conventional Commits** | 커밋 메시지 형식 강제 (선택적) |
| **append-only 경로** | 지정 경로에서 파일 삭제·내용 수정·중간 삽입 차단 (DB 마이그레이션 등) |
| **빌드 산출물·캐시 디렉터리** | node_modules, dist, build, target, __pycache__, .venv 등의 커밋 차단 |
| **clean 명령** | 미추적 캐시/빌드 파일 정리 (git 추적 파일은 보존) |
| **리포지터리 분석** | 개발 언어 감지 및 린트 설정 누락 경고 |
| **자동 수정 (fix)** | 유니코드/인코딩 위반 사항을 git history에서 일괄 수정 |
| **설정 마이그레이션** | 구 버전 설정 파일을 자동 감지하여 최신 스키마로 변환 |
| **진행 표시기** | ratatui 기반 TUI 스피너 (TTY 감지, 비TTY 시 텍스트 폴백) |

## 설치

### 바이너리 직접 다운로드

[GitHub Releases](https://github.com/zcube/git-warden/releases) 페이지에서 플랫폼에 맞는 파일을 다운로드합니다.

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

`<TAG>`는 최신 릴리스 태그(예: `v0.1.0`)로 교체합니다.

### 소스에서 빌드 (Rust 1.88+)

```bash
cargo install --git https://github.com/zcube/git-warden --bin git-warden
```

설치 후 `git-warden version`으로 확인합니다.

## Git 훅 연동 (lefthook)

### 1. lefthook 설치

```bash
# macOS
brew install lefthook

# npm
npm install --save-dev lefthook

# go install
go install github.com/evilmartians/lefthook@latest
```

### 2. git-warden 설치

[GitHub Releases](https://github.com/zcube/git-warden/releases)에서 다운로드하거나 소스에서 빌드합니다.

### 3. lefthook.yml 작성

프로젝트 루트에 `lefthook.yml`을 생성합니다:

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

### 4. 훅 설치

```bash
lefthook install
```

이후 `git commit`마다 자동으로 검사가 실행됩니다.

### 선택적 훅 (필요한 것만 추가)

#### 자동 수정 (fix)

`fix`는 수정한 파일을 `git add`로 자동 재스테이징합니다. 아래 블록으로 `pre-commit`을 교체합니다 (lefthook은 이름 순으로 실행하므로 `auto-fix`가 먼저 실행됩니다):

```yaml
pre-commit:
  commands:
    auto-fix:
      run: git-warden fix
      stage_fixed: true
    git-warden:
      run: git-warden diff
```

#### merge 우회 방지 (pre-merge-commit)

merge 커밋은 pre-commit 훅을 트리거하지 않으므로 별도로 등록합니다:

```yaml
pre-merge-commit:
  commands:
    git-warden:
      run: git-warden diff
```

#### 커밋 메시지 정책 힌트 (prepare-commit-msg)

커밋 메시지 편집기 하단에 `#` 주석으로 현재 정책 힌트를 표시합니다. `{0}` 사용 필수:

```yaml
prepare-commit-msg:
  commands:
    policy-hint:
      run: git-warden prepare-msg {0}
```

#### 푸시 전 커밋 메시지 검사 (pre-push)

```yaml
pre-push:
  commands:
    check-commits:
      run: git-warden push
```

### 5. 기존 파일 검사 (최초 도입 시)

훅 설치 전에 커밋된 파일을 일괄 검사하려면 `run` 명령을 사용합니다:

```bash
git-warden run
```

위반 사항을 자동 수정하려면:

```bash
git-warden fix --dry-run   # 미리보기
git-warden fix             # 실제 수정
```

### husky (Node.js 프로젝트)

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

### Git 2.54+ 설정 기반 훅 (훅 매니저 불필요)

```bash
# 기본: staged 변경사항 검사 (pre-commit)
git config set hook.git-warden-diff.command "git-warden diff"
git config set --append hook.git-warden-diff.event pre-commit

# 기본: 커밋 메시지 검사 (commit-msg)
git config set hook.git-warden-msg.command "git-warden msg"
git config set --append hook.git-warden-msg.event commit-msg

# 선택: 푸시 전 커밋 검사 (pre-push)
git config set hook.git-warden-push.command "git-warden push"
git config set --append hook.git-warden-push.event pre-push

# 선택: merge 커밋 검사 (pre-merge-commit)
git config set hook.git-warden-merge.command "git-warden diff"
git config set --append hook.git-warden-merge.event pre-merge-commit

# 선택: 커밋 메시지 편집기 정책 힌트 (prepare-commit-msg)
git config set hook.git-warden-prepare.command "git-warden prepare-msg"
git config set --append hook.git-warden-prepare.event prepare-commit-msg
```

`--global`을 추가하면 모든 저장소에 적용됩니다. `git hook list pre-commit`으로 등록을 확인합니다.

### 기타 훅 연동

#### git am 워크플로

```bash
git config set hook.git-warden-am-msg.command "git-warden msg"
git config set --append hook.git-warden-am-msg.event applypatch-msg

git config set hook.git-warden-am-diff.command "git-warden diff"
git config set --append hook.git-warden-am-diff.event pre-applypatch
```

#### 서버 사이드 강제 (update 훅)

```bash
#!/bin/sh
# hooks/update — 인수: <refname> <old> <new>
exec git-warden push --range "$2..$3"
```

신규 브랜치(old가 전부 0인 경우)는 경고를 출력하고 건너뜁니다.

## 전역 설치

### 전역 훅 + 전역 설정

```bash
git config set --global hook.git-warden-diff.command "git-warden diff"
git config set --global --append hook.git-warden-diff.event pre-commit
git config set --global hook.git-warden-msg.command "git-warden msg"
git config set --global --append hook.git-warden-msg.event commit-msg
```

전역 설정 파일 탐색 순서 (첫 번째로 존재하는 파일을 사용):

| 순서 | 위치 |
|---|---|
| 1 | `$GIT_WARDEN_GLOBAL_CONFIG` (명시적, 파일 없으면 경고 후 무시) |
| 2 | `$XDG_CONFIG_HOME/git-warden/config.yaml` (`config.yml` 지원) |
| 3 | OS 표준 설정 디렉터리 — Linux `~/.config/git-warden/config.yaml`, macOS `~/Library/Application Support/git-warden/config.yaml`, Windows `%AppData%\git-warden\config.yaml` |
| 4 | `$HOME/.config/git-warden/config.yaml` (`config.yml` 지원) |
| 5 | `~/.git-warden.yml` (레거시) |

```yaml
# 전역 설정 예시
# macOS: ~/Library/Application Support/git-warden/config.yml
# Linux: ~/.config/git-warden/config.yml
commit_message:
  no_ai_coauthor: true
  no_unicode_spaces: true
  no_ambiguous_chars: true
  no_bad_runes: true
  no_emoji: true
  locale: ko

  conventional_commit:
    enabled: true
    locale: en

  language_check:
    enabled: true
    locale: ko
```

### 디렉터리별 정책 (gitdir include)

```yaml
# ~/.config/git-warden/config.yaml
include:
  - path: ~/.config/git-warden/base.yml
  - path: ~/.config/git-warden/work.yml
    gitdir: ~/work/
comment_language:
  locale: ko
```

- 우선순위: 본문 > 나중 include > 앞 include
- `gitdir`: `~`는 홈 디렉터리로 확장, 끝의 `/`는 전체 하위 트리와 일치

### 저장소별 제어 (override · opt-out · opt-in)

**override** — 저장소에 `.git-warden.yml` 또는 `.git-warden.yaml`이 있으면 전역 설정은 완전히 무시됩니다.

**opt-out** — 특정 저장소의 모든 검사를 비활성화:

```yaml
enabled: false
```

**opt-in** — 설정 파일이 있는 저장소만 검사:

```bash
git config set --global hook.git-warden-diff.command "git-warden diff --require-config"
git config set --global hook.git-warden-msg.command "git-warden msg --require-config"
```

## 설정

프로젝트 루트에 `.git-warden.yml`을 작성합니다. `git-warden init`으로 기본 설정 파일을 생성할 수 있습니다.
VS Code에서 `.git-warden.schema.json`을 사용하면 자동완성이 지원됩니다.

```yaml
# yaml-language-server: $schema=./.git-warden.schema.json

comment_language:
  enabled: true
  required_language: korean   # korean | english | japanese | chinese | any
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

editorconfig:
  enabled: true

commit_message:
  no_ai_coauthor: true
  no_unicode_spaces: true
  no_ambiguous_chars: true
  no_bad_runes: true
  no_emoji: false
  locale: ko
  conventional_commit:
    enabled: false
  language_check:
    enabled: false
    required_language: korean

append_only:
  enabled: false
  # paths:
  #   - "migrations/**"

cache_dir:
  enabled: true
```

설정 파일이 없으면 기본값이 적용됩니다.

### 바이너리 파일 정책

| 정책 | 동작 |
|---|---|
| `block` | 거부 (기본) |
| `allow` | 허용 |
| `lfs` | git LFS로 추적되는 경우에만 허용 |

내장 이미지 확장자(`.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.bmp`, `.ico`, `.tiff`, `.tif`, `.heic`, `.heif`, `.avif`)는 별도 규칙이 없으면 **`allow`**가 적용됩니다.

### 데이터 파일 린트

`.jsonc` 확장자 파일은 항상 JSON5 모드로 검사됩니다. `# git-warden: skip-lint` 주석으로 파일별 검사를 비활성화할 수 있습니다.

```yaml
lint:
  enabled: true
  yaml:
    enabled: true
    comment_filter: true   # git-warden: skip-lint 주석 지원
  json:
    enabled: true
    comment_filter: true   # .json 파일에서 주석 제거 후 검사
  xml:
    enabled: true
```

### append-only 경로

```yaml
append_only:
  enabled: true
  paths:
    - "migrations/**"
    - "db/migrations/**"
```

허용: 새 파일 추가(기존 파일 이후 정렬), 기존 파일 끝에 내용 추가.
차단: 파일 삭제, 기존 줄 수정/삭제, 중간 삽입.

### protected_paths (보호 경로)

모든 staged 변경을 차단하는 완전 동결 정책입니다:

```yaml
protected_paths:
  enabled: true
  paths:
    - "legacy/**"
```

### 빌드 산출물·캐시 디렉터리

```yaml
cache_dir:
  enabled: true
  ignore_dirs:
    - vendor
```

#### clean 명령

```bash
git-warden clean         # 발견된 항목 목록 (미리보기)
git-warden clean --yes   # 실제 삭제
```

### 허용 단어 사전

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

세 가지 소스(인라인, 파일, URL)가 합산됩니다.

### 파일별 언어 규칙

```yaml
comment_language:
  required_language: korean
  file_languages:
    - pattern: "locales/**"
      language: any
    - pattern: "locale/en/**"
      language: english
```

### 인소스 지시자

```go
// git-warden:ignore
// 이 주석은 의도적입니다 (다음 주석 하나만 적용)

// git-warden:file-lang=korean

// git-warden:disable:lang=english
// Intentional English block
// git-warden:enable
```

| 지시자 | 설명 |
|---|---|
| `git-warden:ignore` | 다음 주석 하나만 건너뜀 |
| `git-warden:disable` | 이 줄부터 검사 비활성화 |
| `git-warden:disable:lang=<L>` | 비활성화하고 이 구간에 언어 L 적용 |
| `git-warden:enable` | 검사 재활성화 |
| `git-warden:lang=<L>` | 이 줄부터 요구 언어를 L로 전환 |
| `git-warden:file-lang=<L>` | 파일 전체의 요구 언어를 L로 설정 |

`<L>` 값: `korean` `english` `japanese` `chinese` `any` (또는 `ko` `en` `ja` `zh`)

### 개선 가이드

검사 실패 시 위반 목록 뒤에 카테고리별 수정 가이드가 출력됩니다. 비활성화:

```yaml
guide:
  enabled: false
```

## 명령어

```
git-warden init          기본 설정 파일 생성 (.git-warden.yml)
git-warden diff          staged diff 검사 (주석/인코딩/린트/바이너리/유니코드)
git-warden run           추적 중인 모든 파일 정책 검사
git-warden msg <file>    커밋 메시지 파일 검사
git-warden prepare-msg   prepare-commit-msg 훅: 편집기에 정책 힌트 표시
git-warden fix           git history 자동 수정 (--dry-run 지원)
git-warden migrate       설정 파일을 최신 스키마로 마이그레이션
git-warden analyze       리포지터리 분석 (언어 감지, 린트 설정 확인)
git-warden clean         캐시/빌드 디렉터리의 미추적 파일 제거
git-warden version       버전 정보 출력
```

### diff 명령 (CI 친화적 from..to)

```bash
git-warden diff                    # 기본: staged (pre-commit)
git-warden diff --staged           # 명시적
git-warden diff HEAD               # HEAD ↔ 워킹 트리
git-warden diff origin/main        # origin/main ↔ 워킹 트리
git-warden diff A B                # A ↔ B
git-warden diff A..B               # range
git-warden diff A...B              # merge-base(A,B) ↔ B
```

`--only` 플래그로 특정 검사만 실행:

```bash
git-warden diff --only comment_language
git-warden diff --only lint,encoding
```

CI 사용 예시:

```yaml
# GitHub Actions
- run: git-warden diff ${{ github.event.pull_request.base.sha }}..HEAD

# GitLab CI
- git-warden diff ${CI_MERGE_REQUEST_DIFF_BASE_SHA}..HEAD
```

## 지원 언어

| 언어 | 확장자 |
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

## i18n 지원

CLI 출력 지원 언어: 한국어(ko), English(en), 日本語(ja), 中文(zh).

`GIT_WARDEN_LANG`, `LC_ALL`, `LC_MESSAGES`, `LANG` 환경 변수 또는 설정 파일의 `locale` 필드로 지정합니다.

## 라이선스

MIT
