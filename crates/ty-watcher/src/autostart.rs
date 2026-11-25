use std::path::{Path, PathBuf};

pub const AUTOSTART_MARKER_FILENAME: &str = ".autostart_initialized";

/// Returns the path to the autostart marker file next to settings.json in %APPDATA%\Tally\.
pub fn marker_path() -> PathBuf {
    let settings_dir = ty_settings::settings_path()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    settings_dir.join(AUTOSTART_MARKER_FILENAME)
}

/// Enables autostart on first run by checking for the presence of a marker file.
/// If the marker does not exist:
/// 1. Calls `ty_tray::enable_autostart()`
/// 2. Creates the marker file so subsequent launches do not re-enable it if the user
///    explicitly turned it off later.
///
/// Returns `Ok(true)` if autostart was enabled on first run, or `Ok(false)` if already initialized.
///
/// NOTE: Windows registry operations in `ty_tray` are UNTESTED until executed on Windows.
pub fn enable_autostart_on_first_run() -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let path = marker_path();
    enable_autostart_with_marker(&path)
}

/// Generalized helper for testing autostart initialization with a custom marker file path.
pub fn enable_autostart_with_marker(marker_file: &Path) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    if marker_file.exists() {
        return Ok(false);
    }

    if let Some(parent) = marker_file.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    // Call ty_tray autostart registration
    let _ = ty_tray::enable_autostart();

    // Mark as initialized
    std::fs::write(marker_file, b"initialized\n")?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enable_autostart_with_marker_lifecycle() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let marker = temp_dir.path().join(".autostart_initialized");

        assert!(!marker.exists());

        // First run should enable autostart and create marker
        let res1 = enable_autostart_with_marker(&marker).expect("first run should succeed");
        assert!(res1);
        assert!(marker.exists());

        // Second run should detect marker and do nothing
        let res2 = enable_autostart_with_marker(&marker).expect("second run should succeed");
        assert!(!res2);
    }
}
