//! allowed_words.rs: reads allowed words from allowed_words_file/url and merges them. Corresponds to Go `internal/config/allowed_words.go`.

use super::cache;
use super::types::Config;

/// Merges words from allowed_words_file and allowed_words_url into allowed_words. Go `resolveAllowedWords`.
pub fn resolve_allowed_words(cfg: &mut Config) -> Result<(), String> {
    if !cfg.comment_language.allowed_words_file.is_empty() {
        let mut file_path = cfg.comment_language.allowed_words_file.clone();
        if let Some(rest) = file_path.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                file_path = home.join(rest).to_string_lossy().to_string();
                cfg.comment_language.allowed_words_file = file_path.clone();
            }
        }
        let data = std::fs::read_to_string(&file_path)
            .map_err(|e| format!("failed to read allowed_words_file: {e}"))?;
        let words = parse_word_lines(&data);
        cfg.comment_language.allowed_words.extend(words);
    }
    if !cfg.comment_language.allowed_words_url.is_empty() {
        let url = cfg.comment_language.allowed_words_url.clone();
        let cache_cfg = cfg.comment_language.allowed_words_cache.clone();
        if let Some(bytes) = cache::load_cached_words_bytes(&cache_cfg, &url) {
            let words = parse_word_lines(&String::from_utf8_lossy(&bytes));
            cfg.comment_language.allowed_words.extend(words);
        } else {
            let body = cache::fetch_url(&url)
                .map_err(|e| format!("failed to fetch allowed_words_url: {e}"))?;
            let words = parse_word_lines(&String::from_utf8_lossy(&body));
            cfg.comment_language.allowed_words.extend(words);
            cache::save_cached_words(&cache_cfg, &url, &body);
        }
    }
    Ok(())
}

/// Splits text into lines and returns non-empty, non-comment entries. Go `parseWordLines`.
pub fn parse_word_lines(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    for line in text.split('\n') {
        let word = line.trim();
        if word.is_empty() || word.starts_with('#') {
            continue;
        }
        words.push(word.to_string());
    }
    words
}
