//! loader.rs: core config loading, global config, and preset handling. Corresponds to Go `internal/config/config.go`.

use crate::allowed_words::resolve_allowed_words;
use crate::cache;
use crate::defaults::apply_defaults;
use crate::error::format_config_error;
use crate::include::resolve_includes;
use crate::merge::merge_configs;
use crate::schema;
use crate::types::Config;
use crate::validate::validate;
use std::path::{Path, PathBuf};

/// Parses YAML bytes into Config (with auto-migration applied). Internal shared helper.
pub(crate) fn parse_yaml_config(data: &[u8]) -> Result<Config, String> {
    let mut data = data.to_vec();
    let ver = schema::detect_version(&data);
    if ver != schema::Version::Current && ver != schema::Version::Unknown {
        if let Ok(result) = schema::migrate(&data) {
            data = result.data;
        }
    }
    let s = String::from_utf8_lossy(&data);
    serde_yaml::from_str::<Config>(&s).map_err(|e| e.to_string())
}

/// Loads config from the given YAML file. Corresponds to Go `Load`.
/// Project config takes priority; falls back to global config (or defaults if absent).
pub fn load(cfg_path: &str) -> Result<Config, String> {
    let data = match std::fs::read(cfg_path) {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // No project config → use global config (or defaults).
            match load_global_config() {
                Some((global_cfg, global_path)) => {
                    return finalize_config(global_cfg, &global_path)
                }
                None => {
                    let mut cfg = Config::default();
                    apply_defaults(&mut cfg);
                    return Ok(cfg);
                }
            }
        }
        Err(e) => return Err(e.to_string()),
    };

    // Auto-migrate old schema versions.
    let mut data = data;
    let ver = schema::detect_version(&data);
    if ver != schema::Version::Current && ver != schema::Version::Unknown {
        match schema::migrate(&data) {
            Ok(result) => data = result.data,
            Err(e) => git_warden_logger::warn(
                "config auto-migration failed, proceeding with original",
                &[("path", cfg_path.to_string()), ("error", e)],
            ),
        }
    }

    let s = String::from_utf8_lossy(&data);
    let cfg: Config =
        serde_yaml::from_str(&s).map_err(|e| format_config_error(cfg_path, &e.to_string()))?;

    let resolved = resolve_includes(&cfg, cfg_path);
    finalize_config(resolved, cfg_path)
}

/// Merges preset → applies defaults → resolves allowed_words → emits validation warnings. Corresponds to Go `finalizeConfig`.
fn finalize_config(cfg: Config, cfg_path: &str) -> Result<Config, String> {
    let mut cfg = cfg;
    if !cfg.preset.url.is_empty() {
        let preset_cfg = load_preset_config(&cfg.preset.url, &cfg.preset.cache)
            .map_err(|e| format!("failed to load preset url: {e}"))?;
        cfg = merge_configs(&preset_cfg, &cfg);
    }
    apply_defaults(&mut cfg);
    resolve_allowed_words(&mut cfg)?;
    for w in validate(&cfg, cfg_path) {
        git_warden_logger::warn(&w, &[]);
    }
    Ok(cfg)
}

/// Fetches and parses config from preset.url. Corresponds to Go `loadPresetConfig`.
fn load_preset_config(
    url: &str,
    cache_cfg: &crate::types::AllowedWordsCacheConfig,
) -> Result<Config, String> {
    let body = match cache::load_cached_bytes(cache_cfg, url) {
        Some(b) => b,
        None => {
            let b = cache::fetch_url(url)?;
            cache::save_cached_bytes(cache_cfg, url, &b);
            b
        }
    };

    let mut body = body;
    let ver = schema::detect_version(&body);
    if ver != schema::Version::Current && ver != schema::Version::Unknown {
        if let Ok(result) = schema::migrate(&body) {
            body = result.data;
        }
    }

    let s = String::from_utf8_lossy(&body);
    let mut cfg: Config =
        serde_yaml::from_str(&s).map_err(|e| format!("failed to parse preset YAML: {e}"))?;
    if !cfg.preset.url.is_empty() {
        return Err(format!(
            "presets cannot be nested (preset.url inside a preset is not allowed): {}",
            cfg.preset.url
        ));
    }
    if !cfg.include.is_empty() {
        git_warden_logger::warn(
            "include in preset config is ignored (local file inclusion from remote config is disallowed)",
            &[("url", url.to_string())],
        );
        cfg.include.clear();
    }
    Ok(cfg)
}

const GLOBAL_CONFIG_ENV: &str = "GIT_WARDEN_GLOBAL_CONFIG";

/// Determines the global config file path. Corresponds to Go `GlobalConfigPath`.
pub fn global_config_path() -> Option<PathBuf> {
    if let Ok(env_path) = std::env::var(GLOBAL_CONFIG_ENV) {
        if !env_path.is_empty() {
            if global_file_exists(Path::new(&env_path)) {
                return Some(PathBuf::from(env_path));
            }
            git_warden_logger::warn(
                "GIT_WARDEN_GLOBAL_CONFIG points to a missing file, ignoring global config",
                &[("path", env_path)],
            );
            return None;
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            candidates.push(Path::new(&xdg).join("git-warden").join("config.yaml"));
            candidates.push(Path::new(&xdg).join("git-warden").join("config.yml"));
        }
    }
    if let Some(dir) = dirs::config_dir() {
        candidates.push(dir.join("git-warden").join("config.yaml"));
        candidates.push(dir.join("git-warden").join("config.yml"));
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".config").join("git-warden").join("config.yaml"));
        candidates.push(home.join(".config").join("git-warden").join("config.yml"));
        candidates.push(home.join(".git-warden.yml"));
    }
    candidates.into_iter().find(|p| global_file_exists(p))
}

fn global_file_exists(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| !m.is_dir())
        .unwrap_or(false)
}

/// Loads the global config. Corresponds to Go `loadGlobalConfig`.
fn load_global_config() -> Option<(Config, String)> {
    let global_path = global_config_path()?;
    let path_str = global_path.to_string_lossy().to_string();
    let data = match std::fs::read(&global_path) {
        Ok(d) => d,
        Err(e) => {
            git_warden_logger::warn(
                "global config read error, ignoring",
                &[("path", path_str.clone()), ("error", e.to_string())],
            );
            return None;
        }
    };

    let mut data = data;
    let ver = schema::detect_version(&data);
    if ver != schema::Version::Current && ver != schema::Version::Unknown {
        match schema::migrate(&data) {
            Ok(result) => data = result.data,
            Err(e) => git_warden_logger::warn(
                "global config auto-migration failed, proceeding with original",
                &[("path", path_str.clone()), ("error", e)],
            ),
        }
    }

    let s = String::from_utf8_lossy(&data);
    let cfg: Config = match serde_yaml::from_str(&s) {
        Ok(c) => c,
        Err(e) => {
            git_warden_logger::warn(
                "global config parse error, ignoring",
                &[("path", path_str), ("error", e.to_string())],
            );
            return None;
        }
    };
    let resolved = resolve_includes(&cfg, &path_str);
    Some((resolved, path_str))
}
