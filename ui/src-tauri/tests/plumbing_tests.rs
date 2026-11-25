use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use chrono::{Duration, Utc};
use serde_json::json;
use tempfile::NamedTempFile;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use tally_ui::client::{ClassifiedEvents, TallyHttpClient};
use tally_ui::commands::{get_day, get_range_hours, get_settings, set_settings};
use tally_ui::dates::{parse_day_range, parse_day_start, parse_range};
use tally_ui::export::{write_export_file, ExportData};
use ty_analytics::{DayReport, Hit, RangeHours, SiteList, TimeRange};
use ty_settings::Settings;
use ty_types::Event;

/// Spawns a mock HTTP server returning configured JSON responses.
/// Returns the base URL (e.g. `http://127.0.0.1:12345`) and request counter.
async fn spawn_mock_server() -> (String, u16, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let base_url = format!("http://{}", addr);
    let port = addr.port();
    let req_count = Arc::new(AtomicUsize::new(0));
    let req_count_clone = Arc::clone(&req_count);

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(conn) => conn,
                Err(_) => break,
            };

            let req_counter = Arc::clone(&req_count_clone);
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let n = match socket.read(&mut buf).await {
                    Ok(n) if n > 0 => n,
                    _ => return,
                };
                req_counter.fetch_add(1, Ordering::SeqCst);

                let req_text = String::from_utf8_lossy(&buf[..n]);
                let first_line = req_text.lines().next().unwrap_or_default();
                let parts: Vec<&str> = first_line.split_whitespace().collect();
                if parts.len() < 2 {
                    return;
                }

                let uri = parts[1];
                let (path, _query) = match uri.split_once('?') {
                    Some((p, q)) => (p, Some(q)),
                    None => (uri, None),
                };

                let (status, body) = if path == "/api/0/info" {
                    (
                        "200 OK",
                        json!({
                            "hostname": "test-host",
                            "version": "0.1.0",
                            "testing": true
                        })
                        .to_string(),
                    )
                } else if path == "/api/0/buckets/" {
                    let now = Utc::now();
                    let buckets = json!({
                        "aw-watcher-window_test": {
                            "id": "aw-watcher-window_test",
                            "type": "currentwindow",
                            "client": "aw-watcher-window",
                            "hostname": "test-host",
                            "created": now.to_rfc3339(),
                            "data": {}
                        },
                        "aw-watcher-afk_test": {
                            "id": "aw-watcher-afk_test",
                            "type": "afkstatus",
                            "client": "aw-watcher-afk",
                            "hostname": "test-host",
                            "created": now.to_rfc3339(),
                            "data": {}
                        },
                        "aw-watcher-web-chrome_test": {
                            "id": "aw-watcher-web-chrome_test",
                            "type": "web.tab.current",
                            "client": "aw-watcher-web-chrome",
                            "hostname": "test-host",
                            "created": now.to_rfc3339(),
                            "data": {}
                        },
                        "unrelated-bucket": {
                            "id": "unrelated-bucket",
                            "type": "other",
                            "client": "other",
                            "hostname": "test-host",
                            "created": now.to_rfc3339(),
                            "data": {}
                        }
                    });
                    ("200 OK", buckets.to_string())
                } else if path.contains("/events") && !path.contains("error") {
                    let now = Utc::now();
                    let events = if path.contains("window") {
                        json!([
                            {
                                "id": 1,
                                "timestamp": (now - Duration::minutes(10)).to_rfc3339(),
                                "duration": 300.0,
                                "data": { "app": "code.exe", "title": "main.rs" }
                            }
                        ])
                    } else if path.contains("afk") {
                        json!([
                            {
                                "id": 2,
                                "timestamp": (now - Duration::minutes(10)).to_rfc3339(),
                                "duration": 300.0,
                                "data": { "status": "not-afk" }
                            }
                        ])
                    } else if path.contains("web") {
                        json!([
                            {
                                "id": 3,
                                "timestamp": (now - Duration::minutes(10)).to_rfc3339(),
                                "duration": 300.0,
                                "data": { "url": "https://docs.rs", "title": "docs.rs" }
                            }
                        ])
                    } else {
                        json!([])
                    };
                    ("200 OK", events.to_string())
                } else if path == "/api/0/error-500" {
                    ("500 Internal Server Error", "Internal error occurred".to_string())
                } else {
                    ("404 Not Found", "Not Found".to_string())
                };

                let response = format!(
                    "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status,
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            });
        }
    });

    (base_url, port, req_count)
}

