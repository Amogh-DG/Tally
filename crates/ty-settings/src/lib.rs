use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub afk_timeout_secs: u64,
    pub day_start: String,
    pub port: u16,
    pub extension_origins: Vec<String>,
    pub data_dir: PathBuf,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            afk_timeout_secs: 180,
            day_start: "00:00".to_string(),
            port: 5600,
            extension_origins: Vec::new(),
            data_dir: PathBuf::from("%APPDATA%\\Tally\\data"),
        }
    }
}

pub fn load() -> Result<Settings, Box<dyn std::error::Error + Send + Sync>> {
    todo!("ty_settings::load not yet implemented")
}

pub fn save(_settings: &Settings) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    todo!("ty_settings::save not yet implemented")
}

pub fn settings_path() -> PathBuf {
    todo!("ty_settings::settings_path not yet implemented")
}
