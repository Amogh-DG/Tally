use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[cfg(windows)]
use tray_icon::{
    menu::{Menu, MenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

pub struct TrayCallbacks {
    pub on_open_ui: Box<dyn Fn() + Send + 'static>,
    pub on_quit: Box<dyn Fn() + Send + 'static>,
}

impl TrayCallbacks {
    pub fn new(
        on_open_ui: impl Fn() + Send + 'static,
        on_quit: impl Fn() + Send + 'static,
    ) -> Self {
        Self {
            on_open_ui: Box::new(on_open_ui),
            on_quit: Box::new(on_quit),
        }
    }
}

pub struct Tray {
    callbacks: TrayCallbacks,
    total_text: Arc<Mutex<String>>,
    #[cfg(windows)]
    total_item: Option<MenuItem>,
    #[cfg(windows)]
    _tray_icon: Option<TrayIcon>,
}

pub const MENU_ID_OPEN_UI: &str = "open_ui";
pub const MENU_ID_TOTAL_TIME: &str = "total_time";
pub const MENU_ID_QUIT: &str = "quit";

impl Tray {
    /// Creates a new Tray instance with callbacks for opening UI and quitting.
    ///
    /// NOTE: Windows-specific logic is UNTESTED until executed on Windows.
    pub fn new(callbacks: TrayCallbacks) -> Self {
        let initial_text = "Today: --".to_string();
        let total_text = Arc::new(Mutex::new(initial_text.clone()));

        #[cfg(windows)]
        {
            let (tray_icon, total_item) = create_windows_tray(&initial_text);
            Self {
                callbacks,
                total_text,
                total_item,
                _tray_icon: tray_icon,
            }
        }
        #[cfg(not(windows))]
        {
            Self {
                callbacks,
                total_text,
            }
        }
    }

    /// Sets the text for the disabled line showing today's total active time.
    ///
    /// NOTE: Windows-specific logic is UNTESTED until executed on Windows.
    pub fn set_total_text(&self, text: &str) {
        if let Ok(mut lock) = self.total_text.lock() {
            *lock = text.to_string();
        }
        #[cfg(windows)]
        {
            if let Some(item) = &self.total_item {
                item.set_text(text);
            }
        }
    }

    /// Returns the currently set total active time text.
    pub fn current_total_text(&self) -> String {
        self.total_text.lock().map(|t| t.clone()).unwrap_or_default()
    }

    /// Runs the tray message loop (blocks the thread and owns the loop).
    ///
    /// NOTE: Windows-specific logic is UNTESTED until executed on Windows.
    pub fn run(self) {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                DispatchMessageW, GetMessageW, PostQuitMessage, TranslateMessage, MSG,
            };

            let callbacks = Arc::new(Mutex::new(self.callbacks));

            // Set up menu event handler
            let callbacks_menu = Arc::clone(&callbacks);
            tray_icon::menu::MenuEvent::set_event_handler(Some(move |event: tray_icon::menu::MenuEvent| {
                let action_id = event.id().as_ref();
                if let Ok(cb) = callbacks_menu.lock() {
                    handle_menu_action(action_id, &cb, || unsafe {
                        PostQuitMessage(0);
                    });
                }
            }));

            // Optional left click handler on tray icon to open the UI
            let callbacks_click = Arc::clone(&callbacks);
            tray_icon::TrayIconEvent::set_event_handler(Some(move |event: tray_icon::TrayIconEvent| {
                if let tray_icon::TrayIconEvent::Click {
                    button: tray_icon::MouseButton::Left,
                    button_state: tray_icon::MouseButtonState::Up,
                    ..
                } = event
                {
                    if let Ok(cb) = callbacks_click.lock() {
                        (cb.on_open_ui)();
                    }
                }
            }));

            // Message loop owning the thread
            unsafe {
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
        #[cfg(not(windows))]
        {
            // Non-windows stub: no display/loop to block on
        }
    }
}

/// Dispatches menu actions to the respective callbacks.
pub fn handle_menu_action(
    action_id: &str,
    callbacks: &TrayCallbacks,
    on_exit: impl FnOnce(),
) {
    match action_id {
        MENU_ID_OPEN_UI => {
            (callbacks.on_open_ui)();
        }
        MENU_ID_QUIT => {
            (callbacks.on_quit)();
            on_exit();
        }
        _ => {}
    }
}

#[cfg(windows)]
fn create_default_icon() -> Option<Icon> {
    let width = 16;
    let height = 16;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            if x >= 2 && x < 14 && y >= 2 && y < 14 {
                // Blue square icon for Tally
                rgba.extend_from_slice(&[41, 128, 185, 255]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    Icon::from_rgba(rgba, width, height).ok()
}

#[cfg(windows)]
fn create_windows_tray(initial_total_text: &str) -> (Option<TrayIcon>, Option<MenuItem>) {
    let menu = Menu::new();
    let item_open = MenuItem::with_id(MENU_ID_OPEN_UI, "Open the UI", true, None);
    let item_total = MenuItem::with_id(MENU_ID_TOTAL_TIME, initial_total_text, false, None);
    let item_quit = MenuItem::with_id(MENU_ID_QUIT, "Quit", true, None);

    let _ = menu.append(&item_open);
    let _ = menu.append(&item_total);
    let _ = menu.append(&item_quit);

    let mut builder = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Tally");

    if let Some(icon) = create_default_icon() {
        builder = builder.with_icon(icon);
    }

    let tray_icon = builder.build().ok();
    (tray_icon, Some(item_total))
}

const REG_RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const REG_APP_NAME: &str = "Tally";

/// Enables autostart via HKCU\Software\Microsoft\Windows\CurrentVersion\Run.
///
/// NOTE: Windows-specific logic is UNTESTED until executed on Windows.
pub fn enable_autostart() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe()?;
        let exe_str = exe.to_str().ok_or("invalid current exe path unicode")?;
        let cmd = if exe_str.contains(' ') && !exe_str.starts_with('"') {
            format!("\"{}\"", exe_str)
        } else {
            exe_str.to_string()
        };
        enable_autostart_in(REG_RUN_KEY, REG_APP_NAME, &cmd)
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
        disable_autostart_in(REG_RUN_KEY, REG_APP_NAME)
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
        is_autostart_enabled_in(REG_RUN_KEY, REG_APP_NAME)
    }
    #[cfg(not(windows))]
    {
        Ok(false)
    }
}

#[cfg(windows)]
pub fn enable_autostart_in(
    subkey: &str,
    value_name: &str,
    command: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(subkey)?;
    key.set_value(value_name, &command)?;
    Ok(())
}

#[cfg(windows)]
pub fn disable_autostart_in(
    subkey: &str,
    value_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags(subkey, KEY_SET_VALUE) {
        match key.delete_value(value_name) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(Box::new(e)),
        }
    } else {
        Ok(())
    }
}

#[cfg(windows)]
pub fn is_autostart_enabled_in(
    subkey: &str,
    value_name: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags(subkey, KEY_READ) {
        match key.get_value::<String, _>(value_name) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(Box::new(e)),
        }
    } else {
        Ok(false)
    }
}

