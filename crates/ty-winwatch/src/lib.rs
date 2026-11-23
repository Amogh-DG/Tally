use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowSample {
    pub app: String,
    pub title: String,
}

pub struct WindowWatcher;

impl WindowWatcher {
    pub fn new() -> Self {
        Self
    }

    /// Polls the current foreground window.
    ///
    /// Returns `None` if there is no foreground window (e.g. lock screen, UAC, secure desktop).
    /// NOTE: Windows-specific logic is UNTESTED until executed on Windows.
    pub fn poll(&mut self) -> Option<WindowSample> {
        #[cfg(windows)]
        {
            // Untested on Windows stub
            todo!("WindowWatcher::poll (untested Windows implementation)")
        }
        #[cfg(not(windows))]
        {
            None
        }
    }
}

impl Default for WindowWatcher {
    fn default() -> Self {
        Self::new()
    }
}
