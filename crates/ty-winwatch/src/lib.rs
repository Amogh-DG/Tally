use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowSample {
    pub app: String,
    pub title: String,
}

/// Window watcher that polls the active foreground window.
pub struct WindowWatcher;

impl WindowWatcher {
    pub fn new() -> Self {
        Self
    }

    /// Polls the current foreground window.
    ///
    /// Returns `None` if there is no foreground window (e.g. lock screen, UAC prompt,
    /// secure desktop, screen saver).
    ///
    /// NOTE: Windows-specific logic is UNTESTED until executed on Windows.
    pub fn poll(&mut self) -> Option<WindowSample> {
        #[cfg(windows)]
        {
            poll_windows()
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

/// Extracts the file name component from an executable path (e.g. "firefox.exe").
pub fn extract_file_name(path: &str) -> &str {
    let clean = path.trim();
    if clean.is_empty() {
        return "";
    }
    match clean.rsplit(|c| c == '\\' || c == '/').next() {
        Some(name) => name,
        None => clean,
    }
}

/// Given a host PID, a list of child PIDs, and an app resolver callback,
/// finds the first child whose PID differs from the host's and resolves to a valid app
/// (non-empty, not "unknown", and not "ApplicationFrameHost.exe").
///
/// Returns `Some(child_app)` if found, or `None` if none has been resolved yet.
pub fn find_uwp_child_app<F>(host_pid: u32, child_pids: &[u32], mut get_app_for_pid: F) -> Option<String>
where
    F: FnMut(u32) -> String,
{
    for &child_pid in child_pids {
        if child_pid != 0 && child_pid != host_pid {
            let app = get_app_for_pid(child_pid);
            if !app.is_empty()
                && app != "unknown"
                && !app.eq_ignore_ascii_case("ApplicationFrameHost.exe")
            {
                return Some(app);
            }
        }
    }
    None
}

#[cfg(windows)]
fn poll_windows() -> Option<WindowSample> {
    use windows::Win32::Foundation::{
        GetLastError, SetLastError, ERROR_SUCCESS, WIN32_ERROR,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }

        let mut host_pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut host_pid));

        let mut app = get_process_app_name(host_pid);

        // UWP apps: if the app is "ApplicationFrameHost.exe", enumerate child windows
        // and find a child whose PID differs from the host's.
        if app.eq_ignore_ascii_case("ApplicationFrameHost.exe") {
            if let Some(child_app) = get_uwp_child_app(hwnd, host_pid) {
                app = child_app;
            }
        }

        // Title: GetWindowTextW. On failure "unknown"; an empty title stays "".
        SetLastError(WIN32_ERROR(0));
        let mut text_buf = [0u16; 1024];
        let len = GetWindowTextW(hwnd, &mut text_buf);
        let title = if len > 0 {
            String::from_utf16_lossy(&text_buf[..len as usize])
        } else {
            let err = GetLastError();
            if err == ERROR_SUCCESS {
                String::new()
            } else {
                "unknown".to_string()
            }
        };

        Some(WindowSample { app, title })
    }
}

#[cfg(windows)]
fn get_process_app_name(pid: u32) -> String {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };

    if pid == 0 {
        return "unknown".to_string();
    }

    unsafe {
        let handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(h) if !h.0.is_null() => h,
            _ => return "unknown".to_string(),
        };

        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let res = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);

        if res.is_ok() && len > 0 {
            let full_path = String::from_utf16_lossy(&buf[..len as usize]);
            let file_name = extract_file_name(&full_path);
            if file_name.is_empty() {
                "unknown".to_string()
            } else {
                file_name.to_string()
            }
        } else {
            "unknown".to_string()
        }
    }
}

#[cfg(windows)]
unsafe extern "system" fn enum_child_proc(
    child_hwnd: windows::Win32::Foundation::HWND,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::core::BOOL {
    let children = &mut *(lparam.0 as *mut Vec<windows::Win32::Foundation::HWND>);
    children.push(child_hwnd);
    windows::core::BOOL(1)
}

#[cfg(windows)]
fn get_uwp_child_app(hwnd: windows::Win32::Foundation::HWND, host_pid: u32) -> Option<String> {
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{EnumChildWindows, GetWindowThreadProcessId};

    let mut children: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumChildWindows(
            Some(hwnd),
            Some(enum_child_proc),
            LPARAM(&mut children as *mut _ as isize),
        );
    }

    let child_pids: Vec<u32> = children
        .into_iter()
        .map(|child_hwnd| {
            let mut pid = 0u32;
            unsafe {
                GetWindowThreadProcessId(child_hwnd, Some(&mut pid));
            }
            pid
        })
        .collect();

    find_uwp_child_app(host_pid, &child_pids, get_process_app_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_file_name() {
        assert_eq!(
            extract_file_name(r"C:\Program Files\Mozilla Firefox\firefox.exe"),
            "firefox.exe"
        );
        assert_eq!(
            extract_file_name(r"C:\Windows\System32\ApplicationFrameHost.exe"),
            "ApplicationFrameHost.exe"
        );
        assert_eq!(
            extract_file_name("C:/Program Files/Google/Chrome/Application/chrome.exe"),
            "chrome.exe"
        );
        assert_eq!(extract_file_name("code.exe"), "code.exe");
        assert_eq!(extract_file_name(""), "");
        assert_eq!(extract_file_name("   "), "");
        assert_eq!(extract_file_name(r"C:\foo\bar\"), "");
    }

    #[test]
    fn test_find_uwp_child_app_choice() {
        let host_pid = 1000;

        // No child windows
        assert_eq!(find_uwp_child_app(host_pid, &[], |_| "app.exe".into()), None);

        // Children only match host_pid
        assert_eq!(
            find_uwp_child_app(host_pid, &[1000, 1000], |_| "ApplicationFrameHost.exe".into()),
            None
        );

        // Child PID is 0
        assert_eq!(
            find_uwp_child_app(host_pid, &[0], |_| "something.exe".into()),
            None
        );

        // Child resolves to "unknown" or empty or ApplicationFrameHost.exe
        assert_eq!(
            find_uwp_child_app(host_pid, &[2000, 3000], |pid| {
                if pid == 2000 {
                    "unknown".into()
                } else {
                    "ApplicationFrameHost.exe".into()
                }
            }),
            None
        );

        // First valid child app chosen
        assert_eq!(
            find_uwp_child_app(host_pid, &[1000, 2000, 3000], |pid| {
                match pid {
                    1000 => "ApplicationFrameHost.exe".into(),
                    2000 => "unknown".into(),
                    3000 => "CalculatorApp.exe".into(),
                    _ => "other.exe".into(),
                }
            }),
            Some("CalculatorApp.exe".into())
        );
    }

    #[test]
    fn test_window_sample_serialization() {
        let sample = WindowSample {
            app: "firefox.exe".into(),
            title: "GitHub - Mozilla Firefox".into(),
        };
        let json = serde_json::to_string(&sample).unwrap();
        assert!(json.contains("\"app\":\"firefox.exe\""));
        assert!(json.contains("\"title\":\"GitHub - Mozilla Firefox\""));

        let deserialized: WindowSample = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, sample);
    }

    #[test]
    fn test_watcher_poll_instantiation() {
        let mut watcher = WindowWatcher::new();
        // Running poll() on Windows will query the actual foreground window or return None
        let _ = watcher.poll();
    }
}
