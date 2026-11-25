pub mod client;
pub mod commands;
pub mod dates;
pub mod export;

pub use commands::*;

pub fn run() {
    tauri::Builder::default()
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed = event {
                use tauri::Manager;
                window.app_handle().exit(0);
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_day,
            get_range_hours,
            get_sites,
            search,
            export_json,
            get_settings,
            set_settings,
            server_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
