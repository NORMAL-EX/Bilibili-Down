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
            .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
            .unwrap_or_else(|| PathBuf::from("."))
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
        // 确保配置目录存在
        if let Some(parent) = config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(config_path, content);
        }
    }
    
    fn config_path() -> PathBuf {
        // 优先使用 XDG/AppData 标准配置目录
        if let Some(config_dir) = dirs::config_dir() {
            let app_config = config_dir.join("bilibili-down");
            let _ = fs::create_dir_all(&app_config);
            return app_config.join("config.json");
        }
        
        // 回退：使用可执行文件旁边
        std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("."))
            .parent()
            .unwrap_or(&PathBuf::from("."))
            .join("config.json")
    }
}
