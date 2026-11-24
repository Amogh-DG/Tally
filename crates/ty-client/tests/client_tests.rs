mod common;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use chrono::{Duration as ChronoDuration, Utc};
use serde_json::json;
use ty_client::{BucketSpec, Client, ClientConfig};
use ty_types::Event;

use common::mock_server::{MockResponse, MockServer};

fn unique_temp_dir(name: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir()
        .join(format!("tally_client_test_{}_{}_{}", name, std::process::id(), id));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn test_pre_merge() {
    let server = MockServer::start();
    let queue_dir = unique_temp_dir("pre_merge");

    let config = ClientConfig {
        base_url: server.base_url(),
        queue_dir: queue_dir.clone(),
        commit_interval_secs: 60,
    };
    let client = Client::start_with_retry_interval(config, Duration::from_millis(50));

    let spec = BucketSpec {
        id: "test-window".to_string(),
        bucket_type: "currentwindow".to_string(),
        client: "test".to_string(),
        hostname: "localhost".to_string(),
    };
    client.register_bucket(spec);

    let t0 = Utc::now();
    // Send 3 heartbeats 1 second apart with identical data
    client.heartbeat_at("test-window", json!({"app": "code.exe"}), 2.0, t0);
    client.heartbeat_at(
        "test-window",
        json!({"app": "code.exe"}),
        2.0,
        t0 + ChronoDuration::seconds(1),
    );
    client.heartbeat_at(
        "test-window",
        json!({"app": "code.exe"}),
        2.0,
        t0 + ChronoDuration::seconds(2),
    );

    thread::sleep(Duration::from_millis(150));

    // Verify no heartbeats sent to the server yet (all merged in memory)
    let hb_reqs: Vec<_> = server
        .requests()
        .into_iter()
        .filter(|r| r.path.contains("/heartbeat"))
        .collect();
    assert_eq!(
        hb_reqs.len(),
        0,
        "Heartbeats should be pre-merged in memory and not yet sent"
    );

    // Shutdown flushes the merged event
    client.shutdown();

    let final_hb_reqs: Vec<_> = server
        .requests()
        .into_iter()
        .filter(|r| r.path.contains("/heartbeat"))
        .collect();
    assert_eq!(
        final_hb_reqs.len(),
        1,
        "Exactly 1 merged heartbeat should be sent"
    );

    let event: Event = final_hb_reqs[0].json().expect("Valid Event json");
    assert_eq!(event.data, json!({"app": "code.exe"}));
    assert_eq!(event.duration, 2.0);

    let _ = fs::remove_dir_all(&queue_dir);
}

#[test]
fn test_commit_on_change() {
    let server = MockServer::start();
    let queue_dir = unique_temp_dir("commit_on_change");

    let config = ClientConfig {
        base_url: server.base_url(),
        queue_dir: queue_dir.clone(),
        commit_interval_secs: 60,
    };
    let client = Client::start_with_retry_interval(config, Duration::from_millis(50));

    let spec = BucketSpec {
        id: "test-window".to_string(),
        bucket_type: "currentwindow".to_string(),
        client: "test".to_string(),
        hostname: "localhost".to_string(),
    };
    client.register_bucket(spec);

    let t0 = Utc::now();
    client.heartbeat_at("test-window", json!({"app": "code.exe"}), 2.0, t0);
    client.heartbeat_at(
        "test-window",
        json!({"app": "code.exe"}),
        2.0,
        t0 + ChronoDuration::seconds(1),
    );

    thread::sleep(Duration::from_millis(100));
    assert_eq!(
        server
            .requests()
            .into_iter()
            .filter(|r| r.path.contains("/heartbeat"))
            .count(),
        0
    );

    // Change data to firefox.exe: this commits the previous code.exe event!
    client.heartbeat_at(
        "test-window",
        json!({"app": "firefox.exe"}),
        2.0,
        t0 + ChronoDuration::seconds(2),
    );

    // Wait for worker to send the committed item
    thread::sleep(Duration::from_millis(250));

    let hb_reqs: Vec<_> = server
        .requests()
        .into_iter()
        .filter(|r| r.path.contains("/heartbeat"))
        .collect();
    assert_eq!(hb_reqs.len(), 1, "code.exe should be committed on change");

    let ev1: Event = hb_reqs[0].json().unwrap();
    assert_eq!(ev1.data, json!({"app": "code.exe"}));
    assert_eq!(ev1.duration, 1.0);

    // Now change data to terminal.exe: this commits firefox.exe!
    client.heartbeat_at(
        "test-window",
        json!({"app": "terminal.exe"}),
        2.0,
        t0 + ChronoDuration::seconds(3),
    );

    thread::sleep(Duration::from_millis(250));

    let hb_reqs2: Vec<_> = server
        .requests()
        .into_iter()
        .filter(|r| r.path.contains("/heartbeat"))
        .collect();
    assert_eq!(
        hb_reqs2.len(),
        2,
        "firefox.exe should be committed on change"
    );

    let ev2: Event = hb_reqs2[1].json().unwrap();
    assert_eq!(ev2.data, json!({"app": "firefox.exe"}));
    assert_eq!(ev2.duration, 0.0);

    client.shutdown();
    let _ = fs::remove_dir_all(&queue_dir);
}

#[test]
fn test_commit_on_interval() {
    let server = MockServer::start();
    let queue_dir = unique_temp_dir("commit_on_interval");

    let config = ClientConfig {
        base_url: server.base_url(),
        queue_dir: queue_dir.clone(),
        commit_interval_secs: 1, // 1 second commit interval
    };
    let client = Client::start_with_retry_interval(config, Duration::from_millis(50));

    let spec = BucketSpec {
        id: "test-window".to_string(),
        bucket_type: "currentwindow".to_string(),
        client: "test".to_string(),
        hostname: "localhost".to_string(),
    };
    client.register_bucket(spec);

    let t0 = Utc::now();
    client.heartbeat_at("test-window", json!({"app": "code.exe"}), 2.0, t0);
    client.heartbeat_at(
        "test-window",
        json!({"app": "code.exe"}),
        2.0,
        t0 + ChronoDuration::milliseconds(500),
    );

    thread::sleep(Duration::from_millis(100));
    assert_eq!(
        server
            .requests()
            .into_iter()
            .filter(|r| r.path.contains("/heartbeat"))
            .count(),
        0
    );

    // Wait for the 1-second commit interval to expire
    thread::sleep(Duration::from_millis(1300));

    let hb_reqs: Vec<_> = server
        .requests()
        .into_iter()
        .filter(|r| r.path.contains("/heartbeat"))
        .collect();
    assert_eq!(
        hb_reqs.len(),
        1,
        "Heartbeat should be committed when commit_interval passes"
    );

    let ev: Event = hb_reqs[0].json().unwrap();
    assert_eq!(ev.data, json!({"app": "code.exe"}));
    assert_eq!(ev.duration, 0.5);

    client.shutdown();
    let _ = fs::remove_dir_all(&queue_dir);
}

#[test]
fn test_queue_persistence_across_restart() {
    let queue_dir = unique_temp_dir("persistence");

    // 1. First run: server returns 500 so items cannot be delivered and stay in queue
    let server1 = MockServer::start_with_handler(Box::new(|req| {
        if req.path.contains("/heartbeat") {
            MockResponse::status(500, "Internal Server Error")
        } else {
            MockResponse::ok()
        }
    }));

    let config1 = ClientConfig {
        base_url: server1.base_url(),
        queue_dir: queue_dir.clone(),
        commit_interval_secs: 60,
    };
    let client1 = Client::start_with_retry_interval(config1, Duration::from_millis(50));

    let spec = BucketSpec {
        id: "persist-bucket".to_string(),
        bucket_type: "currentwindow".to_string(),
        client: "test".to_string(),
        hostname: "localhost".to_string(),
    };
    client1.register_bucket(spec);

    let t0 = Utc::now();
    client1.heartbeat_at("persist-bucket", json!({"app": "app1"}), 2.0, t0);
    // Change data to commit app1
    client1.heartbeat_at(
        "persist-bucket",
        json!({"app": "app2"}),
        2.0,
        t0 + ChronoDuration::seconds(1),
    );

    thread::sleep(Duration::from_millis(200));

    // Shutdown client 1
    client1.shutdown();
    drop(server1);

    // Verify items remain on disk in pending/
    let pending_files: Vec<_> = fs::read_dir(queue_dir.join("pending"))
        .unwrap()
        .flatten()
        .collect();
    assert!(
        !pending_files.is_empty(),
        "Queue items must be persisted on disk across client restart"
    );

    // 2. Second run: server returns 200 OK. Client 2 restarts with same queue_dir
    let server2 = MockServer::start();
    let config2 = ClientConfig {
        base_url: server2.base_url(),
        queue_dir: queue_dir.clone(),
        commit_interval_secs: 60,
    };
    let client2 = Client::start_with_retry_interval(config2, Duration::from_millis(50));

    // Wait for client 2's worker to send the persisted items
    thread::sleep(Duration::from_millis(400));
    client2.shutdown();

    let hb_reqs: Vec<_> = server2
        .requests()
        .into_iter()
        .filter(|r| r.path.contains("/heartbeat"))
        .collect();
    assert!(
        !hb_reqs.is_empty(),
        "Persisted items should be sent by client 2 on restart"
    );

    let ev: Event = hb_reqs[0].json().unwrap();
    assert_eq!(ev.data, json!({"app": "app1"}));

    // Verify pending directory is now empty
    let remaining_files: Vec<_> = fs::read_dir(queue_dir.join("pending"))
        .unwrap()
        .flatten()
        .collect();
    assert_eq!(
        remaining_files.len(),
        0,
        "Pending queue directory should be empty after successful flush"
    );

    let _ = fs::remove_dir_all(&queue_dir);
}

#[test]
fn test_retry_on_connection_error_408_429_5xx() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_clone = attempts.clone();

    let server = MockServer::start_with_handler(Box::new(move |req| {
        if !req.path.contains("/heartbeat") {
            return MockResponse::ok();
        }

        let count = attempts_clone.fetch_add(1, Ordering::SeqCst);
        match count {
            0 => MockResponse::drop_connection(), // Connection error / dropped socket
            1 => MockResponse::status(408, "Request Timeout"),
            2 => MockResponse::status(429, "Too Many Requests"),
            3 => MockResponse::status(500, "Internal Server Error"),
            4 => MockResponse::status(503, "Service Unavailable"),
            _ => MockResponse::ok(), // Finally 200 OK
        }
    }));

    let queue_dir = unique_temp_dir("retry_errors");
    let config = ClientConfig {
        base_url: server.base_url(),
        queue_dir: queue_dir.clone(),
        commit_interval_secs: 60,
    };
    // Fast retry interval for testing (40ms)
    let client = Client::start_with_retry_interval(config, Duration::from_millis(40));

    let spec = BucketSpec {
        id: "retry-bucket".to_string(),
        bucket_type: "currentwindow".to_string(),
        client: "test".to_string(),
        hostname: "localhost".to_string(),
    };
    client.register_bucket(spec);

    let t0 = Utc::now();
    client.heartbeat_at("retry-bucket", json!({"app": "resilient"}), 2.0, t0);
    // Data change to commit resilient
    client.heartbeat_at(
        "retry-bucket",
        json!({"app": "next"}),
        2.0,
        t0 + ChronoDuration::seconds(1),
    );

    // Wait for retries to progress through dropped connection, 408, 429, 500, 503, and finally 200
    let mut success = false;
    for _ in 0..60 {
        thread::sleep(Duration::from_millis(50));
        if attempts.load(Ordering::SeqCst) >= 6 {
            success = true;
            break;
        }
    }

    assert!(
        success,
        "Worker should retry through connection error, 408, 429, 500, 503 until 200 OK; attempts: {}",
        attempts.load(Ordering::SeqCst)
    );

    client.shutdown();

    // Verify item was not dropped and pending is now clean
    let remaining: Vec<_> = fs::read_dir(queue_dir.join("pending"))
        .unwrap()
        .flatten()
        .collect();
    assert_eq!(remaining.len(), 0);

    let _ = fs::remove_dir_all(&queue_dir);
}

