use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AfkStatus {
    Afk,
    NotAfk,
}

pub struct AfkMonitor {
    pub timeout_secs: u64,
}

impl AfkMonitor {
    pub fn new(timeout_secs: u64) -> Self {
        Self { timeout_secs }
    }

    pub fn set_timeout(&mut self, secs: u64) {
        self.timeout_secs = secs;
    }

    /// Polls the user's AFK status.
    ///
    /// NOTE: Windows-specific logic is UNTESTED until executed on Windows.
    pub fn poll(&mut self) -> AfkStatus {
        #[cfg(windows)]
        {
            // Untested on Windows stub
            todo!("AfkMonitor::poll (untested Windows implementation)")
        }
        #[cfg(not(windows))]
        {
            AfkStatus::NotAfk
        }
    }

    /// Returns idle time in seconds.
    ///
    /// NOTE: Windows-specific logic is UNTESTED until executed on Windows.
    pub fn idle_secs(&self) -> f64 {
        #[cfg(windows)]
        {
            // Untested on Windows stub
            todo!("AfkMonitor::idle_secs (untested Windows implementation)")
        }
        #[cfg(not(windows))]
        {
            0.0
        }
    }
}
