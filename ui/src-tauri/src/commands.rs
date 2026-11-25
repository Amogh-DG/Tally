use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use ty_analytics::{DayReport, Hit, RangeHours, SiteList, TimeRange};
use ty_settings::Settings;

use crate::client::TallyHttpClient;
use crate::dates::{parse_day_range, parse_range};
use crate::export::write_export_file;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerStatus {
    pub reachable: bool,
}

#[tauri::command]
pub async fn get_day(date: String) -> Result<DayReport, String> {
    let settings = ty_settings::load().unwrap_or_default();
    let range = parse_day_range(&date, &settings.day_start)?;
    let client = TallyHttpClient::from_port(settings.port);
    let events = client
        .fetch_all_events(Some(range.start), Some(range.end))
        .await?;

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ty_analytics::build_day_report(
            &events.window,
            &events.afk,
            &events.web,
            range,
            &settings.day_start,
        )
    })) {
        Ok(report) => Ok(report),
        Err(_) => Err("ty_analytics::build_day_report is not yet implemented".to_string()),
    }
}

#[tauri::command]
pub async fn get_range_hours(from: String, to: String) -> Result<RangeHours, String> {
    let settings = ty_settings::load().unwrap_or_default();
    let range = parse_range(&from, &to, &settings.day_start)?;
    let client = TallyHttpClient::from_port(settings.port);
    let events = client
        .fetch_all_events(Some(range.start), Some(range.end))
        .await?;

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ty_analytics::build_range_hours(
            &events.window,
            &events.afk,
            range,
            &settings.day_start,
        )
    })) {
        Ok(hours) => Ok(hours),
        Err(_) => Err("ty_analytics::build_range_hours is not yet implemented".to_string()),
    }
}

#[tauri::command]
pub async fn get_sites(from: String, to: String) -> Result<SiteList, String> {
    let settings = ty_settings::load().unwrap_or_default();
    let range = parse_range(&from, &to, &settings.day_start)?;
    let client = TallyHttpClient::from_port(settings.port);
    let events = client
        .fetch_all_events(Some(range.start), Some(range.end))
        .await?;

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ty_analytics::build_site_list(
            &events.window,
            &events.afk,
            &events.web,
            range,
        )
    })) {
        Ok(sites) => Ok(sites),
        Err(_) => Err("ty_analytics::build_site_list is not yet implemented".to_string()),
    }
}

#[tauri::command]
pub async fn search(
    query: String,
    mode: String,
    site: Option<String>,
    from: Option<String>,
    to: Option<String>,
) -> Result<Vec<Hit>, String> {
    let settings = ty_settings::load().unwrap_or_default();
    let range = match (from.as_deref(), to.as_deref()) {
        (Some(f), Some(t)) => parse_range(f, t, &settings.day_start)?,
        (Some(f), None) => parse_range(f, &Utc::now().to_rfc3339(), &settings.day_start)?,
        (None, Some(t)) => parse_range("1970-01-01T00:00:00Z", t, &settings.day_start)?,
        (None, None) => TimeRange {
            start: DateTime::<Utc>::UNIX_EPOCH,
            end: Utc::now() + Duration::days(1),
        },
    };

    let client = TallyHttpClient::from_port(settings.port);
    let events = client
        .fetch_all_events(Some(range.start), Some(range.end))
        .await?;

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let site_list = ty_analytics::build_site_list(
            &events.window,
            &events.afk,
            &events.web,
            range,
        );
        let index = ty_analytics::build_search_index(&site_list);
        ty_analytics::search(&index, &query, &mode, site.as_deref())
    })) {
        Ok(hits) => Ok(hits),
        Err(_) => Err("ty_analytics::search is not yet implemented".to_string()),
    }
}

#[tauri::command]
pub async fn export_json(from: String, to: String, path: String) -> Result<String, String> {
    let settings = ty_settings::load().unwrap_or_default();
    let range = parse_range(&from, &to, &settings.day_start)?;
    let client = TallyHttpClient::from_port(settings.port);
    let events = client
        .fetch_all_events(Some(range.start), Some(range.end))
        .await?;

    write_export_file(&path, range, events, &settings)
}

#[tauri::command]
pub fn get_settings() -> Result<Settings, String> {
    ty_settings::load().map_err(|e| format!("Failed to load settings: {}", e))
}

#[tauri::command]
pub fn set_settings(settings: Settings) -> Result<Settings, String> {
    ty_settings::save(&settings)
        .map(|_| settings)
        .map_err(|e| format!("Failed to save settings: {}", e))
}

#[tauri::command]
pub async fn server_status() -> Result<ServerStatus, String> {
    let port = ty_settings::load().map(|s| s.port).unwrap_or(5600);
    let client = TallyHttpClient::from_port(port);
    let reachable = client.check_server_status().await;
    Ok(ServerStatus { reachable })
}