#[test]
fn test_bucket_recreation_on_404() {
    let bucket_created = Arc::new(AtomicUsize::new(0));
    let bucket_created_clone = bucket_created.clone();

    let server = MockServer::start_with_handler(Box::new(move |req| {
        if req.method == "POST" && req.path == "/api/0/buckets/recreate-bucket" {
            bucket_created_clone.fetch_add(1, Ordering::SeqCst);
            return MockResponse::ok();
        }

        if req.path.contains("/heartbeat") {
            if bucket_created_clone.load(Ordering::SeqCst) == 0 {
                // Not created yet -> 404
                return MockResponse::status(404, "Not Found");
            } else {
                return MockResponse::ok();
            }
        }

        MockResponse::ok()
    }));

    let queue_dir = unique_temp_dir("recreate_404");
    let config = ClientConfig {
        base_url: server.base_url(),
        queue_dir: queue_dir.clone(),
        commit_interval_secs: 60,
    };
    let client = Client::start_with_retry_interval(config, Duration::from_millis(50));

    let spec = BucketSpec {
        id: "recreate-bucket".to_string(),
        bucket_type: "currentwindow".to_string(),
        client: "test".to_string(),
        hostname: "localhost".to_string(),
    };
    client.register_bucket(spec);

    let t0 = Utc::now();
    client.heartbeat_at("recreate-bucket", json!({"app": "recreated"}), 2.0, t0);
    client.heartbeat_at(
        "recreate-bucket",
        json!({"app": "second"}),
        2.0,
        t0 + ChronoDuration::seconds(1),
    );

    // Wait for 404 -> recreate -> retry flow
    let mut recreated = false;
    for _ in 0..40 {
        thread::sleep(Duration::from_millis(50));
        if bucket_created.load(Ordering::SeqCst) > 0 {
            recreated = true;
            break;
        }
    }

    assert!(
        recreated,
        "Client should detect 404 and recreate the bucket via POST /api/0/buckets/<id>"
    );

    client.shutdown();

    let hb_success: Vec<_> = server
        .requests()
        .into_iter()
        .filter(|r| r.path.contains("/heartbeat"))
        .collect();
    assert!(
        hb_success.len() >= 2,
        "Should have initial 404 heartbeat and subsequent retried heartbeat"
    );

    let _ = fs::remove_dir_all(&queue_dir);
}

