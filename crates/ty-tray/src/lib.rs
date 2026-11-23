pub struct TrayCallbacks {
    pub on_open_ui: Box<dyn Fn() + Send + 'static>,
    pub on_quit: Box<dyn Fn() + Send + 'static>,
}

pub struct Tray {
    _callbacks: TrayCallbacks,
}

impl Tray {
    pub fn new(callbacks: TrayCallbacks) -> Self {
        Self {
            _callbacks: callbacks,
        }
    }

    /// Sets the text for the disabled line showing today's total active time.
    ///
    /// NOTE: Windows-specific logic is UNTESTED until executed on Windows.
    pub fn set_total_text(&self, _text: &str) {
        #[cfg(windows)]
        {
            // Untested on Windows stub
            todo!("Tray::set_total_text (untested Windows implementation)")
        }
    }

    /// Runs the tray message loop (blocks the thread).
    ///
    /// NOTE: Windows-specific logic is UNTESTED until executed on Windows.
    pub fn run(self) {
        #[cfg(windows)]
        {
            // Untested on Windows stub
            todo!("Tray::run (untested Windows implementation)")
        }
    }
}

/// Enables autostart via HKCU\Software\Microsoft\Windows\CurrentVersion\Run.
///
/// NOTE: Windows-specific logic is UNTESTED until executed on Windows.
pub fn enable_autostart() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(windows)]
    {
        todo!("enable_autostart (untested Windows implementation)")
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}

/// Disables autostart via HKCU\Software\Microsoft\Windows\CurrentVersion\Run.
///
/// NOTE: Windows-specific logic is UNTESTED until executed on Windows.
pub fn disable_autostart() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(windows)]
    {
        todo!("disable_autostart (untested Windows implementation)")
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}

/// Checks if autostart is enabled in HKCU\Software\Microsoft\Windows\CurrentVersion\Run.
///
/// NOTE: Windows-specific logic is UNTESTED until executed on Windows.
pub fn is_autostart_enabled() -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(windows)]
    {
        todo!("is_autostart_enabled (untested Windows implementation)")
    }
    #[cfg(not(windows))]
    {
        Ok(false)
    }
}

/// Launches `tally-ui.exe` located next to the running executable.
///
/// NOTE: Windows-specific logic is UNTESTED until executed on Windows.
pub fn open_ui() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(windows)]
    {
        todo!("open_ui (untested Windows implementation)")
    }
    #[cfg(not(windows))]
    {
        Ok(())
    }
}