#[tokio::test]
async fn test_mock_server_status() {
    let (base_url, _, _) = spawn_mock_server().await;
    let client = TallyHttpClient::new(base_url);

    // When mock server is running, check_server_status returns true
    assert!(client.check_server_status().await);

    // When pointing to unreachable port, check_server_status returns false
    let bad_client = TallyHttpClient::new("http://127.0.0.1:59999".to_string());
    assert!(!bad_client.check_server_status().await);
}

#[tokio::test]
async fn test_bucket_discovery_and_classification() {
    let (base_url, _, _) = spawn_mock_server().await;
    let client = TallyHttpClient::new(base_url);

    let buckets = client.fetch_buckets().await.expect("fetch buckets");
    assert_eq!(buckets.len(), 4);

    let categories = TallyHttpClient::classify_buckets(&buckets);
    assert_eq!(categories.window_bucket_ids, vec!["aw-watcher-window_test"]);
    assert_eq!(categories.afk_bucket_ids, vec!["aw-watcher-afk_test"]);
    assert_eq!(categories.web_bucket_ids, vec!["aw-watcher-web-chrome_test"]);
}

#[tokio::test]
async fn test_fetch_events_and_classify() {
    let (base_url, _, _) = spawn_mock_server().await;
    let client = TallyHttpClient::new(base_url);

    let now = Utc::now();
    let start = Some(now - Duration::hours(1));
    let end = Some(now);

    let classified = client
        .fetch_all_events(start, end)
        .await
        .expect("fetch all events");

    assert_eq!(classified.window.len(), 1);
    assert_eq!(classified.afk.len(), 1);
    assert_eq!(classified.web.len(), 1);

    assert_eq!(
        classified.window[0].data["app"].as_str().unwrap(),
        "code.exe"
    );
    assert_eq!(
        classified.afk[0].data["status"].as_str().unwrap(),
        "not-afk"
    );
    assert_eq!(
        classified.web[0].data["url"].as_str().unwrap(),
        "https://docs.rs"
    );
}

#[tokio::test]
async fn test_http_error_returns_plain_string() {
    let (base_url, _, _) = spawn_mock_server().await;
    let bad_client = TallyHttpClient::new(format!("{}/api/0/error-500", base_url));

    let res = bad_client.fetch_buckets().await;
    assert!(res.is_err());
    let err_msg = res.unwrap_err();
    assert!(
        err_msg.contains("500") || err_msg.contains("error status"),
        "Error should describe HTTP failure: {}",
        err_msg
    );
}

#[test]
fn test_dates_plumbing() {
    assert_eq!(parse_day_start("04:00"), (4, 0));
    assert_eq!(parse_day_start("00:00"), (0, 0));
    assert_eq!(parse_day_start("invalid"), (0, 0));

    // Day range for YYYY-MM-DD
    let day_range = parse_day_range("2026-11-23", "00:00").expect("parse date");
    assert!(day_range.start < day_range.end);
    let diff = day_range.end - day_range.start;
    assert!(diff.num_hours() >= 23 && diff.num_hours() <= 25);

    // Day range from RFC 3339
    let rfc_range = parse_day_range("2026-11-23T10:00:00Z", "04:00").expect("parse rfc3339");
    assert!(rfc_range.start < rfc_range.end);

    // Arbitrary range
    let range = parse_range("2026-11-01", "2026-11-07", "00:00").expect("parse range");
    assert!(range.start < range.end);
    assert!(range.end - range.start >= Duration::days(6));

    // Inverted range returns error string
    let bad_range = parse_range("2026-11-10", "2026-11-01", "00:00");
    assert!(bad_range.is_err());
    assert!(bad_range.unwrap_err().contains("after 'to'"));

    // Invalid format returns error string
    let malformed = parse_range("bad-from", "2026-11-01", "00:00");
    assert!(malformed.is_err());
}

#[test]
fn test_settings_plumbing() {
    let settings = Settings::default();
    assert_eq!(settings.port, 5600);
    assert_eq!(settings.afk_timeout_secs, 180);
    assert_eq!(settings.day_start, "00:00");

    let saved = set_settings(settings.clone()).expect("set_settings should succeed");
    assert_eq!(saved, settings);

    let loaded = get_settings().expect("get_settings should succeed");
    assert_eq!(loaded.port, 5600);
}

