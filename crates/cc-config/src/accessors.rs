//! accessors.rs: getter and query methods for config structs. Corresponds to Go `internal/config/accessors.go`.
//! Reproduces `*bool` default semantics (e.g., enabled defaults to true).

use crate::types::*;

fn ext_lower(path: &str) -> String {
    // filepath.Ext: from the last '.' to the end (empty string if none).
    let base = path.rsplit('/').next().unwrap_or(path);
    match base.rfind('.') {
        Some(i) => base[i..].to_lowercase(),
        None => String::new(),
    }
}

fn normalize_policy(p: &str) -> String {
    match p.trim().to_lowercase().as_str() {
        "allow" => "allow",
        "lfs" => "lfs",
        "block" | "" => "block",
        _ => "block",
    }
    .to_string()
}

impl Config {
    /// Whether git-warden is globally enabled (default true). Corresponds to Go `Config.IsEnabled`.
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }
}

impl FileLanguageRule {
    /// Normalized language identifier (Locale > Language). Corresponds to Go `FileLanguageRule.GetLocale`.
    pub fn get_locale(&self) -> String {
        let v = cc_langdetect::normalize_locale(&self.locale);
        if !v.is_empty() {
            return v;
        }
        cc_langdetect::normalize_locale(&self.language)
    }
}

impl CommentLanguageConfig {
    /// Default true.
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }
    /// Locale > RequiredLanguage > "korean". Go `CommentLanguageConfig.GetLocale`.
    pub fn get_locale(&self) -> String {
        let v = cc_langdetect::normalize_locale(&self.locale);
        if !v.is_empty() {
            return v;
        }
        let v = cc_langdetect::normalize_locale(&self.required_language);
        if !v.is_empty() {
            return v;
        }
        cc_langdetect::KOREAN.to_string()
    }
    /// Default false.
    pub fn is_no_emoji(&self) -> bool {
        self.no_emoji.unwrap_or(false)
    }
    /// check_mode == "full".
    pub fn is_full_mode(&self) -> bool {
        self.check_mode == "full"
    }
    /// Default false.
    pub fn is_check_strings(&self) -> bool {
        self.check_strings.unwrap_or(false)
    }
    /// Default true.
    pub fn is_skip_technical_strings(&self) -> bool {
        self.skip_technical_strings.unwrap_or(true)
    }
}

impl CommitMessageLanguageConfig {
    /// Default false.
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }
    /// Locale > RequiredLanguage > "korean".
    pub fn get_locale(&self) -> String {
        let v = cc_langdetect::normalize_locale(&self.locale);
        if !v.is_empty() {
            return v;
        }
        let v = cc_langdetect::normalize_locale(&self.required_language);
        if !v.is_empty() {
            return v;
        }
        cc_langdetect::KOREAN.to_string()
    }
}

impl ConventionalCommitConfig {
    /// Default false.
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }
    /// Default false.
    pub fn is_require_scope(&self) -> bool {
        self.require_scope.unwrap_or(false)
    }
    /// Default true.
    pub fn is_allow_merge_commits(&self) -> bool {
        self.allow_merge_commits.unwrap_or(true)
    }
    /// Default true.
    pub fn is_allow_revert_commits(&self) -> bool {
        self.allow_revert_commits.unwrap_or(true)
    }
    /// Configured type list (falls back to built-in defaults). Go `GetTypes`.
    pub fn get_types(&self) -> Vec<String> {
        if !self.types.is_empty() {
            return self.types.clone();
        }
        DEFAULT_CONVENTIONAL_TYPES
            .iter()
            .map(|s| s.to_string())
            .collect()
    }
    /// Type aliases (user-defined > locale built-in > empty). Go `GetTypeAliases`.
    pub fn get_type_aliases(&self) -> std::collections::BTreeMap<String, String> {
        if !self.type_aliases.is_empty() {
            return self.type_aliases.clone();
        }
        if !self.locale.is_empty() {
            if let Some(m) = localized_conventional_types(&self.locale) {
                return m
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
            }
        }
        std::collections::BTreeMap::new()
    }
    /// All allowed types (standard + aliases). Go `GetAllAllowedTypes`.
    pub fn get_all_allowed_types(&self) -> Vec<String> {
        let types = self.get_types();
        let aliases = self.get_type_aliases();
        if aliases.is_empty() {
            return types;
        }
        let mut all = types;
        for alias in aliases.keys() {
            all.push(alias.clone());
        }
        all
    }
    /// Resolves an alias to its canonical type. Go `ResolveType`.
    pub fn resolve_type(&self, commit_type: &str) -> String {
        let aliases = self.get_type_aliases();
        if let Some(standard) = aliases.get(commit_type) {
            return standard.clone();
        }
        commit_type.to_string()
    }
}

impl SubjectLimitConfig {
    /// Default false.
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }
    /// Default 72.
    pub fn get_max_length(&self) -> i64 {
        if self.max_length <= 0 {
            72
        } else {
            self.max_length
        }
    }
}

impl BodyLineLimitConfig {
    /// Default false.
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }
    /// Default 100.
    pub fn get_max_length(&self) -> i64 {
        if self.max_length <= 0 {
            100
        } else {
            self.max_length
        }
    }
}

