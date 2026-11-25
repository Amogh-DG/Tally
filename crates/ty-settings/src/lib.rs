use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
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
            data_dir: PathBuf::from(r"%APPDATA%\Tally\data"),
        }
    }
}

/// Returns the default path to the settings file (`%APPDATA%\Tally\settings.json` on Windows).
pub fn settings_path() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata).join("Tally").join("settings.json");
        }
        if let Some(userprofile) = std::env::var_os("USERPROFILE") {
            return PathBuf::from(userprofile)
                .join("AppData")
                .join("Roaming")
                .join("Tally")
                .join("settings.json");
        }
    }
    #[cfg(not(windows))]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(".tally").join("settings.json");
        }
    }
    PathBuf::from("settings.json")
}

/// Loads settings from the default settings path.
/// If the file is missing, creates it with default settings and returns them.
pub fn load() -> Result<Settings, Box<dyn std::error::Error + Send + Sync>> {
    load_from_path(settings_path())
}

/// Saves settings to the default settings path.
pub fn save(settings: &Settings) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    save_to_path(settings, settings_path())
}

/// Loads settings from a specific path.
/// If the file does not exist, writes the default settings to the path and returns them.
/// Tolerates missing fields (fills from defaults) and extra fields in JSON.
pub fn load_from_path(path: impl AsRef<Path>) -> Result<Settings, Box<dyn std::error::Error + Send + Sync>> {
    let path = path.as_ref();
    if !path.exists() {
        let defaults = Settings::default();
        save_to_path(&defaults, path)?;
        return Ok(defaults);
    }

    let content = std::fs::read_to_string(path)?;
    let settings: Settings = serde_json::from_str(&content)?;
    Ok(settings)
}

/// Saves settings to a specific path, creating any missing parent directories.
pub fn save_to_path(settings: &Settings, path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let json = serde_json::to_string_pretty(settings)?;
    std::fs::write(path, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct TempFileGuard {
        path: PathBuf,
    }

    impl TempFileGuard {
        fn new(name: &str) -> Self {
            let mut path = std::env::temp_dir();
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            path.push(format!("tally_test_{}_{}.json", name, nonce));
            Self { path }
        }
    }

    impl Drop for TempFileGuard {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }

    #[test]
    fn test_defaults() {
        let settings = Settings::default();
        assert_eq!(settings.afk_timeout_secs, 180);
        assert_eq!(settings.day_start, "00:00");
        assert_eq!(settings.port, 5600);
        assert!(settings.extension_origins.is_empty());
        assert_eq!(settings.data_dir, PathBuf::from(r"%APPDATA%\Tally\data"));
    }

    #[test]
    fn test_load_creates_default_when_missing() {
        let guard = TempFileGuard::new("missing_file");
        assert!(!guard.path.exists());

        let loaded = load_from_path(&guard.path).expect("should create and load defaults");
        assert_eq!(loaded, Settings::default());
        assert!(guard.path.exists());

        // Second load returns the saved file content
        let reloaded = load_from_path(&guard.path).expect("should reload saved settings");
        assert_eq!(reloaded, Settings::default());
    }

    #[test]
    fn test_round_trip() {
        let guard = TempFileGuard::new("round_trip");
        let custom = Settings {
            afk_timeout_secs: 300,
            day_start: "04:00".to_string(),
            port: 5700,
            extension_origins: vec!["moz-extension://test-id".to_string()],
            data_dir: PathBuf::from("D:\\custom\\tally\\data"),
        };

        save_to_path(&custom, &guard.path).expect("save should succeed");
        let loaded = load_from_path(&guard.path).expect("load should succeed");
        assert_eq!(loaded, custom);
    }

    #[test]
    fn test_missing_fields_tolerated() {
        let guard = TempFileGuard::new("missing_fields");
        // JSON missing afk_timeout_secs, day_start, extension_origins, and data_dir
        let partial_json = r#"{
            "port": 8080
        }"#;
        fs::write(&guard.path, partial_json).expect("write file");

        let loaded = load_from_path(&guard.path).expect("should deserialize with defaults for missing fields");
        assert_eq!(loaded.port, 8080);
        assert_eq!(loaded.afk_timeout_secs, 180);
        assert_eq!(loaded.day_start, "00:00");
        assert!(loaded.extension_origins.is_empty());
        assert_eq!(loaded.data_dir, PathBuf::from(r"%APPDATA%\Tally\data"));
    }

    #[test]
    fn test_extra_fields_tolerated() {
        let guard = TempFileGuard::new("extra_fields");
        // JSON containing unexpected fields
        let extra_json = r#"{
            "port": 5600,
            "afk_timeout_secs": 240,
            "day_start": "01:00",
            "extension_origins": [],
            "data_dir": "%APPDATA%\\Tally\\data",
            "unknown_bool": true,
            "extra_object": { "a": 1, "b": "c" },
            "obsolete_field": 12345
        }"#;
        fs::write(&guard.path, extra_json).expect("write file");

        let loaded = load_from_path(&guard.path).expect("should ignore extra fields");
        assert_eq!(loaded.port, 5600);
        assert_eq!(loaded.afk_timeout_secs, 240);
        assert_eq!(loaded.day_start, "01:00");
    }

    #[test]
    fn test_matches_fixture() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("docs")
            .join("fixtures")
            .join("settings.json");

        if fixture_path.exists() {
            let content = fs::read_to_string(&fixture_path).expect("read fixture");
            let parsed: Settings = serde_json::from_str(&content).expect("parse fixture");
            assert_eq!(parsed, Settings::default());
        }
    }
}
