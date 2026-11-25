use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tempfile::tempdir;
use ty_server::{run as run_server, ServerConfig};
use ty_watcher::{
    afk_bucket_id, compute_today_range, enable_autostart_with_marker, format_active_time,
    sanitize_hostname, spawn_watcher_loop, window_bucket_id, WatcherConfig,
};

fn get_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

#[test]
fn test_hostname_sanitization_and_bucket_naming() {
    assert_eq!(sanitize_hostname("laptop 123"), "laptop-123");
    assert_eq!(sanitize_hostname("  workstation   pc  "), "workstation-pc");
    assert_eq!(sanitize_hostname(""), "localhost");
    assert_eq!(sanitize_hostname("   "), "localhost");
    assert_eq!(sanitize_hostname("myhost"), "myhost");

    assert_eq!(
        window_bucket_id("laptop-123"),
        "aw-watcher-window_laptop-123"
    );
    assert_eq!(afk_bucket_id("laptop-123"), "aw-watcher-afk_laptop-123");
}

#[test]
fn test_autostart_marker_file() {
    let temp_dir = tempdir().expect("temp dir");
    let marker = temp_dir.path().join(".autostart_initialized");

    assert!(!marker.exists());
    let enabled1 = enable_autostart_with_marker(&marker).expect("enable 1");
    assert!(enabled1);
    assert!(marker.exists());

    let enabled2 = enable_autostart_with_marker(&marker).expect("enable 2");
    assert!(!enabled2);
}

#[test]
fn test_active_time_formatting() {
    assert_eq!(format_active_time(0.0), "Today: 0m");
    assert_eq!(format_active_time(30.0), "Today: 0m");
    assert_eq!(format_active_time(60.0), "Today: 1m");
    assert_eq!(format_active_time(3540.0), "Today: 59m");
    assert_eq!(format_active_time(3600.0), "Today: 1h 0m");
    assert_eq!(format_active_time(13320.0), "Today: 3h 42m");
}

#[test]
fn test_compute_today_range() {
    let range = compute_today_range("00:00");
    let duration = range.end - range.start;
    assert_eq!(duration.num_hours(), 24);
}

#[test]
fn test_watcher_lifecycle_and_flush_on_stop() {
    let port = get_free_port();
    let temp = tempdir().expect("temp dir");
    let data_dir = temp.path().join("server_data");
    let queue_dir = temp.path().join("client_queue");

    // 1. Start server
    let server_handle = run_server(ServerConfig {
        port,
        data_dir,
        extension_origins: Vec::new(),
    });

    let base_url = format!("http://127.0.0.1:{}", port);
    let host = "test-host".to_string();
    let update_count = Arc::new(AtomicUsize::new(0));
    let update_clone = Arc::clone(&update_count);

    // 2. Spawn watcher with fast intervals
    let config = WatcherConfig {
        base_url: base_url.clone(),
        queue_dir,
        hostname: host.clone(),
        afk_timeout_secs: 180,
        day_start: "00:00".to_string(),
        poll_window_interval: Duration::from_millis(50),
        poll_afk_interval: Duration::from_millis(100),
        settings_interval: Duration::from_millis(200),
        active_time_interval: Duration::from_millis(300),
    };

    let mut watcher = spawn_watcher_loop(config, move |_text| {
        update_clone.fetch_add(1, Ordering::SeqCst);
    });

    // Let it run to send heartbeats and query active time
    std::thread::sleep(Duration::from_millis(400));

    // 3. Stop watcher -> triggers client.shutdown() which flushes queue!
    watcher.stop();

    // 4. Verify heartbeats reached the server
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(2))
        .timeout_read(Duration::from_secs(2))
        .build();

    let afk_url = format!("{}/api/0/buckets/aw-watcher-afk_{}/events", base_url, host);
    let afk_resp = agent.get(&afk_url).call().expect("get afk events");
    assert_eq!(afk_resp.status(), 200);

    let afk_events: Vec<ty_types::Event> = afk_resp.into_json().expect("parse afk events");
    assert!(!afk_events.is_empty(), "AFK events should be flushed to server on stop");

    // 5. Clean server shutdown
    server_handle.shutdown();
}