impl CommitMessageConfig {
    /// Default true.
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }
    /// Default true.
    pub fn is_no_ai_coauthor(&self) -> bool {
        self.no_ai_coauthor.unwrap_or(true)
    }
    /// Default true.
    pub fn is_no_unicode_spaces(&self) -> bool {
        self.no_unicode_spaces.unwrap_or(true)
    }
    /// Default true.
    pub fn is_no_ambiguous_chars(&self) -> bool {
        self.no_ambiguous_chars.unwrap_or(true)
    }
    /// Default true.
    pub fn is_no_bad_runes(&self) -> bool {
        self.no_bad_runes.unwrap_or(true)
    }
    /// Default false.
    pub fn is_no_emoji(&self) -> bool {
        self.no_emoji.unwrap_or(false)
    }
    /// Returns true when the email matches a built-in AI pattern or a user-defined removal pattern. Go `CoauthorShouldRemove`.
    pub fn coauthor_should_remove(&self, email: &str) -> bool {
        let email_lower = email.trim().to_lowercase();
        for pattern in BUILTIN_AI_COAUTHOR_PATTERNS {
            if cc_pathutil::fnmatch(pattern, &email_lower) {
                return true;
            }
        }
        for pattern in &self.coauthor_remove_emails {
            let low = pattern.trim().to_lowercase();
            if cc_pathutil::fnmatch(&low, &email_lower) {
                return true;
            }
        }
        false
    }
}

/// Extracts the email address from a Co-authored-by trailer (strips angle brackets). Go `ExtractCoauthorEmail`.
pub fn extract_coauthor_email(line: &str) -> String {
    let lt = line.rfind('<');
    let gt = line.rfind('>');
    if let (Some(lt), Some(gt)) = (lt, gt) {
        if gt > lt {
            return line[lt + 1..gt].to_string();
        }
    }
    String::new()
}

impl BinaryFileConfig {
    /// Default true.
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }
    /// Returns the policy for the given path (rules > built-in image allow > default). Go `PolicyFor`.
    pub fn policy_for(&self, path: &str) -> String {
        let ext = ext_lower(path);
        for r in &self.rules {
            for e in &r.extensions {
                if ext.eq_ignore_ascii_case(e) {
                    return normalize_policy(&r.policy);
                }
            }
        }
        for e in BUILTIN_IMAGE_EXTENSIONS {
            if ext == *e {
                return "allow".to_string();
            }
        }
        normalize_policy(&self.default_policy)
    }
}

impl LintConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }
}
impl LintRuleConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }
}
impl YamlLintConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }
    pub fn is_comment_filter(&self) -> bool {
        self.comment_filter.unwrap_or(false)
    }
}
impl JsonLintConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }
    pub fn is_allow_json5(&self) -> bool {
        self.allow_json5.unwrap_or(false)
    }
    pub fn is_comment_filter(&self) -> bool {
        self.comment_filter.unwrap_or(false)
    }
}

impl EncodingConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }
    pub fn is_require_utf8(&self) -> bool {
        self.require_utf8.unwrap_or(true)
    }
    pub fn is_no_invisible_chars(&self) -> bool {
        self.no_invisible_chars.unwrap_or(false)
    }
    pub fn is_no_ambiguous_chars(&self) -> bool {
        self.no_ambiguous_chars.unwrap_or(false)
    }
}

impl EditorConfigConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }
}

impl ProtectedPathsConfig {
    /// enabled && !paths.is_empty(). Go `ProtectedPathsConfig.IsEnabled`.
    pub fn is_enabled(&self) -> bool {
        self.enabled && !self.paths.is_empty()
    }
}

impl AppendOnlyConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled && !self.paths.is_empty()
    }
    /// Numeric ordering check is active unless filename_order == "none" (default true). Go `IsFilenameOrderNumeric`.
    pub fn is_filename_order_numeric(&self) -> bool {
        self.filename_order != "none"
    }
}

impl CacheDirConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }
}

impl GuideConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }
}

impl AllowedWordsCacheConfig {
    /// Default false.
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }
    /// Cache TTL in seconds (default 24h). Go `GetTTL`.
    pub fn get_ttl_secs(&self) -> u64 {
        if self.ttl.is_empty() {
            return 24 * 3600;
        }
        parse_duration_secs(&self.ttl).unwrap_or(24 * 3600)
    }
    /// Cache directory path. Go `GetDir`.
    pub fn get_dir(&self) -> std::path::PathBuf {
        if !self.dir.is_empty() {
            return std::path::PathBuf::from(&self.dir);
        }
        match dirs::home_dir() {
            Some(home) => home.join(".cache").join("git-warden"),
            None => std::env::temp_dir().join("git-warden-cache"),
        }
    }
}

/// Parses a subset of Go's time.ParseDuration format (s/m/h/d units) and returns seconds.
fn parse_duration_secs(s: &str) -> Option<u64> {
    // Simple format: number + unit repeated (e.g. "24h", "1h30m").
    let mut total: f64 = 0.0;
    let mut num = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_digit() || c == '.' {
            num.push(c);
            i += 1;
            continue;
        }
        // Parse unit.
        let unit_start = i;
        while i < bytes.len() && !(bytes[i] as char).is_ascii_digit() && bytes[i] as char != '.' {
            i += 1;
        }
        let unit = &s[unit_start..i];
        let value: f64 = num.parse().ok()?;
        num.clear();
        let mult = match unit {
            "ns" => 1e-9,
            "us" | "µs" => 1e-6,
            "ms" => 1e-3,
            "s" => 1.0,
            "m" => 60.0,
            "h" => 3600.0,
            _ => return None,
        };
        total += value * mult;
    }
    if !num.is_empty() {
        return None; // trailing number with no unit
    }
    Some(total as u64)
}