#[test]
fn test_drop_on_400() {
    let requests_count = Arc::new(AtomicUsize::new(0));
    let requests_count_clone = requests_count.clone();

    let server = MockServer::start_with_handler(Box::new(move |req| {
        if req.path.contains("/heartbeat") {
            let c = requests_count_clone.fetch_add(1, Ordering::SeqCst);
            if c == 0 {
                // First heartbeat gets 400 Bad Request
                return MockResponse::status(400, "Bad Request");
            } else {
                return MockResponse::ok();
            }
        }
        MockResponse::ok()
    }));

    let queue_dir = unique_temp_dir("drop_400");
    let config = ClientConfig {
        base_url: server.base_url(),
        queue_dir: queue_dir.clone(),
        commit_interval_secs: 60,
    };
    let client = Client::start_with_retry_interval(config, Duration::from_millis(50));

    let spec = BucketSpec {
        id: "drop-bucket".to_string(),
        bucket_type: "currentwindow".to_string(),
        client: "test".to_string(),
        hostname: "localhost".to_string(),
    };
    client.register_bucket(spec);

    let t0 = Utc::now();
    // Item 1: bad item that gets 400
    client.heartbeat_at("drop-bucket", json!({"app": "bad"}), 2.0, t0);
    // Data change commits item 1
    client.heartbeat_at(
        "drop-bucket",
        json!({"app": "good"}),
        2.0,
        t0 + ChronoDuration::seconds(1),
    );

    // Wait for item 1 to be sent and dropped
    thread::sleep(Duration::from_millis(250));

    // Data change commits item 2 (good)
    client.heartbeat_at(
        "drop-bucket",
        json!({"app": "final"}),
        2.0,
        t0 + ChronoDuration::seconds(2),
    );

    // Wait for item 2 to be sent
    thread::sleep(Duration::from_millis(250));

    assert!(
        requests_count.load(Ordering::SeqCst) >= 2,
        "Second request must proceed after 400 is dropped (queue must not be blocked)"
    );

    client.shutdown();
    let _ = fs::remove_dir_all(&queue_dir);
}

