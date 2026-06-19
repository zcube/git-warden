//! types.rs: Config and sub-config struct definitions with built-in default data. Corresponds to Go `internal/config/types.go`.
//!
//! Go's `*bool` tri-state is mapped to `Option<bool>` (unset = None). All YAML keys
//! match Rust field names (snake_case), so serde maps them without explicit renaming.
//! All structs use `#[serde(default)]` to fill missing fields with defaults; the top-level
//! Config allows unknown fields (same as Go's non-strict yaml.Unmarshal).

use serde::Deserialize;

/// Conditional config include rule. Corresponds to Go `IncludeRule`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct IncludeRule {
    pub path: String,
    pub gitdir: String,
}

/// Remote URL preset. Corresponds to Go `PresetConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PresetConfig {
    pub url: String,
    pub cache: AllowedWordsCacheConfig,
}

/// Caching config for allowed_words_url / preset. Corresponds to Go `AllowedWordsCacheConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AllowedWordsCacheConfig {
    pub enabled: Option<bool>,
    pub ttl: String,
    pub dir: String,
}

/// Comment language check config. Corresponds to Go `CommentLanguageConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CommentLanguageConfig {
    pub enabled: Option<bool>,
    pub required_language: String,
    pub languages: Vec<String>,
    pub extensions: Vec<String>,
    pub min_length: i64,
    pub skip_directives: Vec<String>,
    pub check_mode: String,
    pub ignore_files: Vec<String>,
    pub locale: String,
    pub file_languages: Vec<FileLanguageRule>,
    pub no_emoji: Option<bool>,
    pub check_strings: Option<bool>,
    pub skip_technical_strings: Option<bool>,
    pub allowed_words: Vec<String>,
    pub allowed_words_file: String,
    pub allowed_words_url: String,
    pub allowed_words_cache: AllowedWordsCacheConfig,
}

/// Per-file language rule. Corresponds to Go `FileLanguageRule`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FileLanguageRule {
    pub pattern: String,
    pub locale: String,
    pub language: String,
}

/// Commit message check config. Corresponds to Go `CommitMessageConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CommitMessageConfig {
    pub enabled: Option<bool>,
    pub no_ai_coauthor: Option<bool>,
    pub coauthor_remove_emails: Vec<String>,
    pub no_unicode_spaces: Option<bool>,
    pub no_ambiguous_chars: Option<bool>,
    pub no_bad_runes: Option<bool>,
    pub no_emoji: Option<bool>,
    pub locale: String,
    pub language_check: CommitMessageLanguageConfig,
    pub conventional_commit: ConventionalCommitConfig,
    pub subject_limit: SubjectLimitConfig,
    pub body_line_limit: BodyLineLimitConfig,
}

/// Commit message body language check config. Corresponds to Go `CommitMessageLanguageConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CommitMessageLanguageConfig {
    pub enabled: Option<bool>,
    pub locale: String,
    pub required_language: String,
    pub min_length: i64,
    pub skip_prefixes: Vec<String>,
}

/// Conventional Commits format config. Corresponds to Go `ConventionalCommitConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ConventionalCommitConfig {
    pub enabled: Option<bool>,
    pub types: Vec<String>,
    pub type_aliases: std::collections::BTreeMap<String, String>,
    pub locale: String,
    pub require_scope: Option<bool>,
    pub allow_merge_commits: Option<bool>,
    pub allow_revert_commits: Option<bool>,
}

/// Subject length limit config. Corresponds to Go `SubjectLimitConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct SubjectLimitConfig {
    pub enabled: Option<bool>,
    pub max_length: i64,
}

/// Body line length limit config. Corresponds to Go `BodyLineLimitConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct BodyLineLimitConfig {
    pub enabled: Option<bool>,
    pub max_length: i64,
}

/// Binary file detection config. Corresponds to Go `BinaryFileConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct BinaryFileConfig {
    pub enabled: Option<bool>,
    pub default_policy: String,
    pub rules: Vec<BinaryFilePolicyRule>,
    pub ignore_files: Vec<String>,
}

/// Per-extension binary policy rule. Corresponds to Go `BinaryFilePolicyRule`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct BinaryFilePolicyRule {
    pub extensions: Vec<String>,
    pub policy: String,
}

/// Data file lint config. Corresponds to Go `LintConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct LintConfig {
    pub enabled: Option<bool>,
    pub yaml: YamlLintConfig,
    pub json: JsonLintConfig,
    pub xml: LintRuleConfig,
    pub toml: LintRuleConfig,
}

/// Single lint rule config. Corresponds to Go `LintRuleConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct LintRuleConfig {
    pub enabled: Option<bool>,
    pub ignore_files: Vec<String>,
}

/// YAML lint config. Corresponds to Go `YAMLLintConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct YamlLintConfig {
    pub enabled: Option<bool>,
    pub comment_filter: Option<bool>,
    pub ignore_files: Vec<String>,
}

/// JSON lint config. Corresponds to Go `JSONLintConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct JsonLintConfig {
    pub enabled: Option<bool>,
    pub allow_json5: Option<bool>,
    pub comment_filter: Option<bool>,
    pub ignore_files: Vec<String>,
}

