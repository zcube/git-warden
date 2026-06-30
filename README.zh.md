[English](./README.md) | [한국어](./README.ko.md) | [日本語](./README.ja.md) | [中文](./README.zh.md)

# git-warden

自动检查Git提交消息和源代码策略的CLI工具。
与 [lefthook](https://github.com/evilmartians/lefthook) / husky 等Git钩子管理器配合使用。

## 功能

| 检查项 | 说明 |
|---|---|
| **注释语言** | 检查注释是否使用指定语言（韩语/英语/日语/中文）编写 |
| **允许词典** | 注册技术术语和专有名词以防止误报 |
| **Co-authored-by** | 阻止AI共同作者尾部标记（支持邮箱白名单） |
| **Unicode空格** | 阻止不可见/非标准Unicode空白字符（NBSP、ZWSP、BiDi等） |
| **易混淆字符** | 阻止与ASCII字符相似的Unicode字符（如西里尔字母A vs 拉丁字母A） |
| **文件Unicode检查** | 检测源代码/Markdown文件中的不可见和易混淆Unicode字符 |
| **无效UTF-8** | 阻止无效的字节序列 |
| **表情符号禁止** | 阻止在提交消息和注释中使用表情符号（可选） |
| **二进制文件策略** | 按扩展名 block / allow / lfs 策略（图片默认允许，支持git LFS验证） |
| **编码检查** | 阻止提交非UTF-8编码的文件（基于chardet） |
| **数据文件lint** | YAML、JSON（支持JSON5/JSONC）、XML语法验证 |
| **EditorConfig** | 验证文件是否符合.editorconfig规则 |
| **约定式提交** | 强制执行提交消息格式（可选） |
| **append-only路径** | 禁止在指定路径中删除文件、修改内容或中间插入（如DB迁移文件） |
| **缓存/构建目录** | 阻止node_modules、dist、build、target、__pycache__、.venv等的提交 |
| **clean命令** | 清理缓存/构建目录中的未追踪文件（追踪文件保留） |
| **仓库分析** | 检测开发语言并警告缺失的lint配置 |
| **自动修复（fix）** | 在git历史中批量修复unicode/编码违规 |
| **配置迁移** | 自动检测旧版配置文件并迁移到最新架构 |
| **进度指示器** | ratatui TUI旋转器（TTY感知，非TTY时纯文本回退） |

## 安装

### 二进制下载

从 [GitHub Releases](https://github.com/zcube/git-warden/releases) 下载适合您平台的文件：

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

将 `<TAG>` 替换为最新发布标签（如 `v0.1.0`）。

### 从源码构建（Rust 1.88+）

```bash
cargo install --git https://github.com/zcube/git-warden --bin git-warden
```

使用 `git-warden version` 验证安装。

## Git钩子集成（lefthook）

### 1. 安装lefthook

```bash
# macOS
brew install lefthook

# npm
npm install --save-dev lefthook

# go install
go install github.com/evilmartians/lefthook@latest
```

### 2. 安装git-warden

从 [GitHub Releases](https://github.com/zcube/git-warden/releases) 下载或从源码构建。

### 3. 创建lefthook.yml

在项目根目录创建 `lefthook.yml`：

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

### 4. 安装钩子

```bash
lefthook install
```

之后每次 `git commit` 时会自动执行检查。

### 可选钩子（按需添加）

#### 自动修复（fix）

`fix` 会自行对修改过的文件执行 `git add` 重新暂存。请将 `pre-commit` 块替换为以下内容：

```yaml
pre-commit:
  commands:
    auto-fix:
      run: git-warden fix
      stage_fixed: true
    git-warden:
      run: git-warden diff
```

#### 防止merge绕过（pre-merge-commit）

merge提交不会触发pre-commit钩子，请同时注册：

```yaml
pre-merge-commit:
  commands:
    git-warden:
      run: git-warden diff
```

#### 提交信息策略提示（prepare-commit-msg）

在编辑器底部以 `#` 注释显示当前策略提示。请务必使用 `{0}`：

```yaml
prepare-commit-msg:
  commands:
    policy-hint:
      run: git-warden prepare-msg {0}
```

#### push前检查提交信息（pre-push）

```yaml
pre-push:
  commands:
    check-commits:
      run: git-warden push
```

### 5. 检查现有全部文件（初次引入时）

```bash
git-warden run
```

如需自动修复违规项：

```bash
git-warden fix --dry-run   # 预览
git-warden fix             # 应用
```

### husky（Node.js项目）

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

### Git 2.54+ 基于配置的钩子（无需钩子管理器）

```bash
# 基础：检查暂存的变更（pre-commit）
git config set hook.git-warden-diff.command "git-warden diff"
git config set --append hook.git-warden-diff.event pre-commit

# 基础：检查提交信息（commit-msg）
git config set hook.git-warden-msg.command "git-warden msg"
git config set --append hook.git-warden-msg.event commit-msg

# 可选：push前检查（pre-push）
git config set hook.git-warden-push.command "git-warden push"
git config set --append hook.git-warden-push.event pre-push

# 可选：检查merge提交（pre-merge-commit）
git config set hook.git-warden-merge.command "git-warden diff"
git config set --append hook.git-warden-merge.event pre-merge-commit

# 可选：提交信息编辑器中显示策略提示（prepare-commit-msg）
git config set hook.git-warden-prepare.command "git-warden prepare-msg"
git config set --append hook.git-warden-prepare.event prepare-commit-msg
```

加上 `--global` 可一次性应用到所有仓库。使用 `git hook list pre-commit` 确认注册。

### 其他钩子集成

#### git am工作流

```bash
git config set hook.git-warden-am-msg.command "git-warden msg"
git config set --append hook.git-warden-am-msg.event applypatch-msg

git config set hook.git-warden-am-diff.command "git-warden diff"
git config set --append hook.git-warden-am-diff.event pre-applypatch
```

#### 服务器端强制（update钩子）

```bash
#!/bin/sh
# hooks/update — 参数: <refname> <old> <new>
exec git-warden push --range "$2..$3"
```

## 全局安装

### 全局钩子 + 全局配置

```bash
git config set --global hook.git-warden-diff.command "git-warden diff"
git config set --global --append hook.git-warden-diff.event pre-commit
git config set --global hook.git-warden-msg.command "git-warden msg"
git config set --global --append hook.git-warden-msg.event commit-msg
```

全局配置文件查找顺序（使用第一个存在的文件）：

| 顺序 | 位置 |
|---|---|
| 1 | `$GIT_WARDEN_GLOBAL_CONFIG`（显式指定，文件不存在时警告并忽略） |
| 2 | `$XDG_CONFIG_HOME/git-warden/config.yaml`（也支持`config.yml`） |
| 3 | OS标准配置目录 — Linux `~/.config/git-warden/config.yaml`、macOS `~/Library/Application Support/git-warden/config.yaml`、Windows `%AppData%\git-warden\config.yaml` |
| 4 | `$HOME/.config/git-warden/config.yaml`（也支持`config.yml`） |
| 5 | `~/.git-warden.yml`（legacy） |

```yaml
# 全局配置示例
# macOS: ~/Library/Application Support/git-warden/config.yml
# Linux: ~/.config/git-warden/config.yml
commit_message:
  no_ai_coauthor: true
  no_unicode_spaces: true
  no_ambiguous_chars: true
  no_bad_runes: true
  no_emoji: true
  locale: zh

  conventional_commit:
    enabled: true
    locale: en

  language_check:
    enabled: true
    locale: zh
```

### 按目录的策略（gitdir include）

```yaml
# ~/.config/git-warden/config.yaml
include:
  - path: ~/.config/git-warden/base.yml
  - path: ~/.config/git-warden/work.yml
    gitdir: ~/work/
comment_language:
  locale: zh
```

- 优先级：正文 > 后面的include > 前面的include
- `gitdir`：`~`展开为主目录，以`/`结尾时匹配整个子目录树

### 按仓库的控制（override · opt-out · opt-in）

**override** — 仓库中存在 `.git-warden.yml` 或 `.git-warden.yaml` 时，全局配置被完全忽略。

**opt-out** — 在特定仓库中禁用所有检查：

```yaml
enabled: false
```

**opt-in** — 仅检查存在项目配置文件的仓库：

```bash
git config set --global hook.git-warden-diff.command "git-warden diff --require-config"
git config set --global hook.git-warden-msg.command "git-warden msg --require-config"
```

## 配置

在项目根目录创建 `.git-warden.yml`。运行 `git-warden init` 可自动生成默认配置。
VS Code中可通过 `.git-warden.schema.json` 获得自动补全。

```yaml
# yaml-language-server: $schema=./.git-warden.schema.json

comment_language:
  enabled: true
  required_language: chinese  # korean | english | japanese | chinese | any
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
  locale: zh
  conventional_commit:
    enabled: false
  language_check:
    enabled: false
    required_language: chinese

append_only:
  enabled: false

cache_dir:
  enabled: true
```

### 源码内指令

```go
// git-warden:ignore
// 此注释为有意为之（仅跳过下一条）

// git-warden:file-lang=chinese

// git-warden:disable:lang=english
// Intentional English block
// git-warden:enable
```

| 指令 | 说明 |
|---|---|
| `git-warden:ignore` | 仅跳过下一个注释的检查 |
| `git-warden:disable` | 从此行开始禁用检查 |
| `git-warden:disable:lang=<L>` | 禁用并在此区间使用语言L |
| `git-warden:enable` | 重新启用检查 |
| `git-warden:lang=<L>` | 从此行开始切换所需语言 |
| `git-warden:file-lang=<L>` | 设置整个文件的所需语言 |

`<L>` 的取值: `korean` `english` `japanese` `chinese` `any`（或`ko` `en` `ja` `zh`）

## 命令

```
git-warden init          生成默认配置文件（.git-warden.yml）
git-warden diff          检查staged diff（注释/编码/lint/二进制/Unicode）
git-warden run           检查所有已跟踪文件的策略合规性
git-warden msg <file>    检查提交消息文件
git-warden prepare-msg   用于prepare-commit-msg钩子：在编辑器中显示策略提示
git-warden fix           自动修复git历史（支持--dry-run）
git-warden migrate       将配置文件迁移到最新架构
git-warden analyze       仓库分析（语言检测、lint配置确认）
git-warden clean         清理缓存/构建目录的未追踪文件
git-warden version       输出版本信息
```

### diff命令（CI友好的from..to）

```bash
git-warden diff                    # 默认：staged（pre-commit）
git-warden diff --staged           # 显式
git-warden diff HEAD               # HEAD ↔ working tree
git-warden diff origin/main        # origin/main ↔ working tree
git-warden diff A B                # A ↔ B
git-warden diff A..B               # range
git-warden diff A...B              # merge-base(A,B) ↔ B
```

`--only` 标志仅运行指定检查：

```bash
git-warden diff --only comment_language
git-warden diff --only lint,encoding
```

CI示例：

```yaml
# GitHub Actions
- run: git-warden diff ${{ github.event.pull_request.base.sha }}..HEAD

# GitLab CI
- git-warden diff ${CI_MERGE_REQUEST_DIFF_BASE_SHA}..HEAD
```

## 支持的语言

| 语言 | 扩展名 |
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

## i18n支持

CLI输出支持：韩语(ko)、English(en)、日本語(ja)、中文(zh)。

通过环境变量 `GIT_WARDEN_LANG`、`LC_ALL`、`LC_MESSAGES`、`LANG` 或配置文件的 `locale` 字段设置。

## 许可证

MIT
