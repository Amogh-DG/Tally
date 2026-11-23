#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use ty_analytics::{DayReport, Hit, RangeHours, SiteList};
use ty_settings::Settings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    pub reachable: bool,
}

#[tauri::command]
fn get_day(_date: String) -> Result<DayReport, String> {
    todo!("get_day command not yet implemented")
}

#[tauri::command]
fn get_range_hours(_from: String, _to: String) -> Result<RangeHours, String> {
    todo!("get_range_hours command not yet implemented")
}

#[tauri::command]
fn get_sites(_from: String, _to: String) -> Result<SiteList, String> {
    todo!("get_sites command not yet implemented")
}

#[tauri::command]
fn search(
    _query: String,
    _mode: String,
    _site: Option<String>,
    _from: Option<String>,
    _to: Option<String>,
) -> Result<Vec<Hit>, String> {
    todo!("search command not yet implemented")
}

#[tauri::command]
fn export_json(_from: String, _to: String, _path: String) -> Result<String, String> {
    todo!("export_json command not yet implemented")
}

#[tauri::command]
fn get_settings() -> Result<Settings, String> {
    todo!("get_settings command not yet implemented")
}

#[tauri::command]
fn set_settings(_settings: Settings) -> Result<Settings, String> {
    todo!("set_settings command not yet implemented")
}

#[tauri::command]
fn server_status() -> Result<ServerStatus, String> {
    todo!("server_status command not yet implemented")
}

fn main() {
    tauri::Builder::default()
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
