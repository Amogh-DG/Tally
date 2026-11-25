use std::fs;
use chrono::{Duration, Local, NaiveDate, TimeZone, Utc};
use serde_json::json;
use tempfile::tempdir;

use tally_ui::client::TallyHttpClient;
use tally_ui::dates::{parse_day_range, parse_range};
use tally_ui::export::{write_export_file, ExportData};
use ty_server::{run, ServerConfig};
use ty_settings::Settings;

#[tokio::test]
async fn test_end_to_end_real_server_analytics_and_export() {
    let temp = tempdir().expect("temp dir");
    let data_dir = temp.path().join("server_data");

    // 1. Start the real ty-server (backed by SqliteStore in data_dir) on an ephemeral port
    let server_handle = run(ServerConfig {
        port: 0,
        data_dir: data_dir.clone(),
        extension_origins: Vec::new(),
    });
    let port = server_handle.port();
    let base_url = format!("http://127.0.0.1:{}", port);

    // Confirm SqliteStore DB file was created on disk
    assert!(
        data_dir.join("tally.db").exists(),
        "real server should create tally.db in data_dir"
    );

    // 2. Register watcher-style buckets and post heartbeats (matching ty-watcher shapes)
    let http = reqwest::Client::new();
    let host = "e2e-host";
    let window_bucket = format!("aw-watcher-window_{}", host);
    let afk_bucket = format!("aw-watcher-afk_{}", host);

    let res = http
        .post(format!("{}/api/0/buckets/{}", base_url, window_bucket))
        .json(&json!({
            "client": "aw-watcher-window",
            "type": "currentwindow",
            "hostname": host,
            "data": {}
        }))
        .send()
        .await
        .expect("create window bucket");
    assert_eq!(res.status(), 200);

    let res = http
        .post(format!("{}/api/0/buckets/{}", base_url, afk_bucket))
        .json(&json!({
            "client": "aw-watcher-afk",
            "type": "afkstatus",
            "hostname": host,
            "data": {}
        }))
        .send()
        .await
        .expect("create afk bucket");
    assert_eq!(res.status(), 200);

    // Use noon local today so all events fall cleanly inside today's local day range
    let today_local: NaiveDate = Local::now().date_naive();
    let noon_local = today_local.and_hms_opt(12, 0, 0).unwrap();
    let t0 = Local
        .from_local_datetime(&noon_local)
        .earliest()
        .unwrap()
        .with_timezone(&Utc);

    // Send AFK heartbeats (not-afk spanning 12:00:00 -> 12:05:00 = 300s)
    let res = http
        .post(format!(
            "{}/api/0/buckets/{}/heartbeat?pulsetime=7.5",
            base_url, afk_bucket
        ))
        .json(&json!({
            "timestamp": t0,
            "duration": 0.0,
            "data": { "status": "not-afk" }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = http
        .post(format!(
            "{}/api/0/buckets/{}/heartbeat?pulsetime=310.0",
            base_url, afk_bucket
        ))
        .json(&json!({
            "timestamp": t0 + Duration::seconds(300),
            "duration": 0.0,
            "data": { "status": "not-afk" }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // Window events: code.exe for 180s, firefox.exe for 120s (title-only browser)
    let res = http
        .post(format!(
            "{}/api/0/buckets/{}/heartbeat?pulsetime=2.0",
            base_url, window_bucket
        ))
        .json(&json!({
            "timestamp": t0,
            "duration": 180.0,
            "data": { "app": "code.exe", "title": "lib.rs - Tally" }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = http
        .post(format!(
            "{}/api/0/buckets/{}/heartbeat?pulsetime=2.0",
            base_url, window_bucket
        ))
        .json(&json!({
            "timestamp": t0 + Duration::seconds(180),
            "duration": 120.0,
            "data": { "app": "firefox.exe", "title": "GitHub - Mozilla Firefox" }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // 3. Exercise TallyHttpClient (same client used by Tauri commands, including ?limit=-1)
    let tauri_client = TallyHttpClient::from_port(port);
    assert!(tauri_client.check_server_status().await);

    let today_str = today_local.format("%Y-%m-%d").to_string();
    let day_range = parse_day_range(&today_str, "00:00").expect("day range");
    let events = tauri_client
        .fetch_all_events(Some(day_range.start), Some(day_range.end))
        .await
        .expect("fetch_all_events against real ty-server");

    assert_eq!(events.window.len(), 2);
    assert_eq!(events.afk.len(), 1);
    assert_eq!(events.web.len(), 0);

    // 4. Exercise ty-analytics builders
    let day_report = ty_analytics::build_day_report(
        &events.window,
        &events.afk,
        &events.web,
        day_range.clone(),
        "00:00",
    );
    assert_eq!(day_report.date, today_str);
    assert!((day_report.active_secs - 300.0).abs() < 0.01);
    assert_eq!(day_report.top_apps.len(), 2);
    assert_eq!(day_report.top_apps[0].app, "code.exe");
    assert_eq!(day_report.top_sites.len(), 1);
    assert_eq!(day_report.top_sites[0].domain, "firefox.exe");

    let range = parse_range(&today_str, &today_str, "00:00").expect("range");
    let range_hours = ty_analytics::build_range_hours(&events.window, &events.afk, range.clone(), "00:00");
    assert_eq!(range_hours.len(), 1);
    assert!((range_hours[0].active_secs - 300.0).abs() < 0.01);

    let site_list = ty_analytics::build_site_list(&events.window, &events.afk, &events.web, range.clone());
    assert_eq!(site_list.len(), 1);
    assert_eq!(site_list[0].domain, "firefox.exe");
    assert_eq!(site_list[0].pages[0].title, "GitHub - Mozilla Firefox");

    let index = ty_analytics::build_search_index(&site_list);
    let hits = ty_analytics::search(&index, "GitHub", "titles", None);
    assert!(!hits.is_empty());
    assert_eq!(hits[0].domain, "firefox.exe");

    // 5. Exercise JSON export end-to-end
    let export_path = temp.path().join("exports").join("tidemark_export.json");
    let export_path_str = export_path.to_string_lossy().to_string();
    let settings = Settings {
        port,
        data_dir: data_dir.clone(),
        ..Settings::default()
    };
    let written = write_export_file(&export_path_str, range, events, &settings)
        .expect("export JSON should succeed");
    assert_eq!(written, export_path_str);

    let export_raw = fs::read_to_string(&export_path).expect("read exported JSON");
    let parsed: ExportData = serde_json::from_str(&export_raw).expect("parse exported JSON");
    assert_eq!(parsed.events.window.len(), 2);
    assert!(parsed.day_report.is_some());
    assert!((parsed.day_report.unwrap().active_secs - 300.0).abs() < 0.01);
    assert!(parsed.range_hours.is_some());
    assert!(parsed.site_list.is_some());

    server_handle.shutdown();
}
