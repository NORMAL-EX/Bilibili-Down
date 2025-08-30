use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Language {
    SimplifiedChinese,
    English,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub theme: Theme,
    pub language: Language,
    pub download_threads: u32,
    pub download_path: PathBuf,
    pub cookies: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        let download_path = dirs::download_dir()
            .unwrap_or_else(|| PathBuf::from("C:\\"))
            .join("Bilidown");
        
        if !download_path.exists() {
            let _ = fs::create_dir_all(&download_path);
        }
        
        Self {
            theme: Theme::System,
            language: Language::SimplifiedChinese,
            download_threads: 32,
            download_path,
            cookies: None,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = Self::config_path();
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        
        let config = Self::default();
        config.save();
        config
    }
    
    pub fn save(&self) {
        let config_path = Self::config_path();
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(config_path, content);
        }
    }
    
    fn config_path() -> PathBuf {
        std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("."))
            .parent()
            .unwrap_or(&PathBuf::from("."))
            .join("config.json")
    }
}