#[test]
fn test_flush_on_shutdown() {
    let server = MockServer::start();
    let queue_dir = unique_temp_dir("flush_shutdown");

    let config = ClientConfig {
        base_url: server.base_url(),
        queue_dir: queue_dir.clone(),
        commit_interval_secs: 60,
    };
    let client = Client::start_with_retry_interval(config, Duration::from_millis(50));

    let spec = BucketSpec {
        id: "flush-bucket".to_string(),
        bucket_type: "currentwindow".to_string(),
        client: "test".to_string(),
        hostname: "localhost".to_string(),
    };
    client.register_bucket(spec);

    // Send single heartbeat: remains buffered in memory because data did not change and interval has not elapsed
    client.heartbeat("flush-bucket", json!({"app": "in-flight"}), 2.0);

    thread::sleep(Duration::from_millis(100));
    assert_eq!(
        server
            .requests()
            .into_iter()
            .filter(|r| r.path.contains("/heartbeat"))
            .count(),
        0,
        "Should not be sent before shutdown"
    );

    // Shutdown should flush the in-flight event and drain queue
    client.shutdown();

    let hb_reqs: Vec<_> = server
        .requests()
        .into_iter()
        .filter(|r| r.path.contains("/heartbeat"))
        .collect();
    assert_eq!(
        hb_reqs.len(),
        1,
        "In-flight event must be flushed on shutdown"
    );

    let ev: Event = hb_reqs[0].json().unwrap();
    assert_eq!(ev.data, json!({"app": "in-flight"}));

    // Verify queue directory pending is empty
    let remaining: Vec<_> = fs::read_dir(queue_dir.join("pending"))
        .unwrap()
        .flatten()
        .collect();
    assert_eq!(remaining.len(), 0);

    let _ = fs::remove_dir_all(&queue_dir);
}
