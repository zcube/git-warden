use std::collections::HashMap;
use std::fs;
use serde::{Deserialize, Serialize};

// 설정 구조체 — 애플리케이션 전반의 설정을 담습니다
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    /// 서버 바인딩 주소
    pub host: String,
    /// 리슨 포트 번호
    pub port: u16,
}

impl Config {
    // 파일에서 설정을 불러옵니다
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        /* 파일 읽기 및 JSON 파싱:
           오류 발생 시 상위로 전파합니다.
        */
        let content = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// 기본 설정 값을 반환합니다.
    pub fn default_config() -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("host".to_string(), "로컬호스트".to_string()); // 기본 호스트
        map
    }
}
