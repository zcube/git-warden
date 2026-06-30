[English](./README.md) | [한국어](./README.ko.md) | [日本語](./README.ja.md) | [中文](./README.zh.md)

# git-warden

Gitコミットメッセージとソースコードのポリシーを自動的に検査するCLIツールです。
[lefthook](https://github.com/evilmartians/lefthook) / husky などのGitフックマネージャーと一緒に使用します。

## 機能

| 検査項目 | 説明 |
|---|---|
| **コメント言語** | 指定された言語（韓国語/英語/日本語/中国語）でコメントが書かれているか検査 |
| **許可単語辞書** | 技術用語・固有名詞を許可単語として登録し、誤検出を防止 |
| **Co-authored-by** | AI共著者トレーラーのブロック（メール許可リスト対応） |
| **Unicode空白** | NBSP、EM SPACE、ZWSP、BiDi制御文字などの非標準空白をブロック |
| **紛らわしい文字** | ASCII文字に似たUnicode文字をブロック（例：キリル文字のA vs ラテン文字のA） |
| **ファイルUnicode検査** | ソース/マークダウンファイル内の不可視・紛らわしいUnicode文字を検出 |
| **不正なUTF-8** | 無効なバイトシーケンスをブロック |
| **絵文字禁止** | コミットメッセージやコメントでの絵文字使用をブロック（オプション） |
| **バイナリファイルポリシー** | 拡張子別 block / allow / lfs ポリシー（画像は既定で許可、git LFS 検証対応） |
| **エンコーディング検査** | UTF-8以外のファイルのコミットをブロック（chardetベース） |
| **データファイルlint** | YAML、JSON（JSON5/JSONC対応）、XML構文検査 |
| **EditorConfig** | .editorconfigルールへの準拠を検査 |
| **Conventional Commits** | コミットメッセージ形式の強制（オプション） |
| **append-onlyパス** | 指定パスでのファイル削除・内容変更・中間挿入を禁止（DBマイグレーション等） |
| **キャッシュ/ビルドディレクトリ** | `node_modules`, `dist`, `build`, `target`, `__pycache__`, `.venv` などへのコミットをブロック（親インジケータ検証） |
| **clean コマンド** | キャッシュ/ビルドディレクトリ内の未追跡ファイルを整理（追跡ファイルは保護） |
| **リポジトリ分析** | 開発言語の検出とlint設定の欠落警告 |
| **自動修正（fix）** | Unicode/エンコーディング違反をgit履歴で一括修正 |
| **設定マイグレーション** | 旧バージョンの設定ファイルを自動検出し、最新スキーマに変換 |
| **進捗表示** | ratatui TUIスピナー（TTY検出、非TTY時テキストフォールバック） |

## インストール

### バイナリダウンロード

[GitHub Releases](https://github.com/zcube/git-warden/releases) からプラットフォームに合ったファイルをダウンロードします。

```bash
# Linux (amd64)
curl -L https://github.com/zcube/git-warden/releases/latest/download/git-warden_<TAG>_x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv git-warden /usr/local/bin/

# Linux (arm64)
curl -L https://github.com/zcube/git-warden/releases/latest/download/git-warden_<TAG>_aarch64-unknown-linux-gnu.tar.gz | tar xz
sudo mv git-warden /usr/local/bin/

# macOS (Apple Silicon)
curl -L https://github.com/zcube/git-warden/releases/latest/download/git-warden_<TAG>_aarch64-apple-darwin.tar.gz | tar xz
sudo mv git-warden /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/zcube/git-warden/releases/latest/download/git-warden_<TAG>_x86_64-apple-darwin.tar.gz | tar xz
sudo mv git-warden /usr/local/bin/
```

`<TAG>` を最新のリリースタグ（例: `v0.1.0`）に置き換えてください。最新バージョンは [Releases](https://github.com/zcube/git-warden/releases) で確認できます。

### ソースからビルド

```bash
# Rust 1.88以上が必要です
cargo install --git https://github.com/zcube/git-warden --bin git-warden
```

`git-warden version` でインストールを確認してください。

## Git フック統合（lefthook）

### 1. lefthook のインストール

```bash
# macOS
brew install lefthook

# npm
npm install --save-dev lefthook

# go install
go install github.com/evilmartians/lefthook@latest
```

### 2. git-warden のインストール

[GitHub Releases](https://github.com/zcube/git-warden/releases) からダウンロードするか、ソースからビルドします。

### 3. lefthook.yml の作成

プロジェクトルートに `lefthook.yml` を作成します：

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

### 4. フックのインストール

```bash
lefthook install
```

以降、`git commit` のたびに自動的に検査が実行されます。

### オプションのフック（必要なものだけ追加）

以下の各ブロックは独立しています。必要なものだけ `lefthook.yml` に追記してください。

#### 検査前の自動修正（fix）

`fix` は修正したファイルを自動的に `git add` で再ステージします。基本設定の `pre-commit` ブロックを以下に置き換えてください（lefthook はコマンドを名前順に実行するため、`auto-fix` が `git-warden` より先に実行されます）：

```yaml
pre-commit:
  commands:
    auto-fix:
      run: git-warden fix
      stage_fixed: true
    git-warden:
      run: git-warden diff
```

#### マージによる回避を防止（pre-merge-commit）

マージコミットは pre-commit フックをトリガーしません。マージ経由での違反混入を防ぐために pre-merge-commit にも登録します：

```yaml
pre-merge-commit:
  commands:
    git-warden:
      run: git-warden diff
```

#### コミットメッセージのポリシーヒント（prepare-commit-msg）

コミットメッセージエディタの下部に `#` コメントとして現在有効なポリシーヒントを表示します（-m/merge/squash/amend 時は何もしません）。必ず `{0}` を使用してください：

```yaml
prepare-commit-msg:
  commands:
    policy-hint:
      run: git-warden prepare-msg {0}
```

#### push 前のコミットメッセージ検査（pre-push）

```yaml
pre-push:
  commands:
    check-commits:
      run: git-warden push
```

### 5. 既存ファイルの全件チェック（初回導入時）

既存リポジトリに git-warden を導入する場合、フックインストール前のファイルは検査されません。導入時に一度全ファイルを検査するには `run` コマンドを使用します：

```bash
git-warden run
```

違反を自動修正するには `fix` コマンドと組み合わせます：

```bash
# 変更内容のプレビュー
git-warden fix --dry-run

# 修正を適用
git-warden fix
```

### husky（Node.js プロジェクト）

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

### Git 2.54+ 設定ベースのフック（フックマネージャー不要）

Git 2.54 以降では、lefthook などのフックマネージャーを使わずに git 設定だけで git-warden を統合できます。

```bash
# 基本：ステージされた変更を検査（pre-commit）
git config set hook.git-warden-diff.command "git-warden diff"
git config set --append hook.git-warden-diff.event pre-commit

# 基本：コミットメッセージを検査（commit-msg）
git config set hook.git-warden-msg.command "git-warden msg"
git config set --append hook.git-warden-msg.event commit-msg

# オプション：push 前のコミットメッセージ検査（pre-push）
git config set hook.git-warden-push.command "git-warden push"
git config set --append hook.git-warden-push.event pre-push

# オプション：マージコミットの検査（pre-merge-commit）
git config set hook.git-warden-merge.command "git-warden diff"
git config set --append hook.git-warden-merge.event pre-merge-commit

# オプション：コミットメッセージエディタにポリシーヒントを表示（prepare-commit-msg）
git config set hook.git-warden-prepare.command "git-warden prepare-msg"
git config set --append hook.git-warden-prepare.event prepare-commit-msg
```

- `--global` を追加するとすべてのリポジトリに適用されます。
- 登録の確認: `git hook list pre-commit`

### その他のフック統合

#### git am ワークフロー

```bash
git config set hook.git-warden-am-msg.command "git-warden msg"
git config set --append hook.git-warden-am-msg.event applypatch-msg

git config set hook.git-warden-am-diff.command "git-warden diff"
git config set --append hook.git-warden-am-diff.event pre-applypatch
```

#### サーバーサイドの強制（update フック）

```bash
#!/bin/sh
# hooks/update — 引数: <refname> <old> <new>
exec git-warden push --range "$2..$3"
```

新しいブランチ（old がすべてゼロの場合）は警告を表示してスキップします。

## グローバルインストール

フックと設定を一度登録するだけで、すべてのリポジトリで git-warden が動作します。

### グローバルフック + グローバル設定

```bash
git config set --global hook.git-warden-diff.command "git-warden diff"
git config set --global --append hook.git-warden-diff.event pre-commit
git config set --global hook.git-warden-msg.command "git-warden msg"
git config set --global --append hook.git-warden-msg.event commit-msg
```

グローバル設定ファイルは以下の順序で検索され、**最初に見つかったファイル**が使用されます：

| 順序 | 場所 |
|---|---|
| 1 | `$GIT_WARDEN_GLOBAL_CONFIG`（明示的指定、ファイルがない場合は警告して無視） |
| 2 | `$XDG_CONFIG_HOME/git-warden/config.yaml`（`config.yml` も対応） |
| 3 | OS標準設定ディレクトリ — Linux `~/.config/git-warden/config.yaml`、macOS `~/Library/Application Support/git-warden/config.yaml`、Windows `%AppData%\git-warden\config.yaml` |
| 4 | `$HOME/.config/git-warden/config.yaml`（`config.yml` も対応） |
| 5 | `~/.git-warden.yml`（レガシー） |

```yaml
# グローバル設定の例
# macOS: ~/Library/Application Support/git-warden/config.yml
# Linux: ~/.config/git-warden/config.yml
commit_message:
  no_ai_coauthor: true
  no_unicode_spaces: true
  no_ambiguous_chars: true
  no_bad_runes: true
  no_emoji: true
  locale: ja

  conventional_commit:
    enabled: true
    locale: en

  language_check:
    enabled: true
    locale: ja
```

### ディレクトリ別ポリシー（gitdir include）

```yaml
# ~/.config/git-warden/config.yaml
include:
  - path: ~/.config/git-warden/base.yml   # 条件なし → 常に読み込み
  - path: ~/.config/git-warden/work.yml
    gitdir: ~/work/                        # ~/work/ 以下のリポジトリのみ
comment_language:
  locale: ja
```

- 優先度：本体 > 後のinclude > 前のinclude。
- `gitdir`：`~` はホームディレクトリに展開され、末尾の `/` でサブツリー全体にマッチします。
- グローバル設定とプロジェクト設定の両方で使用可能。ネストされたincludeは無視されます。

### リポジトリ別の制御（override・opt-out・opt-in）

**override** — リポジトリに `.git-warden.yml` または `.git-warden.yaml` が存在する場合、グローバル設定は完全に無視され、リポジトリ設定のみが適用されます。

**opt-out** — 特定のリポジトリですべての検査を無効にするには：

```yaml
enabled: false
```

**opt-in** — 設定ファイルがあるリポジトリのみ検査する：

```bash
git config set --global hook.git-warden-diff.command "git-warden diff --require-config"
git config set --global hook.git-warden-msg.command "git-warden msg --require-config"
```

## 設定

プロジェクトルートに `.git-warden.yml` を作成します。`git-warden init` でデフォルトの設定ファイルを自動生成できます。VS Code では `.git-warden.schema.json` を使用してオートコンプリートを有効にできます。

```yaml
# yaml-language-server: $schema=./.git-warden.schema.json

comment_language:
  enabled: true
  required_language: japanese  # korean | english | japanese | chinese | any
  min_length: 5
  check_mode: diff             # diff | full
  no_emoji: false              # true でコメント内の絵文字を禁止
  extensions:
    - .go
    - .ts
    - .py
    - .tf

  # 許可単語: 言語検出時に無視する単語リスト
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
  # default_policy: block       # block | allow | lfs (デフォルト: block)
  # rules:
  #   - extensions: [.psd, .ai]
  #     policy: lfs
  # ignore_files:
  #   - "**/*.png"

lint:
  enabled: true
  yaml:
    enabled: true
    # comment_filter: true    # ファイル内 skip-lint コメントを有効化
  json:
    enabled: true
    # allow_json5: true
    # comment_filter: true
  xml:
    enabled: true

encoding:
  enabled: true
  require_utf8: true
  # no_invisible_chars: true
  # no_ambiguous_chars: true

editorconfig:
  enabled: true
  # ignore_files:
  #   - "vendor/**"

commit_message:
  # enabled: true
  no_ai_coauthor: true
  no_unicode_spaces: true
  no_ambiguous_chars: true
  no_bad_runes: true
  no_emoji: false
  locale: ja
  conventional_commit:
    enabled: false
  language_check:
    enabled: false
    required_language: japanese

append_only:
  enabled: false
  # paths:
  #   - "migrations/**"
  #   - "db/migrations/**"

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

設定ファイルがない場合はデフォルト値が適用されます。

### バイナリファイルポリシー

拡張子ごとに3つのポリシーを設定できます：

| ポリシー | 動作 |
|---|---|
| `block` | 拒否（デフォルト） |
| `allow` | 許可 |
| `lfs` | git LFS で追跡されている場合のみ許可（`.gitattributes` の `filter=lfs` を確認） |

組み込み画像拡張子（`.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.bmp`, `.ico`, `.tiff`, `.tif`, `.heic`, `.heif`, `.avif`）はルールがない場合 **`allow`** が適用されます。

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

優先度: `rules` マッチ > 組み込み画像（`allow`）> `default_policy`。

### データファイルlint

`.jsonc` 拡張子のファイルは設定に関わらず常にJSON5モードで検査されます。`# git-warden: skip-lint` コメントでファイルの検査をスキップできます。

```yaml
lint:
  enabled: true
  yaml:
    enabled: true
    comment_filter: true     # ファイル内 skip-lint コメントを有効化
  json:
    enabled: true
    comment_filter: true     # .json ファイルを JSONC モードで検査
  xml:
    enabled: true
```

### append-only パス

```yaml
append_only:
  enabled: true
  paths:
    - "migrations/**"
    - "db/migrations/**"
  # filename_order: none   # デフォルトは numeric; none でソート順チェックを無効化
```

許可される変更：新しいファイルの追加（既存ファイルの後にソートされるもの）、既存ファイルの末尾への追記。
禁止される変更：ファイルの削除、既存行の変更・削除、ファイル中間への挿入。

### protected_paths（保護パス）

append_only より厳格な完全凍結ポリシーです。

```yaml
protected_paths:
  enabled: true
  paths:
    - "legacy/**"
```

| 検査 | 許可される変更 |
|---|---|
| `append_only` | 新しいファイルの追加、既存ファイル末尾への追記 |
| `protected_paths` | なし（完全凍結） |

### ビルド成果物・キャッシュディレクトリ

```yaml
cache_dir:
  enabled: true
  ignore_dirs:
    - vendor
```

対応ディレクトリ: `node_modules`, `dist`, `out`, `build`, `target`, `vendor`, `.gradle`, `.next`, `.nuxt`, `.output`, `.svelte-kit`, `.yarn`, `.bun`, `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, `.turbo`, `.parcel-cache`, `.venv`, `.tox`, `.nox`, `.embuild`, `.dart_tool`。

親インジケータ検証により誤検出を低減します（例: `node_modules` は親に `package.json` がある場合のみブロック）。

#### clean コマンド

```bash
# 発見項目の一覧表示のみ（dry-run）
git-warden clean

# 未追跡ファイルを実際に削除
git-warden clean --yes
```

git 追跡ファイルは絶対に削除されません。

### 許可単語辞書

```yaml
comment_language:
  # インラインリスト
  allowed_words:
    - TypeScript
    - JavaScript
    - API
    - URL

  # ローカルファイル（1行1単語、# コメント対応）
  allowed_words_file: .git-warden-words.txt

  # URL（同じ形式、HTTP/HTTPS）
  allowed_words_url: https://example.com/allowed-words.txt

  # URLキャッシュ（オプション）
  allowed_words_cache:
    enabled: true
    ttl: 24h
```

3つのソース（インライン、ファイル、URL）はマージされます。

### ファイル別言語ルール

```yaml
comment_language:
  required_language: japanese
  file_languages:
    - pattern: "locales/**"
      language: any
    - pattern: "i18n/**"
      language: english
    - pattern: "locale/ko/**"
      language: ko
```

### ソース内ディレクティブ

ファイルまたはリージョン単位で言語ルールをオーバーライドできます：

```go
// git-warden:ignore
// This English comment is intentional (next comment only)

// git-warden:file-lang=english  <- ファイル全体に適用

// git-warden:disable:lang=english
// This block is intentionally in English
// git-warden:enable
```

対応ディレクティブ：

| ディレクティブ | 説明 |
|---|---|
| `git-warden:ignore` | 直後のコメントのみスキップ |
| `git-warden:disable` | この行から検査を無効化 |
| `git-warden:disable:lang=<L>` | 無効化し、このリージョンで言語Lを使用 |
| `git-warden:enable` | 検査を再有効化 |
| `git-warden:lang=<L>` | この行から必須言語をLに切り替え |
| `git-warden:file-lang=<L>` | ファイル全体の必須言語をLに設定 |

`<L>` の値: `korean` `english` `japanese` `chinese` `any`（または `ko` `en` `ja` `zh`）

### 修正ガイド

検査失敗時、違反リストとサマリーの後にカテゴリ別の修正ガイドが表示されます。設定で無効化できます：

```yaml
guide:
  enabled: false
```

グローバルフラグ `--no-guide` でも無効化できます。

## コマンド

```
git-warden init          デフォルト設定ファイルを生成（.git-warden.yml）
git-warden diff          ステージされた差分を検査（コメント/エンコーディング/lint/バイナリ/Unicode）
git-warden run           追跡中のすべてのファイルを検査
git-warden msg <file>    コミットメッセージファイルを検査
git-warden prepare-msg   prepare-commit-msg フック用：エディタにポリシーヒントを表示
git-warden fix           git履歴を自動修正（--dry-run 対応）
git-warden migrate       設定ファイルを最新スキーマに移行
git-warden analyze       リポジトリ分析（言語検出、lint設定確認）
git-warden clean         キャッシュ/ビルドディレクトリの未追跡ファイルを削除
git-warden version       バージョン情報を表示
```

### diff コマンド（CI対応 `from..to`）

```bash
git-warden diff                      # デフォルト: ステージ済み（pre-commit）
git-warden diff --staged             # 明示的（--cached も可）
git-warden diff HEAD                 # HEAD ↔ ワーキングツリー
git-warden diff origin/main          # origin/main ↔ ワーキングツリー
git-warden diff A B                  # A ↔ B
git-warden diff A..B                 # A ↔ B（range表記）
git-warden diff A...B                # merge-base(A,B) ↔ B
```

`--only` フラグで特定の検査のみ実行できます（設定で `enabled: false` のものも強制実行）：

```bash
git-warden diff --only comment_language
git-warden diff --only lint,encoding
```

CI での使用例：

```yaml
# GitHub Actions: PRの差分を検査
- run: git-warden diff ${{ github.event.pull_request.base.sha }}..HEAD

# GitLab CI: MRの差分を検査
- git-warden diff ${CI_MERGE_REQUEST_DIFF_BASE_SHA}..HEAD
```

### init コマンド

```bash
git-warden init             # システムロケールを自動検出
git-warden init --lang ja   # ロケールを指定
git-warden init --force     # 既存ファイルを上書き
```

### run コマンド

```bash
git-warden run              # すべての追跡ファイルを検査
git-warden run --only lint  # 特定の検査のみ実行
```

`diff` と異なり、ステージ状態に関わらず `git ls-files` で追跡されているすべてのファイルを検査します。

### fix コマンド

```bash
git-warden fix --dry-run
git-warden fix --range HEAD~5..HEAD
git-warden fix --mine --dry-run
```

### migrate コマンド

```bash
git-warden migrate
git-warden migrate --dry-run
```

旧バージョンの設定ファイルを最新スキーマに自動変換します。コメントと書式は保持されます。

### analyze コマンド

```bash
git-warden analyze
```

開発言語を検出し、lint設定ファイル（`.golangci.yml`, `.eslintrc.*`, `pyproject.toml` など）が不足している場合に警告します。`.editorconfig`, `.gitattributes`, `.gitignore` の存在も確認します。

## 対応言語

| 言語 | 拡張子 |
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

## i18n サポート

CLIの出力は以下の言語に対応しています：

- 韓国語 (ko) — デフォルト
- English (en)
- 日本語 (ja)
- 中文 (zh)

環境変数 `GIT_WARDEN_LANG`, `LC_ALL`, `LC_MESSAGES`, `LANG` または設定ファイルの `locale` フィールドで設定します。

## ライセンス

MIT
