use serde::Deserialize;
use std::{fs, path::PathBuf};

#[derive(Deserialize)]
pub struct Config {
    pub base_url: String,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::config_path();
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
        Ok(serde_json::from_str(&content)?)
    }

    fn config_path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("config.json")))
            .unwrap_or_else(|| PathBuf::from("config.json"))
    }
}
