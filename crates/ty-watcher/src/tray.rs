use std::sync::{Arc, Mutex};

#[cfg(windows)]
use std::cell::RefCell;
#[cfg(windows)]
use tray_icon::{
    menu::{Menu, MenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

pub const MENU_ID_OPEN_UI: &str = "open_ui";
pub const MENU_ID_TOTAL_TIME: &str = "total_time";
pub const MENU_ID_QUIT: &str = "quit";

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

/// A cloneable, thread-safe handle to update tray display text from background threads.
#[derive(Clone)]
pub struct TrayUpdater {
    total_text: Arc<Mutex<String>>,
}

impl TrayUpdater {
    pub fn new(total_text: Arc<Mutex<String>>) -> Self {
        Self { total_text }
    }

    pub fn set_total_text(&self, text: &str) {
        if let Ok(mut lock) = self.total_text.lock() {
            *lock = text.to_string();
        }
    }

    pub fn current_total_text(&self) -> String {
        self.total_text.lock().map(|t| t.clone()).unwrap_or_default()
    }
}

/// Tray manager hosting the notification icon, context menu, and active time updates.
///
/// NOTE: Windows-specific tray and message loop logic is UNTESTED until executed on Windows.
pub struct TrayManager {
    total_text: Arc<Mutex<String>>,
    callbacks: Arc<Mutex<TrayCallbacks>>,
    #[cfg(windows)]
    _tray_icon: Option<TrayIcon>,
    #[cfg(windows)]
    total_item: Option<MenuItem>,
}

impl TrayManager {
    /// Creates a new TrayManager with callbacks for opening the UI and quitting.
    ///
    /// NOTE: Windows-specific logic is UNTESTED until executed on Windows.
    pub fn new(callbacks: TrayCallbacks) -> Self {
        let initial_text = "Today: --".to_string();
        let total_text = Arc::new(Mutex::new(initial_text.clone()));
        let callbacks = Arc::new(Mutex::new(callbacks));

        #[cfg(windows)]
        {
            let (tray_icon, total_item) = create_windows_tray(&initial_text);
            Self {
                total_text,
                callbacks,
                _tray_icon: tray_icon,
                total_item,
            }
        }
        #[cfg(not(windows))]
        {
            Self {
                total_text,
                callbacks,
            }
        }
    }

    /// Obtains a thread-safe updater handle to update the tray menu line from background threads.
    pub fn updater(&self) -> TrayUpdater {
        TrayUpdater::new(Arc::clone(&self.total_text))
    }

    /// Sets and updates the text for today's total active time line directly.
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

    /// Returns the currently set total active time string.
    pub fn current_total_text(&self) -> String {
        self.total_text.lock().map(|t| t.clone()).unwrap_or_default()
    }

    /// Runs the message loop on the calling thread (blocks until Quit is selected).
    ///
    /// NOTE: Windows-specific logic is UNTESTED until executed on Windows.
    pub fn run(mut self) {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                DispatchMessageW, GetMessageW, KillTimer, PostQuitMessage, SetTimer,
                TranslateMessage, MSG,
            };

            let callbacks_menu = Arc::clone(&self.callbacks);
            tray_icon::menu::MenuEvent::set_event_handler(Some(move |event: tray_icon::menu::MenuEvent| {
                let action_id = event.id().as_ref();
                if let Ok(cb) = callbacks_menu.lock() {
                    match action_id {
                        MENU_ID_OPEN_UI => {
                            (cb.on_open_ui)();
                        }
                        MENU_ID_QUIT => {
                            (cb.on_quit)();
                            unsafe {
                                PostQuitMessage(0);
                            }
                        }
                        _ => {}
                    }
                }
            }));

            // Left-click on tray icon opens the UI
            let callbacks_click = Arc::clone(&self.callbacks);
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

            // Store items on main thread for timer-based menu updates
            ACTIVE_ITEM.with(|c| *c.borrow_mut() = self.total_item.take());
            ACTIVE_TEXT.with(|c| *c.borrow_mut() = Some(Arc::clone(&self.total_text)));

            let timer_id = unsafe { SetTimer(0, 0, 1000, Some(timer_callback)) };

            // Message pump owning the thread
            unsafe {
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                KillTimer(0, timer_id);
            }

            ACTIVE_ITEM.with(|c| *c.borrow_mut() = None);
            ACTIVE_TEXT.with(|c| *c.borrow_mut() = None);
        }
        #[cfg(not(windows))]
        {
            // Non-windows stub: no GUI message pump
        }
    }
}

#[cfg(windows)]
thread_local! {
    static ACTIVE_ITEM: RefCell<Option<MenuItem>> = RefCell::new(None);
    static ACTIVE_TEXT: RefCell<Option<Arc<Mutex<String>>>> = RefCell::new(None);
    static LAST_DISPLAYED: RefCell<String> = RefCell::new(String::new());
}

#[cfg(windows)]
unsafe extern "system" fn timer_callback(
    _hwnd: windows_sys::Win32::Foundation::HWND,
    _msg: u32,
    _id: usize,
    _time: u32,
) {
    ACTIVE_ITEM.with(|item_cell| {
        ACTIVE_TEXT.with(|text_cell| {
            LAST_DISPLAYED.with(|last_cell| {
                if let (Some(item), Some(shared_text)) = (
                    item_cell.borrow().as_ref(),
                    text_cell.borrow().as_ref(),
                ) {
                    if let Ok(cur) = shared_text.lock() {
                        let mut last = last_cell.borrow_mut();
                        if *last != *cur {
                            item.set_text(cur.as_str());
                            *last = cur.clone();
                        }
                    }
                }
            });
        });
    });
}

#[cfg(windows)]
fn create_default_icon() -> Option<Icon> {
    let width = 16;
    let height = 16;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            if x >= 2 && x < 14 && y >= 2 && y < 14 {
                // Blue square icon
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_manager_initial_state() {
        let callbacks = TrayCallbacks::new(|| {}, || {});
        let tray = TrayManager::new(callbacks);
        assert_eq!(tray.current_total_text(), "Today: --");

        let updater = tray.updater();
        updater.set_total_text("Today: 2h 15m");
        assert_eq!(tray.current_total_text(), "Today: 2h 15m");
        assert_eq!(updater.current_total_text(), "Today: 2h 15m");
    }
}
