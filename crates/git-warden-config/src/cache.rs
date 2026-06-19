//! cache.rs: URL response caching and HTTP fetch. Corresponds to Go `internal/config/cache.go` + fetchURL.

use crate::types::AllowedWordsCacheConfig;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

const MAX_FETCH_SIZE: usize = 10 * 1024 * 1024;

/// Fetches raw bytes from an HTTP/HTTPS URL (10 MB limit, 10 s timeout). Go `fetchURL`.
pub fn fetch_url(raw_url: &str) -> Result<Vec<u8>, String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .new_agent();
    let resp = agent.get(raw_url).call().map_err(|e| e.to_string())?;
    if resp.status().as_u16() != 200 {
        return Err(format!("HTTP {}", resp.status().as_u16()));
    }
    let mut buf = Vec::new();
    resp.into_body()
        .into_reader()
        .take((MAX_FETCH_SIZE + 1) as u64)
        .read_to_end(&mut buf)
        .map_err(|e| e.to_string())?;
    if buf.len() > MAX_FETCH_SIZE {
        return Err("url response exceeds 10MB limit".to_string());
    }
    Ok(buf)
}

fn cache_key(raw_url: &str, prefix: &str, ext: &str) -> String {
    let mut h = Sha256::new();
    h.update(raw_url.as_bytes());
    let digest = h.finalize();
    let hex: String = digest[..8].iter().map(|b| format!("{b:02x}")).collect();
    format!("{prefix}_{hex}.{ext}")
}

fn words_key(raw_url: &str) -> String {
    cache_key(raw_url, "words", "txt")
}

fn preset_key(raw_url: &str) -> String {
    cache_key(raw_url, "preset", "yml")
}

fn ttl_duration(cache: &AllowedWordsCacheConfig) -> Duration {
    Duration::from_secs(cache.get_ttl_secs())
}

fn read_if_fresh(path: &PathBuf, ttl: Duration) -> Option<Vec<u8>> {
    let meta = std::fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    let age = SystemTime::now().duration_since(modified).ok()?;
    if age > ttl {
        return None;
    }
    std::fs::read(path).ok()
}

/// Loads cached allowed-words bytes. Go `loadCachedWords` (parsing is done by the caller).
pub fn load_cached_words_bytes(cache: &AllowedWordsCacheConfig, raw_url: &str) -> Option<Vec<u8>> {
    if !cache.is_enabled() {
        return None;
    }
    let path = cache.get_dir().join(words_key(raw_url));
    read_if_fresh(&path, ttl_duration(cache))
}

/// Saves allowed-words bytes to the cache. Go `saveCachedWords`.
pub fn save_cached_words(cache: &AllowedWordsCacheConfig, raw_url: &str, body: &[u8]) {
    if !cache.is_enabled() {
        return;
    }
    let dir = cache.get_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let _ = std::fs::write(dir.join(words_key(raw_url)), body);
}

/// Loads cached preset bytes. Go `loadCachedBytes`.
pub fn load_cached_bytes(cache: &AllowedWordsCacheConfig, raw_url: &str) -> Option<Vec<u8>> {
    if !cache.is_enabled() {
        return None;
    }
    let path = cache.get_dir().join(preset_key(raw_url));
    read_if_fresh(&path, ttl_duration(cache))
}

/// Saves preset bytes to the cache. Go `saveCachedBytes`.
pub fn save_cached_bytes(cache: &AllowedWordsCacheConfig, raw_url: &str, body: &[u8]) {
    if !cache.is_enabled() {
        return;
    }
    let dir = cache.get_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let _ = std::fs::write(dir.join(preset_key(raw_url)), body);
}