#[tokio::test]
async fn test_commands_error_handling() {
    // Malformed date returns a plain string error
    let invalid_date = get_day("invalid-date-format".to_string()).await;
    assert!(invalid_date.is_err());
    let err_str = invalid_date.unwrap_err();
    assert!(err_str.contains("Invalid date"));

    // Inverted range returns a plain string error
    let invalid_range = get_range_hours("2026-11-25".to_string(), "2026-11-20".to_string()).await;
    assert!(invalid_range.is_err());
    let range_err = invalid_range.unwrap_err();
    assert!(range_err.contains("after 'to'"));

    // Unreachable client returns a plain string error
    let unreachable_client = TallyHttpClient::new("http://127.0.0.1:59999".to_string());
    let res = unreachable_client.fetch_buckets().await;
    assert!(res.is_err());
    let err_msg = res.unwrap_err();
    assert!(err_msg.contains("failed"), "Error: {}", err_msg);
}

#[test]
fn test_export_json_plumbing() {
    let file = NamedTempFile::new().expect("temp file");
    let target_path = file.path().to_string_lossy().to_string();

    let now = Utc::now();
    let range = TimeRange {
        start: now - Duration::hours(2),
        end: now,
    };

    let mut events = ClassifiedEvents::default();
    events.window.push(Event::new(
        now - Duration::hours(1),
        3600.0,
        json!({"app": "firefox.exe", "title": "Rust documentation"}),
    ));

    let settings = Settings::default();
    let written_path = write_export_file(&target_path, range.clone(), events, &settings)
        .expect("write export");
    assert_eq!(written_path, target_path);

    let contents = fs::read_to_string(&target_path).expect("read export file");
    let export_data: ExportData = serde_json::from_str(&contents).expect("deserialize export");
    assert_eq!(export_data.range, range);
    assert_eq!(export_data.events.window.len(), 1);
    assert_eq!(
        export_data.events.window[0].data["app"],
        json!("firefox.exe")
    );
}

#[test]
fn test_fixtures_deserialization() {
    // Tests that all fixtures from docs/fixtures/ parse properly into their respective contract types
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("docs")
        .join("fixtures");

    // day_report.json
    let day_report_path = fixtures_dir.join("day_report.json");
    if day_report_path.exists() {
        let content = fs::read_to_string(&day_report_path).expect("read day_report");
        let parsed: DayReport = serde_json::from_str(&content).expect("parse day_report");
        assert_eq!(parsed.date, "2026-11-23");
        assert_eq!(parsed.active_secs, 28800.0);
        assert_eq!(parsed.timeline.len(), 3);
        assert_eq!(parsed.top_apps.len(), 2);
        assert_eq!(parsed.top_sites.len(), 2);
    }

    // range_hours.json
    let range_hours_path = fixtures_dir.join("range_hours.json");
    if range_hours_path.exists() {
        let content = fs::read_to_string(&range_hours_path).expect("read range_hours");
        let parsed: RangeHours = serde_json::from_str(&content).expect("parse range_hours");
        assert_eq!(parsed.len(), 7);
        assert_eq!(parsed[0].date, "2026-11-17");
    }

    // site_list.json
    let site_list_path = fixtures_dir.join("site_list.json");
    if site_list_path.exists() {
        let content = fs::read_to_string(&site_list_path).expect("read site_list");
        let parsed: SiteList = serde_json::from_str(&content).expect("parse site_list");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].domain, "github.com");
    }

    // search_hits.json
    let search_hits_path = fixtures_dir.join("search_hits.json");
    if search_hits_path.exists() {
        let content = fs::read_to_string(&search_hits_path).expect("read search_hits");
        let parsed: Vec<Hit> = serde_json::from_str(&content).expect("parse search_hits");
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].domain, "github.com");
    }

    // settings.json
    let settings_path = fixtures_dir.join("settings.json");
    if settings_path.exists() {
        let content = fs::read_to_string(&settings_path).expect("read settings");
        let parsed: Settings = serde_json::from_str(&content).expect("parse settings");
        assert_eq!(parsed.port, 5600);
        assert_eq!(parsed.afk_timeout_secs, 180);
    }
}
