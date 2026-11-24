use std::path::PathBuf;
use chrono::{Duration, Utc};
use reqwest::StatusCode;
use serde_json::json;
use ty_server::{run, ServerConfig};
use ty_types::{Bucket, Event};

fn spawn_test_server(extension_origins: Vec<String>) -> (ty_server::ServerHandle, String) {
    let config = ServerConfig {
        port: 0, // OS assigns an ephemeral port on 127.0.0.1
        data_dir: PathBuf::from("target/test_data"),
        extension_origins,
    };
    let handle = run(config);
    let port = handle.port();
    let base_url = format!("http://127.0.0.1:{port}");
    (handle, base_url)
}

#[tokio::test]
async fn test_get_info() {
    let (_handle, base_url) = spawn_test_server(vec![]);
    let client = reqwest::Client::new();

    let res = client
        .get(format!("{base_url}/api/0/info"))
        .send()
        .await
        .expect("send request");

    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.expect("json parse");
    assert!(body.get("hostname").is_some());
    assert!(body.get("version").is_some());
    assert_eq!(body.get("testing").and_then(|v| v.as_bool()), Some(false));
}

#[tokio::test]
async fn test_bucket_lifecycle_and_status_codes() {
    let (_handle, base_url) = spawn_test_server(vec![]);
    let client = reqwest::Client::new();
    let bucket_id = "test-bucket-1";

    // 1. GET non-existent bucket -> 404
    let res = client
        .get(format!("{base_url}/api/0/buckets/{bucket_id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 2. POST create bucket -> 200
    let res = client
        .post(format!("{base_url}/api/0/buckets/{bucket_id}"))
        .json(&json!({
            "client": "test-client",
            "type": "test-type",
            "hostname": "test-host",
            "data": {"key": "val"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 3. POST create bucket with same id -> 304 Not Modified (already exists)
    let res = client
        .post(format!("{base_url}/api/0/buckets/{bucket_id}"))
        .json(&json!({
            "client": "test-client",
            "type": "test-type",
            "hostname": "test-host"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_MODIFIED);

    // 4. GET bucket -> 200
    let res = client
        .get(format!("{base_url}/api/0/buckets/{bucket_id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bucket: Bucket = res.json().await.unwrap();
    assert_eq!(bucket.id, bucket_id);
    assert_eq!(bucket.client, "test-client");
    assert_eq!(bucket.bucket_type, "test-type");
    assert_eq!(bucket.hostname, "test-host");

    // 5. GET /api/0/buckets/ (list) -> 200
    let res = client
        .get(format!("{base_url}/api/0/buckets/"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let list: std::collections::HashMap<String, Bucket> = res.json().await.unwrap();
    assert!(list.contains_key(bucket_id));

    // Also test without trailing slash: GET /api/0/buckets -> 200
    let res = client
        .get(format!("{base_url}/api/0/buckets"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 6. DELETE bucket -> 200
    let res = client
        .delete(format!("{base_url}/api/0/buckets/{bucket_id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 7. DELETE bucket again -> 404
    let res = client
        .delete(format!("{base_url}/api/0/buckets/{bucket_id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_hostname_sanitizing_and_local() {
    let (_handle, base_url) = spawn_test_server(vec![]);
    let client = reqwest::Client::new();

    // 1. Hostname with whitespace -> 400 Bad Request
    let res = client
        .post(format!("{base_url}/api/0/buckets/bucket-space"))
        .json(&json!({
            "client": "test",
            "type": "test",
            "hostname": "my laptop host"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 2. Hostname "!local" is replaced by server's own hostname
    let res = client
        .post(format!("{base_url}/api/0/buckets/bucket-local"))
        .json(&json!({
            "client": "test",
            "type": "test",
            "hostname": "!local"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = client
        .get(format!("{base_url}/api/0/buckets/bucket-local"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bucket: Bucket = res.json().await.unwrap();
    assert_ne!(bucket.hostname, "!local");
    assert!(!bucket.hostname.is_empty());
    assert!(!bucket.hostname.chars().any(char::is_whitespace));
}

#[tokio::test]
async fn test_events_endpoints_and_status_codes() {
    let (_handle, base_url) = spawn_test_server(vec![]);
    let client = reqwest::Client::new();
    let bucket_id = "events-bucket";

    // Calling events endpoints on non-existent bucket -> 404
    let res = client
        .get(format!("{base_url}/api/0/buckets/{bucket_id}/events"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let res = client
        .get(format!("{base_url}/api/0/buckets/{bucket_id}/events/count"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let res = client
        .post(format!("{base_url}/api/0/buckets/{bucket_id}/events"))
        .json(&json!([]))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let res = client
        .delete(format!("{base_url}/api/0/buckets/{bucket_id}/events/1"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // Create bucket
    let res = client
        .post(format!("{base_url}/api/0/buckets/{bucket_id}"))
        .json(&json!({"client": "test", "type": "test", "hostname": "host"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Insert batch of events
    let t1 = Utc::now() - Duration::seconds(10);
    let t2 = Utc::now() - Duration::seconds(5);
    let t3 = Utc::now();

    let events = vec![
        Event::new(t1, 2.0, json!({"seq": 1})),
        Event::new(t2, 3.0, json!({"seq": 2})),
        Event::new(t3, 1.0, json!({"seq": 3})),
    ];

    let res = client
        .post(format!("{base_url}/api/0/buckets/{bucket_id}/events"))
        .json(&events)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let inserted: Vec<Event> = res.json().await.unwrap();
    assert_eq!(inserted.len(), 3);
    assert!(inserted[0].id.is_some());
    assert!(inserted[1].id.is_some());
    assert!(inserted[2].id.is_some());

    let ev2_id = inserted[1].id.unwrap();

    // GET count -> 3
    let res = client
        .get(format!("{base_url}/api/0/buckets/{bucket_id}/events/count"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let count: usize = res.json().await.unwrap();
    assert_eq!(count, 3);

    // GET events newest first
    let res = client
        .get(format!("{base_url}/api/0/buckets/{bucket_id}/events"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let fetched: Vec<Event> = res.json().await.unwrap();
    assert_eq!(fetched.len(), 3);
    assert_eq!(fetched[0].data["seq"], 3);
    assert_eq!(fetched[1].data["seq"], 2);
    assert_eq!(fetched[2].data["seq"], 1);

    // GET events with limit=2
    let res = client
        .get(format!(
            "{base_url}/api/0/buckets/{bucket_id}/events?limit=2"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let fetched_limited: Vec<Event> = res.json().await.unwrap();
    assert_eq!(fetched_limited.len(), 2);
    assert_eq!(fetched_limited[0].data["seq"], 3);
    assert_eq!(fetched_limited[1].data["seq"], 2);

    // GET events with time range
    let start_str = (t2 - Duration::seconds(1)).to_rfc3339();
    let end_str = (t2 + Duration::seconds(1)).to_rfc3339();
    let res = client
        .get(format!(
            "{base_url}/api/0/buckets/{bucket_id}/events?start={start_str}&end={end_str}"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let range_events: Vec<Event> = res.json().await.unwrap();
    assert_eq!(range_events.len(), 1);
    assert_eq!(range_events[0].data["seq"], 2);

    // Bad timestamp query -> 400
    let res = client
        .get(format!(
            "{base_url}/api/0/buckets/{bucket_id}/events?start=invalid-date"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // DELETE event by id -> 200
    let res = client
        .delete(format!(
            "{base_url}/api/0/buckets/{bucket_id}/events/{ev2_id}"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // DELETE event again -> 404
    let res = client
        .delete(format!(
            "{base_url}/api/0/buckets/{bucket_id}/events/{ev2_id}"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // DELETE event with non-integer id -> 400
    let res = client
        .delete(format!(
            "{base_url}/api/0/buckets/{bucket_id}/events/not-an-id"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_heartbeat_merge_and_insert() {
    let (_handle, base_url) = spawn_test_server(vec![]);
    let client = reqwest::Client::new();
    let bucket_id = "heartbeat-bucket";

    // 1. Heartbeat on non-existent bucket -> 404
    let res = client
        .post(format!(
            "{base_url}/api/0/buckets/{bucket_id}/heartbeat?pulsetime=5.0"
        ))
        .json(&json!({
            "timestamp": Utc::now().to_rfc3339(),
            "duration": 0.0,
            "data": {"app": "code.exe"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // Create bucket
    client
        .post(format!("{base_url}/api/0/buckets/{bucket_id}"))
        .json(&json!({"client": "test", "type": "test", "hostname": "host"}))
        .send()
        .await
        .unwrap();

    // 2. Negative pulsetime -> 400
    let res = client
        .post(format!(
            "{base_url}/api/0/buckets/{bucket_id}/heartbeat?pulsetime=-1.0"
        ))
        .json(&json!({
            "timestamp": Utc::now().to_rfc3339(),
            "duration": 0.0,
            "data": {"app": "code.exe"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 3. First heartbeat on empty bucket -> inserted (duration = 0)
    let t0 = Utc::now();
    let res = client
        .post(format!(
            "{base_url}/api/0/buckets/{bucket_id}/heartbeat?pulsetime=5.0"
        ))
        .json(&json!({
            "timestamp": t0.to_rfc3339(),
            "duration": 0.0,
            "data": {"app": "code.exe"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let hb1: Event = res.json().await.unwrap();
    assert_eq!(hb1.duration, 0.0);
    assert_eq!(hb1.data["app"], "code.exe");
    let event_id = hb1.id.unwrap();

    // 4. Second heartbeat: same data, within pulsetime (1s later) -> merged!
    let t1 = t0 + Duration::seconds(1);
    let res = client
        .post(format!(
            "{base_url}/api/0/buckets/{bucket_id}/heartbeat?pulsetime=5.0"
        ))
        .json(&json!({
            "timestamp": t1.to_rfc3339(),
            "duration": 0.0,
            "data": {"app": "code.exe"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let hb2: Event = res.json().await.unwrap();
    assert_eq!(hb2.id, Some(event_id)); // Kept original event id
    assert_eq!(hb2.timestamp, hb1.timestamp); // Kept original start timestamp
    assert!((hb2.duration - 1.0).abs() < 1e-4); // Duration updated to 1.0

    // Only 1 event exists in the store because it merged in place
    let count: usize = client
        .get(format!("{base_url}/api/0/buckets/{bucket_id}/events/count"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(count, 1);

    // 5. Third heartbeat: different data -> NOT merged, inserted as new event
    let t2 = t1 + Duration::seconds(1);
    let res = client
        .post(format!(
            "{base_url}/api/0/buckets/{bucket_id}/heartbeat?pulsetime=5.0"
        ))
        .json(&json!({
            "timestamp": t2.to_rfc3339(),
            "duration": 0.0,
            "data": {"app": "firefox.exe"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let hb3: Event = res.json().await.unwrap();
    assert_ne!(hb3.id, Some(event_id)); // New event id
    assert_eq!(hb3.data["app"], "firefox.exe");

    let count: usize = client
        .get(format!("{base_url}/api/0/buckets/{bucket_id}/events/count"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(count, 2);

    // 6. Fourth heartbeat: same data as firefox, but gap exceeds pulsetime (gap = 20s, pulsetime = 5s) -> NOT merged, inserted
    let t3 = t2 + Duration::seconds(20);
    let res = client
        .post(format!(
            "{base_url}/api/0/buckets/{bucket_id}/heartbeat?pulsetime=5.0"
        ))
        .json(&json!({
            "timestamp": t3.to_rfc3339(),
            "duration": 0.0,
            "data": {"app": "firefox.exe"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let hb4: Event = res.json().await.unwrap();
    assert_ne!(hb4.id, hb3.id);

    let count: usize = client
        .get(format!("{base_url}/api/0/buckets/{bucket_id}/events/count"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(count, 3);
}

#[tokio::test]
async fn test_query_endpoint_returns_501() {
    let (_handle, base_url) = spawn_test_server(vec![]);
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{base_url}/api/0/query"))
        .json(&json!({
            "timeperiods": ["2026-01-01/2026-01-02"],
            "query": ["RETURN = 1;"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn test_cors_allow_and_deny() {
    let allowed_ext = "chrome-extension://abcdefghijklmnop";
    let (handle, base_url) = spawn_test_server(vec![allowed_ext.to_string()]);
    let port = handle.port();
    let client = reqwest::Client::new();

    // 1. Direct request (no Origin header) -> allowed
    let res = client
        .get(format!("{base_url}/api/0/info"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2. Localhost origin on server port -> allowed + CORS headers returned
    let res = client
        .get(format!("{base_url}/api/0/info"))
        .header("Origin", format!("http://localhost:{port}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get("access-control-allow-origin").unwrap(),
        &format!("http://localhost:{port}")
    );

    // 3. 127.0.0.1 origin on server port -> allowed + CORS header returned
    let res = client
        .get(format!("{base_url}/api/0/info"))
        .header("Origin", format!("http://127.0.0.1:{port}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get("access-control-allow-origin").unwrap(),
        &format!("http://127.0.0.1:{port}")
    );

    // 4. Allowed extension origin in settings -> allowed
    let res = client
        .get(format!("{base_url}/api/0/info"))
        .header("Origin", allowed_ext)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get("access-control-allow-origin").unwrap(),
        allowed_ext
    );

    // 5. Unknown extension or web origin -> 403 Forbidden
    let res = client
        .get(format!("{base_url}/api/0/info"))
        .header("Origin", "http://evil-site.com")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let res = client
        .get(format!("{base_url}/api/0/info"))
        .header("Origin", "chrome-extension://unauthorized-ext-id")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 6. Preflight OPTIONS for allowed origin -> 200 OK + CORS headers
    let res = client
        .request(reqwest::Method::OPTIONS, format!("{base_url}/api/0/info"))
        .header("Origin", format!("http://localhost:{port}"))
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get("access-control-allow-origin").unwrap(),
        &format!("http://localhost:{port}")
    );
    assert!(res.headers().get("access-control-allow-methods").is_some());

    // 7. Preflight OPTIONS for forbidden origin -> 403 Forbidden
    let res = client
        .request(reqwest::Method::OPTIONS, format!("{base_url}/api/0/info"))
        .header("Origin", "http://evil-site.com")
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_cors_moz_extension_restrictions() {
    let (_handle, base_url) = spawn_test_server(vec![]);
    let client = reqwest::Client::new();
    let moz_origin = "moz-extension://a1b2c3d4-e5f6-7890-abcd-ef1234567890";

    // 1. moz-extension is allowed to call GET /api/0/info
    let res = client
        .get(format!("{base_url}/api/0/info"))
        .header("Origin", moz_origin)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get("access-control-allow-origin").unwrap(),
        moz_origin
    );

    // 2. moz-extension is allowed to POST on aw-watcher-web-* buckets
    let web_bucket = "aw-watcher-web-firefox_myhost";
    let res = client
        .post(format!("{base_url}/api/0/buckets/{web_bucket}"))
        .header("Origin", moz_origin)
        .json(&json!({"client": "aw-watcher-web", "type": "web.tab.current", "hostname": "myhost"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get("access-control-allow-origin").unwrap(),
        moz_origin
    );

    // 3. moz-extension is allowed to POST /heartbeat on aw-watcher-web-* buckets
    let res = client
        .post(format!(
            "{base_url}/api/0/buckets/{web_bucket}/heartbeat?pulsetime=10"
        ))
        .header("Origin", moz_origin)
        .json(&json!({
            "timestamp": Utc::now().to_rfc3339(),
            "duration": 0.0,
            "data": {"url": "https://example.com", "title": "Example"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 4. moz-extension is allowed to POST /events on aw-watcher-web-* buckets
    let res = client
        .post(format!("{base_url}/api/0/buckets/{web_bucket}/events"))
        .header("Origin", moz_origin)
        .json(&json!([
            {"timestamp": Utc::now().to_rfc3339(), "duration": 1.0, "data": {"url": "https://example.com"}}
        ]))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 5. moz-extension calling GET on buckets -> 403 Forbidden!
    let res = client
        .get(format!("{base_url}/api/0/buckets/"))
        .header("Origin", moz_origin)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let res = client
        .get(format!("{base_url}/api/0/buckets/{web_bucket}"))
        .header("Origin", moz_origin)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 6. moz-extension calling DELETE on buckets -> 403 Forbidden!
    let res = client
        .delete(format!("{base_url}/api/0/buckets/{web_bucket}"))
        .header("Origin", moz_origin)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 7. moz-extension calling POST on non aw-watcher-web bucket (e.g. window watcher) -> 403 Forbidden!
    let res = client
        .post(format!(
            "{base_url}/api/0/buckets/aw-watcher-window_myhost"
        ))
        .header("Origin", moz_origin)
        .json(&json!({"client": "test", "type": "test", "hostname": "host"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 8. Preflight OPTIONS for moz-extension on allowed path -> 200 OK
    let res = client
        .request(reqwest::Method::OPTIONS, format!("{base_url}/api/0/info"))
        .header("Origin", moz_origin)
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = client
        .request(
            reqwest::Method::OPTIONS,
            format!("{base_url}/api/0/buckets/{web_bucket}/heartbeat"),
        )
        .header("Origin", moz_origin)
        .header("Access-Control-Request-Method", "POST")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 9. Preflight OPTIONS for moz-extension on forbidden path/method -> 403 Forbidden
    let res = client
        .request(
            reqwest::Method::OPTIONS,
            format!("{base_url}/api/0/buckets/aw-watcher-window_myhost"),
        )
        .header("Origin", moz_origin)
        .header("Access-Control-Request-Method", "POST")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let res = client
        .request(
            reqwest::Method::OPTIONS,
            format!("{base_url}/api/0/buckets/{web_bucket}"),
        )
        .header("Origin", moz_origin)
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_clean_shutdown() {
    let (handle, base_url) = spawn_test_server(vec![]);
    let client = reqwest::Client::new();

    // Verify server is responsive before shutdown
    let res = client
        .get(format!("{base_url}/api/0/info"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Trigger shutdown
    handle.shutdown();

    // Request after shutdown should fail to connect
    let res = client.get(format!("{base_url}/api/0/info")).send().await;
    assert!(
        res.is_err(),
        "Server should refuse connections after clean shutdown"
    );
}
