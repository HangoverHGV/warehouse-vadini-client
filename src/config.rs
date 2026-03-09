use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub base_url: String,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

impl Config {
    pub fn load() -> Self {
        let path = Self::config_path();
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| Config {
                base_url: "https://warehouse.sudurasimontaj.com".to_string(),
                token: None,
                is_admin: false,
                username: None,
                password: None,
            })
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::save_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn data_dir() -> PathBuf {
        #[cfg(target_os = "android")]
        {
            crate::ANDROID_DATA_DIR
                .get()
                .map(|p| p.join("data"))
                .unwrap_or_else(|| PathBuf::from("/data/data/ro.vadini.warehouse/files/data"))
        }
        #[cfg(not(target_os = "android"))]
        {
            // Windows: use %APPDATA%\warehouse-vadini
            #[cfg(target_os = "windows")]
            if let Ok(appdata) = std::env::var("APPDATA") {
                let p = PathBuf::from(appdata).join("warehouse-vadini");
                let _ = fs::create_dir_all(&p);
                return p;
            }

            if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
                let p = PathBuf::from(data_home).join("warehouse-vadini");
                let _ = fs::create_dir_all(&p);
                return p;
            }
            if let Ok(home) = std::env::var("HOME") {
                let p = PathBuf::from(home).join(".local/share/warehouse-vadini");
                let _ = fs::create_dir_all(&p);
                return p;
            }
            PathBuf::from("data")
        }
    }

    /// Path used for loading — checks user config first, then system fallbacks.
    fn config_path() -> PathBuf {
        #[cfg(target_os = "android")]
        {
            crate::ANDROID_DATA_DIR
                .get()
                .map(|p| p.join("config.json"))
                .unwrap_or_else(|| PathBuf::from("/data/data/ro.vadini.warehouse/files/config.json"))
        }
        #[cfg(not(target_os = "android"))]
        {
            // Windows: use %APPDATA%\warehouse-vadini\config.json
            #[cfg(target_os = "windows")]
            if let Ok(appdata) = std::env::var("APPDATA") {
                let p = PathBuf::from(appdata).join("warehouse-vadini/config.json");
                if p.exists() { return p; }
            }

            // 1. Try XDG_CONFIG_HOME
            if let Ok(cfg_home) = std::env::var("XDG_CONFIG_HOME") {
                let p = PathBuf::from(cfg_home).join("warehouse-vadini/config.json");
                if p.exists() { return p; }
            }
            // 2. Try ~/.config
            if let Ok(home) = std::env::var("HOME") {
                let p = PathBuf::from(home).join(".config/warehouse-vadini/config.json");
                if p.exists() { return p; }
            }
            // 3. Try /usr/share/warehouse-vadini (installed system default)
            let p = PathBuf::from("/usr/share/warehouse-vadini/config.json");
            if p.exists() { return p; }

            // 4. Local fallback
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.join("config.json")))
                .unwrap_or_else(|| PathBuf::from("config.json"))
        }
    }

    /// Path used for saving — always writes to the user's writable config dir,
    /// never to system paths like /usr/share which require root.
    fn save_path() -> PathBuf {
        #[cfg(target_os = "android")]
        {
            crate::ANDROID_DATA_DIR
                .get()
                .map(|p| p.join("config.json"))
                .unwrap_or_else(|| PathBuf::from("/data/data/ro.vadini.warehouse/files/config.json"))
        }
        #[cfg(not(target_os = "android"))]
        {
            // Windows: use %APPDATA%\warehouse-vadini\config.json
            #[cfg(target_os = "windows")]
            if let Ok(appdata) = std::env::var("APPDATA") {
                return PathBuf::from(appdata).join("warehouse-vadini/config.json");
            }

            if let Ok(cfg_home) = std::env::var("XDG_CONFIG_HOME") {
                return PathBuf::from(cfg_home).join("warehouse-vadini/config.json");
            }
            if let Ok(home) = std::env::var("HOME") {
                return PathBuf::from(home).join(".config/warehouse-vadini/config.json");
            }
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.join("config.json")))
                .unwrap_or_else(|| PathBuf::from("config.json"))
        }
    }
}