/// Returns the expected path to `tally-ui.exe` (or `tally-ui` on Unix) located next to the running executable.
pub fn ui_path() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let current_exe = std::env::current_exe()?;
    let exe_dir = current_exe
        .parent()
        .ok_or("cannot find parent directory of current executable")?;
    #[cfg(windows)]
    let ui_filename = "tally-ui.exe";
    #[cfg(not(windows))]
    let ui_filename = "tally-ui";

    Ok(exe_dir.join(ui_filename))
}

/// Launches `tally-ui.exe` located next to the running executable.
///
/// NOTE: Windows-specific logic is UNTESTED until executed on Windows.
pub fn open_ui() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = ui_path()?;
    #[cfg(windows)]
    {
        std::process::Command::new(path).spawn()?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new(path).spawn()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn test_menu_action_open_ui() {
        let open_called = Arc::new(AtomicBool::new(false));
        let quit_called = Arc::new(AtomicBool::new(false));
        let exit_called = Arc::new(AtomicBool::new(false));

        let open_clone = Arc::clone(&open_called);
        let quit_clone = Arc::clone(&quit_called);

        let callbacks = TrayCallbacks::new(
            move || open_clone.store(true, Ordering::SeqCst),
            move || quit_clone.store(true, Ordering::SeqCst),
        );

        let exit_clone = Arc::clone(&exit_called);
        handle_menu_action(MENU_ID_OPEN_UI, &callbacks, move || {
            exit_clone.store(true, Ordering::SeqCst);
        });

        assert!(open_called.load(Ordering::SeqCst));
        assert!(!quit_called.load(Ordering::SeqCst));
        assert!(!exit_called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_menu_action_quit() {
        let open_called = Arc::new(AtomicBool::new(false));
        let quit_called = Arc::new(AtomicBool::new(false));
        let exit_called = Arc::new(AtomicBool::new(false));

        let open_clone = Arc::clone(&open_called);
        let quit_clone = Arc::clone(&quit_called);

        let callbacks = TrayCallbacks::new(
            move || open_clone.store(true, Ordering::SeqCst),
            move || quit_clone.store(true, Ordering::SeqCst),
        );

        let exit_clone = Arc::clone(&exit_called);
        handle_menu_action(MENU_ID_QUIT, &callbacks, move || {
            exit_clone.store(true, Ordering::SeqCst);
        });

        assert!(!open_called.load(Ordering::SeqCst));
        assert!(quit_called.load(Ordering::SeqCst));
        assert!(exit_called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_menu_action_disabled_total_line_ignored() {
        let open_called = Arc::new(AtomicBool::new(false));
        let quit_called = Arc::new(AtomicBool::new(false));
        let exit_called = Arc::new(AtomicBool::new(false));

        let open_clone = Arc::clone(&open_called);
        let quit_clone = Arc::clone(&quit_called);

        let callbacks = TrayCallbacks::new(
            move || open_clone.store(true, Ordering::SeqCst),
            move || quit_clone.store(true, Ordering::SeqCst),
        );

        let exit_clone = Arc::clone(&exit_called);
        handle_menu_action(MENU_ID_TOTAL_TIME, &callbacks, move || {
            exit_clone.store(true, Ordering::SeqCst);
        });

        assert!(!open_called.load(Ordering::SeqCst));
        assert!(!quit_called.load(Ordering::SeqCst));
        assert!(!exit_called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_tray_set_total_text() {
        let callbacks = TrayCallbacks::new(|| {}, || {});
        let tray = Tray::new(callbacks);

        assert_eq!(tray.current_total_text(), "Today: --");
        tray.set_total_text("Today: 3h 42m");
        assert_eq!(tray.current_total_text(), "Today: 3h 42m");
    }

    #[test]
    fn test_ui_path_points_next_to_current_exe() {
        let path = ui_path().expect("ui_path should resolve");
        let current_exe = std::env::current_exe().expect("current exe");
        assert_eq!(path.parent(), current_exe.parent());
        #[cfg(windows)]
        assert_eq!(path.file_name().unwrap(), "tally-ui.exe");
        #[cfg(not(windows))]
        assert_eq!(path.file_name().unwrap(), "tally-ui");
    }

    #[cfg(windows)]
    #[test]
    fn test_registry_autostart_isolated_key() {
        let test_subkey = r"Software\Tally_Test_Run_Key";
        let test_val = "TestApp";
        let test_cmd = r#""C:\Program Files\Tally\tally.exe""#;

        // Clean up beforehand just in case
        let _ = disable_autostart_in(test_subkey, test_val);

        assert!(!is_autostart_enabled_in(test_subkey, test_val).unwrap());

        enable_autostart_in(test_subkey, test_val, test_cmd).expect("enable should succeed");
        assert!(is_autostart_enabled_in(test_subkey, test_val).unwrap());

        disable_autostart_in(test_subkey, test_val).expect("disable should succeed");
        assert!(!is_autostart_enabled_in(test_subkey, test_val).unwrap());

        // Clean up test subkey
        use winreg::enums::*;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let _ = hkcu.delete_subkey(test_subkey);
    }
}
