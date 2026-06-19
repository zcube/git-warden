//! schema.rs: config file schema version detection and migration. Corresponds to Go `internal/config/schema`.
//!
//! The base version is determined by strict parsing per version (serde `deny_unknown_fields`),
//! and promotion to a newer version is decided via signature path markers. Migration transforms
//! YAML text line-by-line. Structs mirror the Go schema package (separate from the main Config,
//! with a limited field set).

use serde::Deserialize;

/// Detected schema version. Corresponds to Go `Version`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    Current, // v1.2.0
    V110,
    V102,
    V101,
    V100,
    Unknown,
}

impl Version {
    pub fn as_str(&self) -> &'static str {
        match self {
            Version::Current => "v1.2.0",
            Version::V110 => "v1.1.0",
            Version::V102 => "v1.0.2",
            Version::V101 => "v1.0.1",
            Version::V100 => "v1.0.0",
            Version::Unknown => "unknown",
        }
    }
}

// ── shared structs for strict parsing (deny_unknown_fields) ──

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct AllowedWordsCacheConfig {
    enabled: Option<bool>,
    ttl: String,
    dir: String,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct PresetConfig {
    url: String,
    cache: AllowedWordsCacheConfig,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct FileLanguageRule {
    pattern: String,
    language: String,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct FileLanguageRuleV120 {
    pattern: String,
    locale: String,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct ExceptionsConfig {
    global_ignore: Vec<String>,
    comment_language_ignore: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct CommitMessageLanguageConfig {
    enabled: Option<bool>,
    required_language: String,
    min_length: i64,
    skip_prefixes: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct CommitMessageLanguageConfigV120 {
    enabled: Option<bool>,
    locale: String,
    min_length: i64,
    skip_prefixes: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct ConventionalCommitConfig {
    enabled: Option<bool>,
    types: Vec<String>,
    type_aliases: std::collections::BTreeMap<String, String>,
    locale: String,
    require_scope: Option<bool>,
    allow_merge_commits: Option<bool>,
    allow_revert_commits: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct ConventionalCommitConfigV100 {
    enabled: Option<bool>,
    types: Vec<String>,
    require_scope: Option<bool>,
    allow_merge_commits: Option<bool>,
    allow_revert_commits: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct BinaryFileConfig {
    enabled: Option<bool>,
    ignore_files: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct LintRuleConfig {
    enabled: Option<bool>,
    ignore_files: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct YamlLintConfig {
    enabled: Option<bool>,
    comment_filter: Option<bool>,
    ignore_files: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct JsonLintConfig {
    enabled: Option<bool>,
    allow_json5: Option<bool>,
    comment_filter: Option<bool>,
    ignore_files: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct LintConfig {
    enabled: Option<bool>,
    yaml: YamlLintConfig,
    json: JsonLintConfig,
    xml: LintRuleConfig,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct EditorConfigConfig {
    enabled: Option<bool>,
    ignore_files: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct EncodingConfigCurrent {
    enabled: Option<bool>,
    require_utf8: Option<bool>,
    no_invisible_chars: Option<bool>,
    no_ambiguous_chars: Option<bool>,
    locale: String,
    ignore_files: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct EncodingConfigV101 {
    enabled: Option<bool>,
    require_utf8: Option<bool>,
    ignore_files: Vec<String>,
}

// ── v1.0.0 ──
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct CommentLanguageConfigV100 {
    enabled: Option<bool>,
    required_language: String,
    languages: Vec<String>,
    extensions: Vec<String>,
    min_length: i64,
    skip_directives: Vec<String>,
    check_mode: String,
    ignore_files: Vec<String>,
    locale: String,
    file_languages: Vec<FileLanguageRule>,
    check_strings: Option<bool>,
    skip_technical_strings: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct CommitMessageConfigV100 {
    no_coauthor: Option<bool>,
    coauthor_remove_emails: Vec<String>,
    no_unicode_spaces: Option<bool>,
    no_ambiguous_chars: Option<bool>,
    no_bad_runes: Option<bool>,
    locale: String,
    language_check: CommitMessageLanguageConfig,
    conventional_commit: ConventionalCommitConfigV100,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct ConfigV100 {
    preset: PresetConfig,
    comment_language: CommentLanguageConfigV100,
    commit_message: CommitMessageConfigV100,
    exceptions: ExceptionsConfig,
}

// ── v1.0.1 ──
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct CommentLanguageConfigV101 {
    enabled: Option<bool>,
    required_language: String,
    languages: Vec<String>,
    extensions: Vec<String>,
    min_length: i64,
    skip_directives: Vec<String>,
    check_mode: String,
    ignore_files: Vec<String>,
    locale: String,
    file_languages: Vec<FileLanguageRule>,
    no_emoji: Option<bool>,
    check_strings: Option<bool>,
    skip_technical_strings: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct CommitMessageConfigV101 {
    enabled: Option<bool>,
    no_coauthor: Option<bool>,
    coauthor_remove_emails: Vec<String>,
    no_unicode_spaces: Option<bool>,
    no_ambiguous_chars: Option<bool>,
    no_bad_runes: Option<bool>,
    no_emoji: Option<bool>,
    locale: String,
    language_check: CommitMessageLanguageConfig,
    conventional_commit: ConventionalCommitConfig,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct ConfigV101 {
    preset: PresetConfig,
    comment_language: CommentLanguageConfigV101,
    commit_message: CommitMessageConfigV101,
    binary_file: BinaryFileConfig,
    lint: LintConfig,
    encoding: EncodingConfigV101,
    editorconfig: EditorConfigConfig,
    exceptions: ExceptionsConfig,
}

// ── v1.0.2 ──
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct CommentLanguageConfigV102 {
    enabled: Option<bool>,
    required_language: String,
    languages: Vec<String>,
    extensions: Vec<String>,
    min_length: i64,
    skip_directives: Vec<String>,
    check_mode: String,
    ignore_files: Vec<String>,
    locale: String,
    file_languages: Vec<FileLanguageRule>,
    no_emoji: Option<bool>,
    check_strings: Option<bool>,
    skip_technical_strings: Option<bool>,
}

// v1.0.2 commit_message uses the v1.1.0 shape (no_ai_coauthor).
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct CommitMessageConfigV110 {
    enabled: Option<bool>,
    no_ai_coauthor: Option<bool>,
    coauthor_remove_emails: Vec<String>,
    no_unicode_spaces: Option<bool>,
    no_ambiguous_chars: Option<bool>,
    no_bad_runes: Option<bool>,
    no_emoji: Option<bool>,
    locale: String,
    language_check: CommitMessageLanguageConfig,
    conventional_commit: ConventionalCommitConfig,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct ConfigV102 {
    preset: PresetConfig,
    comment_language: CommentLanguageConfigV102,
    commit_message: CommitMessageConfigV110,
    binary_file: BinaryFileConfig,
    lint: LintConfig,
    encoding: EncodingConfigV101,
    editorconfig: EditorConfigConfig,
    exceptions: ExceptionsConfig,
}

// ── v1.1.0 ──
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct CommentLanguageConfigV110 {
    enabled: Option<bool>,
    required_language: String,
    languages: Vec<String>,
    extensions: Vec<String>,
    min_length: i64,
    skip_directives: Vec<String>,
    check_mode: String,
    ignore_files: Vec<String>,
    locale: String,
    file_languages: Vec<FileLanguageRule>,
    no_emoji: Option<bool>,
    check_strings: Option<bool>,
    skip_technical_strings: Option<bool>,
    allowed_words: Vec<String>,
    allowed_words_file: String,
    allowed_words_url: String,
    allowed_words_cache: AllowedWordsCacheConfig,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct ConfigV110 {
    preset: PresetConfig,
    comment_language: CommentLanguageConfigV110,
    commit_message: CommitMessageConfigV110,
    binary_file: BinaryFileConfig,
    lint: LintConfig,
    encoding: EncodingConfigCurrent,
    editorconfig: EditorConfigConfig,
    exceptions: ExceptionsConfig,
}

// ── v1.2.0 (current) ──
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct CommentLanguageConfigCurrent {
    enabled: Option<bool>,
    languages: Vec<String>,
    extensions: Vec<String>,
    min_length: i64,
    skip_directives: Vec<String>,
    check_mode: String,
    ignore_files: Vec<String>,
    locale: String,
    file_languages: Vec<FileLanguageRuleV120>,
    no_emoji: Option<bool>,
    check_strings: Option<bool>,
    skip_technical_strings: Option<bool>,
    allowed_words: Vec<String>,
    allowed_words_file: String,
    allowed_words_url: String,
    allowed_words_cache: AllowedWordsCacheConfig,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct CommitMessageConfigCurrent {
    enabled: Option<bool>,
    no_ai_coauthor: Option<bool>,
    coauthor_remove_emails: Vec<String>,
    no_unicode_spaces: Option<bool>,
    no_ambiguous_chars: Option<bool>,
    no_bad_runes: Option<bool>,
    no_emoji: Option<bool>,
    locale: String,
    language_check: CommitMessageLanguageConfigV120,
    conventional_commit: ConventionalCommitConfig,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct ConfigCurrent {
    preset: PresetConfig,
    comment_language: CommentLanguageConfigCurrent,
    commit_message: CommitMessageConfigCurrent,
    binary_file: BinaryFileConfig,
    lint: LintConfig,
    encoding: EncodingConfigCurrent,
    editorconfig: EditorConfigConfig,
    exceptions: ExceptionsConfig,
}

fn parse_strict<T: for<'de> Deserialize<'de>>(data: &[u8]) -> bool {
    let s = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return false,
    };
    serde_yaml::from_str::<T>(s).is_ok()
}

struct VersionSpec {
    version: Version,
    parse: fn(&[u8]) -> bool,
    signature: &'static [&'static str],
}

const V110_MARKERS: &[&str] = &[
    "comment_language.allowed_words",
    "comment_language.allowed_words_file",
    "comment_language.allowed_words_url",
    "comment_language.allowed_words_cache",
    "encoding.no_invisible_chars",
    "encoding.no_ambiguous_chars",
    "encoding.locale",
];

fn version_chain() -> Vec<VersionSpec> {
    vec![
        VersionSpec {
            version: Version::V100,
            parse: parse_strict::<ConfigV100>,
            signature: &[],
        },
        VersionSpec {
            version: Version::V101,
            parse: parse_strict::<ConfigV101>,
            signature: &[
                "binary_file",
                "lint",
                "encoding",
                "editorconfig",
                "commit_message.enabled",
                "commit_message.no_emoji",
                "comment_language.no_emoji",
            ],
        },
        VersionSpec {
            version: Version::V102,
            parse: parse_strict::<ConfigV102>,
            signature: &["commit_message.no_ai_coauthor"],
        },
        VersionSpec {
            version: Version::V110,
            parse: parse_strict::<ConfigV110>,
            signature: V110_MARKERS,
        },
        VersionSpec {
            version: Version::Current,
            parse: parse_strict::<ConfigCurrent>,
            signature: V110_MARKERS,
        },
    ]
}

/// Detects the schema version of YAML data. Corresponds to Go `DetectVersion`.
pub fn detect_version(data: &[u8]) -> Version {
    let chain = version_chain();
    // Step 1: strict-parse old→new in order; first success is the base.
    let mut base: i64 = -1;
    for (i, spec) in chain.iter().enumerate() {
        if (spec.parse)(data) {
            base = i as i64;
            break;
        }
    }
    if base < 0 {
        return Version::Unknown;
    }

    // Step 2: starting from the newest, promote if signature markers are present and strict-parse passes.
    let raw: serde_yaml::Value = match std::str::from_utf8(data)
        .ok()
        .and_then(|s| serde_yaml::from_str(s).ok())
    {
        Some(v) => v,
        None => return chain[base as usize].version,
    };
    let mut i = chain.len() as i64 - 1;
    while i > base {
        let spec = &chain[i as usize];
        if has_any_yaml_path(&raw, spec.signature) && (spec.parse)(data) {
            return spec.version;
        }
        i -= 1;
    }
    chain[base as usize].version
}

fn has_any_yaml_path(raw: &serde_yaml::Value, paths: &[&str]) -> bool {
    paths.iter().any(|p| {
        let segs: Vec<&str> = p.split('.').collect();
        yaml_path_exists(raw, &segs)
    })
}

fn yaml_path_exists(node: &serde_yaml::Value, segs: &[&str]) -> bool {
    if segs.is_empty() {
        return true;
    }
    let map = match node {
        serde_yaml::Value::Mapping(m) => m,
        _ => return false,
    };
    let (seg0, is_seq) = match segs[0].strip_suffix("[]") {
        Some(k) => (k, true),
        None => (segs[0], false),
    };
    let value = match map.get(serde_yaml::Value::String(seg0.to_string())) {
        Some(v) => v,
        None => return false,
    };
    if is_seq {
        if let serde_yaml::Value::Sequence(items) = value {
            return items.iter().any(|item| yaml_path_exists(item, &segs[1..]));
        }
        return false;
    }
    yaml_path_exists(value, &segs[1..])
}

/// Migration result. Corresponds to Go `MigrateResult`.
pub struct MigrateResult {
    pub detected_version: Version,
    pub applied: Vec<String>,
    pub data: Vec<u8>,
}

const RENAME_NO_COAUTHOR: &str = "commit_message.no_coauthor → commit_message.no_ai_coauthor";
const RENAME_REQUIRED_LANGUAGE: &str = "required_language / language → locale (merged in v1.2.0)";

/// Returns the migrateUp rule descriptions for the given version (data transformation is done in apply_migration).
fn migrate_up_descriptions(version: Version) -> Vec<&'static str> {
    // Rules in chain order from `version` to the end.
    let mut out = Vec::new();
    let order = [
        Version::V100,
        Version::V101,
        Version::V102,
        Version::V110,
        Version::Current,
    ];
    let start = order
        .iter()
        .position(|v| *v == version)
        .unwrap_or(order.len());
    for v in &order[start..] {
        match v {
            Version::V101 => out.push(RENAME_NO_COAUTHOR), // v1.0.1 → v1.0.2
            Version::V110 => out.push(RENAME_REQUIRED_LANGUAGE), // v1.1.0 → v1.2.0
            _ => {}
        }
    }
    out
}

/// Migrates YAML data to the current schema. Corresponds to Go `Migrate`.
pub fn migrate(data: &[u8]) -> Result<MigrateResult, String> {
    let version = detect_version(data);
    if version == Version::Unknown {
        return Err("unrecognized config file format".to_string());
    }
    let mut result = MigrateResult {
        detected_version: version,
        applied: Vec::new(),
        data: data.to_vec(),
    };
    if version == Version::Current {
        return Ok(result);
    }

    let mut migrated = data.to_vec();
    for desc in migrate_up_descriptions(version) {
        migrated = apply_rule(desc, &migrated);
        result.applied.push(desc.to_string());
    }

    if !parse_strict::<ConfigCurrent>(&migrated) {
        return Err("validation failed after migration".to_string());
    }
    result.data = migrated;
    Ok(result)
}

fn apply_rule(desc: &str, data: &[u8]) -> Vec<u8> {
    match desc {
        RENAME_NO_COAUTHOR => rename_yaml_key(data, "no_coauthor", "no_ai_coauthor"),
        RENAME_REQUIRED_LANGUAGE => {
            let mut d = remove_key_if_peer_exists(data, "required_language", "locale");
            d = remove_key_if_peer_exists(&d, "language", "locale");
            d = rename_yaml_key(&d, "required_language", "locale");
            d = rename_yaml_key(&d, "language", "locale");
            d
        }
        _ => data.to_vec(),
    }
}

/// Renames a key in YAML text (preserving comments). Corresponds to Go `renameYAMLKey`.
fn rename_yaml_key(data: &[u8], old_key: &str, new_key: &str) -> Vec<u8> {
    let text = String::from_utf8_lossy(data);
    let old_prefix = format!("{old_key}:");
    let new_prefix = format!("{new_key}:");
    let lines: Vec<String> = text
        .split('\n')
        .map(|line| {
            let trimmed = line.trim_start_matches([' ', '\t']);
            if trimmed.starts_with(&old_prefix) {
                line.replacen(&old_prefix, &new_prefix, 1)
            } else {
                line.to_string()
            }
        })
        .collect();
    lines.join("\n").into_bytes()
}

/// Removes the stale_key line when peer_key is present in the same block. Corresponds to Go `removeKeyIfPeerExists`.
fn remove_key_if_peer_exists(data: &[u8], stale_key: &str, peer_key: &str) -> Vec<u8> {
    let text = String::from_utf8_lossy(data);
    let lines: Vec<&str> = text.split('\n').collect();
    let stale_prefix = format!("{stale_key}:");
    let peer_prefix = format!("{peer_key}:");

    let indent_of = |s: &str| -> usize { s.chars().take_while(|&c| c == ' ' || c == '\t').count() };
    let key_at = |line: &str| -> String {
        let trimmed = line.trim_start_matches([' ', '\t']);
        match trimmed.find(':') {
            Some(idx) => trimmed[..idx + 1].to_string(),
            None => String::new(),
        }
    };

    let mut to_remove = std::collections::HashSet::new();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start_matches([' ', '\t']);
        if !trimmed.starts_with(&stale_prefix) {
            continue;
        }
        let indent = indent_of(line);
        let has_peer = {
            let scan = |start: i64, step: i64| -> bool {
                let mut j = start;
                while j >= 0 && (j as usize) < lines.len() {
                    if j as usize != i {
                        let ln = lines[j as usize];
                        let lt = ln.trim_start_matches([' ', '\t']);
                        if ln.trim().is_empty() || lt.starts_with('#') {
                            j += step;
                            continue;
                        }
                        let ind = indent_of(ln);
                        if ind < indent {
                            return false; // block boundary
                        }
                        if ind > indent {
                            j += step;
                            continue;
                        }
                        if key_at(ln) == peer_prefix {
                            return true;
                        }
                    }
                    j += step;
                }
                false
            };
            scan(i as i64 - 1, -1) || scan(i as i64 + 1, 1)
        };
        if has_peer {
            to_remove.insert(i);
        }
    }

    if to_remove.is_empty() {
        return data.to_vec();
    }
    let out: Vec<&str> = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| !to_remove.contains(i))
        .map(|(_, l)| *l)
        .collect();
    out.join("\n").into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn td(name: &str) -> Vec<u8> {
        fs::read(format!("testdata/schema/{name}")).unwrap()
    }

    #[test]
    fn detect_version_cases() {
        assert_eq!(detect_version(&td("current.yml")), Version::Current);
        assert_eq!(detect_version(&td("v1_1_0.yml")), Version::V110);
        assert_eq!(detect_version(&td("v1_0_2.yml")), Version::V102);
        assert_eq!(detect_version(&td("v1_0_1.yml")), Version::V101);
        assert_eq!(detect_version(&td("v1_0_0.yml")), Version::V100);
    }

    #[test]
    fn detect_version_invalid() {
        assert_eq!(detect_version(b":::invalid yaml:::"), Version::Unknown);
        assert_eq!(
            detect_version(b"completely_unknown_field: true\n"),
            Version::Unknown
        );
    }

    #[test]
    fn migrate_current_noop() {
        let r = migrate(&td("current.yml")).unwrap();
        assert_eq!(r.detected_version, Version::Current);
        assert!(r.applied.is_empty());
    }

    #[test]
    fn migrate_v100() {
        let r = migrate(&td("v1_0_0.yml")).unwrap();
        assert_eq!(r.detected_version, Version::V100);
        assert!(!r.applied.is_empty());
        assert_eq!(r.data, td("v1_0_0_migrated.yml"));
    }

    #[test]
    fn migrate_v101() {
        let r = migrate(&td("v1_0_1.yml")).unwrap();
        assert_eq!(r.detected_version, Version::V101);
        assert_eq!(r.data, td("v1_0_1_migrated.yml"));
    }

    #[test]
    fn migrate_v102() {
        let r = migrate(&td("v1_0_2.yml")).unwrap();
        assert_eq!(r.detected_version, Version::V102);
        assert_eq!(r.data, td("v1_0_2_migrated.yml"));
    }

    #[test]
    fn migrate_v110() {
        let r = migrate(&td("v1_1_0.yml")).unwrap();
        assert_eq!(r.detected_version, Version::V110);
        assert_eq!(r.data, td("v1_1_0_migrated.yml"));
    }
}