/// File encoding check config. Corresponds to Go `EncodingConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct EncodingConfig {
    pub enabled: Option<bool>,
    pub require_utf8: Option<bool>,
    pub no_invisible_chars: Option<bool>,
    pub no_ambiguous_chars: Option<bool>,
    pub locale: String,
    pub ignore_files: Vec<String>,
}

/// .editorconfig check config. Corresponds to Go `EditorConfigConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct EditorConfigConfig {
    pub enabled: Option<bool>,
    pub ignore_files: Vec<String>,
}

/// Global and feature-specific exclusion patterns. Corresponds to Go `ExceptionsConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ExceptionsConfig {
    pub global_ignore: Vec<String>,
    pub comment_language_ignore: Vec<String>,
}

/// Append-only path config. Corresponds to Go `AppendOnlyConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AppendOnlyConfig {
    pub enabled: bool,
    pub paths: Vec<String>,
    pub filename_order: String,
}

/// Protected paths (fully frozen) config. Corresponds to Go `ProtectedPathsConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ProtectedPathsConfig {
    pub enabled: bool,
    pub paths: Vec<String>,
}

/// Cache/build directory check config. Corresponds to Go `CacheDirConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CacheDirConfig {
    pub enabled: Option<bool>,
    pub ignore_dirs: Vec<String>,
}

/// Improvement guide output config. Corresponds to Go `GuideConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct GuideConfig {
    pub enabled: Option<bool>,
}

/// Regex-based custom rules bundle. Corresponds to Go `CustomRulesConfig`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CustomRulesConfig {
    pub commit_message: Vec<CustomRule>,
    pub diff: Vec<CustomRule>,
}

/// Single custom rule. Corresponds to Go `CustomRule`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CustomRule {
    pub name: String,
    pub pattern: String,
    pub message: String,
    pub required: bool,
}

/// Top-level config. Corresponds to Go `Config`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub enabled: Option<bool>,
    pub include: Vec<IncludeRule>,
    pub preset: PresetConfig,
    pub comment_language: CommentLanguageConfig,
    pub commit_message: CommitMessageConfig,
    pub binary_file: BinaryFileConfig,
    pub lint: LintConfig,
    pub encoding: EncodingConfig,
    pub editorconfig: EditorConfigConfig,
    pub exceptions: ExceptionsConfig,
    pub custom_rules: CustomRulesConfig,
    pub protected_paths: ProtectedPathsConfig,
    pub append_only: AppendOnlyConfig,
    pub cache_dir: CacheDirConfig,
    pub guide: GuideConfig,
}

/// Default allowed conventional commit types. Corresponds to Go `DefaultConventionalTypes`.
pub const DEFAULT_CONVENTIONAL_TYPES: &[&str] = &[
    "feat", "fix", "docs", "style", "refactor", "perf", "test", "build", "ci", "chore", "revert",
];

/// Built-in AI tool email glob patterns. Corresponds to Go `BuiltinAICoauthorPatterns`.
pub const BUILTIN_AI_COAUTHOR_PATTERNS: &[&str] = &[
    "*copilot*@*",
    "noreply@anthropic.com",
    "*@cursor.sh",
    "*@codeium.com",
    "*@tabnine.com",
    "*amazon-q*@*",
    "*@sourcegraph.com",
    "*gemini*@*",
];

/// Built-in image extensions (allowed by default). Corresponds to Go `BuiltinImageExtensions`.
pub const BUILTIN_IMAGE_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp", ".ico", ".tiff", ".tif", ".heic", ".heif",
    ".avif",
];

/// Localized-to-standard conventional commit type mapping by language. Corresponds to Go `LocalizedConventionalTypes`.
pub fn localized_conventional_types(
    locale: &str,
) -> Option<&'static [(&'static str, &'static str)]> {
    const KO: &[(&str, &str)] = &[
        ("기능", "feat"),
        ("수정", "fix"),
        ("문서", "docs"),
        ("스타일", "style"),
        ("리팩터", "refactor"),
        ("리팩토링", "refactor"),
        ("성능", "perf"),
        ("테스트", "test"),
        ("빌드", "build"),
        ("배포", "ci"),
        ("잡일", "chore"),
        ("되돌리기", "revert"),
    ];
    const JA: &[(&str, &str)] = &[
        ("機能", "feat"),
        ("修正", "fix"),
        ("ドキュメント", "docs"),
        ("スタイル", "style"),
        ("リファクタリング", "refactor"),
        ("パフォーマンス", "perf"),
        ("テスト", "test"),
        ("ビルド", "build"),
        ("デプロイ", "ci"),
        ("雑務", "chore"),
        ("リバート", "revert"),
    ];
    const ZH: &[(&str, &str)] = &[
        ("功能", "feat"),
        ("修复", "fix"),
        ("文档", "docs"),
        ("样式", "style"),
        ("重构", "refactor"),
        ("性能", "perf"),
        ("测试", "test"),
        ("构建", "build"),
        ("部署", "ci"),
        ("杂务", "chore"),
        ("回退", "revert"),
    ];
    match locale {
        "ko" => Some(KO),
        "ja" => Some(JA),
        "zh" => Some(ZH),
        _ => None,
    }
}
