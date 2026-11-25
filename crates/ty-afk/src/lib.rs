use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AfkStatus {
    #[serde(rename = "afk")]
    Afk,
    #[serde(rename = "not-afk")]
    NotAfk,
}

/// Computes elapsed idle milliseconds handling 32-bit millisecond tick wraparound.
pub fn calculate_idle_ms(current_tick: u32, last_input_tick: u32) -> u32 {
    current_tick.wrapping_sub(last_input_tick)
}

/// Converts idle milliseconds to idle seconds as f64.
pub fn idle_ms_to_secs(idle_ms: u32) -> f64 {
    idle_ms as f64 / 1000.0
}

/// Determines AfkStatus based on idle seconds and timeout seconds.
pub fn determine_afk_status(idle_secs: f64, timeout_secs: u64) -> AfkStatus {
    if idle_secs >= timeout_secs as f64 {
        AfkStatus::Afk
    } else {
        AfkStatus::NotAfk
    }
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
        let idle = self.idle_secs();
        determine_afk_status(idle, self.timeout_secs)
    }

    /// Returns idle time in seconds.
    ///
    /// Uses Win32 `GetLastInputInfo` and `GetTickCount` on Windows.
    /// Handles tick count wraparound.
    /// NOTE: Windows-specific logic is UNTESTED until executed on Windows.
    pub fn idle_secs(&self) -> f64 {
        #[cfg(windows)]
        {
            get_windows_idle_secs()
        }
        #[cfg(not(windows))]
        {
            0.0
        }
    }
}

#[cfg(windows)]
fn get_windows_idle_secs() -> f64 {
    use windows::Win32::System::SystemInformation::GetTickCount;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    unsafe {
        let mut lii = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if GetLastInputInfo(&mut lii).as_bool() {
            let current_tick = GetTickCount();
            let idle_ms = calculate_idle_ms(current_tick, lii.dwTime);
            idle_ms_to_secs(idle_ms)
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wraparound_calculation() {
        // Normal case without wraparound
        assert_eq!(calculate_idle_ms(10_000, 5_000), 5_000);
        assert_eq!(calculate_idle_ms(1_000, 1_000), 0);

        // Wraparound case:
        // last_input was right before u32::MAX (e.g. 0xFFFF_FFF0 = 4294967280)
        // current_tick has wrapped past 0 (e.g. 0x0000_0020 = 32)
        // Elapsed ticks should be 16 + 32 = 48
        let last_input = 0xFFFF_FFF0u32;
        let current = 0x0000_0020u32;
        assert_eq!(calculate_idle_ms(current, last_input), 48);

        // Another wraparound test with larger delta
        let last_input_2 = u32::MAX - 999;
        let current_2 = 1000;
        // (u32::MAX - 999) to u32::MAX is 1000 ticks. Then + 1000 ticks = 2000 ticks.
        assert_eq!(calculate_idle_ms(current_2, last_input_2), 2000);
    }

    #[test]
    fn test_idle_ms_to_secs() {
        assert_eq!(idle_ms_to_secs(0), 0.0);
        assert_eq!(idle_ms_to_secs(1000), 1.0);
        assert_eq!(idle_ms_to_secs(1500), 1.5);
        assert_eq!(idle_ms_to_secs(180_000), 180.0);
    }

    #[test]
    fn test_determine_afk_status() {
        let timeout = 180;
        assert_eq!(determine_afk_status(0.0, timeout), AfkStatus::NotAfk);
        assert_eq!(determine_afk_status(179.9, timeout), AfkStatus::NotAfk);
        assert_eq!(determine_afk_status(180.0, timeout), AfkStatus::Afk);
        assert_eq!(determine_afk_status(180.1, timeout), AfkStatus::Afk);
        assert_eq!(determine_afk_status(3600.0, timeout), AfkStatus::Afk);
    }

    #[test]
    fn test_afk_monitor_timeout_update() {
        let mut monitor = AfkMonitor::new(180);
        assert_eq!(monitor.timeout_secs, 180);
        monitor.set_timeout(300);
        assert_eq!(monitor.timeout_secs, 300);
    }

    #[test]
    fn test_afk_monitor_poll_non_windows_or_initial() {
        // Since we are running in tests, poll will use idle_secs()
        let mut monitor = AfkMonitor::new(100_000); // 100,000s timeout
        assert_eq!(monitor.poll(), AfkStatus::NotAfk);
    }
}
