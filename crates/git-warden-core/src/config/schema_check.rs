//! schema_check.rs: validates config files against the JSON Schema (`.git-warden.schema.json`).
//!
//! The original Go codebase ships the schema only for editor autocompletion and does not validate
//! at runtime, but this port enhances `validate` to also check against the schema — catching
//! unknown (typo) fields and enum violations that serde silently ignores. Legacy configs are
//! migrated to the current schema before validation.

use once_cell::sync::Lazy;
use std::path::Path;

use super::schema;

/// Embeds the distributed schema file (repo root) at compile time.
const SCHEMA_JSON: &str = include_str!("../../.git-warden.schema.json");

static VALIDATOR: Lazy<Option<jsonschema::Validator>> = Lazy::new(|| {
    let value: serde_json::Value = serde_json::from_str(SCHEMA_JSON).ok()?;
    jsonschema::validator_for(&value).ok()
});

/// Validates YAML text against the schema and returns a list of violation warnings.
/// Returns an empty list if the text fails to parse as YAML (syntax errors are handled at the load stage).
pub fn validate_against_schema(cfg_path: &str, yaml_text: &str) -> Vec<String> {
    let Some(validator) = VALIDATOR.as_ref() else {
        return Vec::new();
    };
    let Ok(value) = serde_yaml::from_str::<serde_json::Value>(yaml_text) else {
        return Vec::new();
    };
    let mut warns = Vec::new();
    for err in validator.iter_errors(&value) {
        let loc = err.instance_path().to_string();
        let location = if loc.is_empty() {
            String::new()
        } else {
            format!(" ({loc})")
        };
        warns.push(crate::t!(
            "validate.schema_violation",
            Path = cfg_path,
            Message = err.to_string(),
            Location = location
        ));
    }
    warns
}

/// Reads a config file (migrating to the current schema if necessary) and validates it against the JSON Schema.
/// Returns an empty list if the file does not exist (e.g., global config). Not present in the original Go implementation.
pub fn validate_schema_file(cfg_path: &str) -> Vec<String> {
    let Ok(data) = std::fs::read(Path::new(cfg_path)) else {
        return Vec::new();
    };
    // Migrate legacy configs before validating against the current schema.
    let text = match schema::detect_version(&data) {
        schema::Version::Current | schema::Version::Unknown => {
            String::from_utf8_lossy(&data).into_owned()
        }
        _ => match schema::migrate(&data) {
            Ok(r) => String::from_utf8_lossy(&r.data).into_owned(),
            Err(_) => String::from_utf8_lossy(&data).into_owned(),
        },
    };
    validate_against_schema(cfg_path, &text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_config_no_schema_warnings() {
        crate::i18n::set_locale("en");
        let yaml = "comment_language:\n  enabled: true\n  check_mode: diff\n";
        assert!(validate_against_schema("c.yml", yaml).is_empty());
    }

    #[test]
    fn unknown_field_flagged() {
        crate::i18n::set_locale("en");
        let yaml = "bogus_top_field: 1\n";
        let w = validate_against_schema("c.yml", yaml);
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(w[0].contains("bogus_top_field"));
    }

    #[test]
    fn bad_enum_flagged_with_location() {
        crate::i18n::set_locale("en");
        let yaml = "comment_language:\n  check_mode: nonsense\n";
        let w = validate_against_schema("c.yml", yaml);
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(w[0].contains("check_mode"));
    }
}
