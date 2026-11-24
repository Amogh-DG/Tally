use std::sync::Arc;
use std::thread;
use chrono::{DateTime, TimeZone, Utc};
use serde_json::json;
use ty_store::SqliteStore;
use ty_types::{Bucket, Event, Store, StoreError};

fn sample_bucket(id: &str) -> Bucket {
    Bucket {
        id: id.to_string(),
        bucket_type: "currentwindow".to_string(),
        client: "aw-watcher-window".to_string(),
        hostname: "test-host".to_string(),
        created: Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
        data: json!({"extra": "info"}),
    }
}

fn sample_event(ts: DateTime<Utc>, duration: f64, app: &str) -> Event {
    Event::new(ts, duration, json!({"app": app, "title": "test title"}))
}

#[test]
fn test_in_memory_bucket_crud() {
    let store = SqliteStore::open_in_memory().expect("open in-memory store");

    // Initially empty
    let buckets = store.list_buckets().expect("list buckets");
    assert!(buckets.is_empty());

    // Create bucket
    let b1 = sample_bucket("bucket-1");
    store.create_bucket(&b1).expect("create bucket-1");

    // Get bucket
    let fetched = store.get_bucket("bucket-1").expect("get bucket-1");
    assert_eq!(fetched.id, b1.id);
    assert_eq!(fetched.bucket_type, b1.bucket_type);
    assert_eq!(fetched.client, b1.client);
    assert_eq!(fetched.hostname, b1.hostname);
    assert_eq!(fetched.created, b1.created);
    assert_eq!(fetched.data, b1.data);

    // Duplicate create returns AlreadyExists
    let err = store.create_bucket(&b1).unwrap_err();
    assert_eq!(err, StoreError::AlreadyExists);

    // List buckets contains bucket-1
    let buckets = store.list_buckets().expect("list buckets");
    assert_eq!(buckets.len(), 1);
    assert_eq!(buckets.get("bucket-1").unwrap().id, "bucket-1");

    // Create second bucket
    let b2 = sample_bucket("bucket-2");
    store.create_bucket(&b2).expect("create bucket-2");
    assert_eq!(store.list_buckets().unwrap().len(), 2);

    // Delete bucket-1
    store.delete_bucket("bucket-1").expect("delete bucket-1");
    assert_eq!(
        store.get_bucket("bucket-1").unwrap_err(),
        StoreError::NotFound
    );
    assert_eq!(
        store.delete_bucket("bucket-1").unwrap_err(),
        StoreError::NotFound
    );

    // bucket-2 still exists
    assert_eq!(store.list_buckets().unwrap().len(), 1);
}

#[test]
fn test_in_memory_event_crud() {
    let store = SqliteStore::open_in_memory().expect("open in-memory store");
    let b = sample_bucket("b1");
    store.create_bucket(&b).expect("create bucket");

    // Empty bucket get_last_event returns Ok(None)
    assert_eq!(store.get_last_event("b1").unwrap(), None);

    // Insert event
    let t1 = Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).unwrap();
    let e1 = sample_event(t1, 5.0, "code.exe");
    let inserted1 = store.insert_event("b1", &e1).expect("insert e1");
    assert!(inserted1.id.is_some());
    let id1 = inserted1.id.unwrap();
    assert_eq!(inserted1.timestamp, t1);
    assert_eq!(inserted1.duration, 5.0);

    // Last event is now inserted1
    let last = store.get_last_event("b1").unwrap().expect("last event");
    assert_eq!(last.id, Some(id1));

    // Event count
    assert_eq!(store.event_count("b1", None, None).unwrap(), 1);

    // Insert second event
    let t2 = Utc.with_ymd_and_hms(2026, 1, 1, 10, 5, 0).unwrap();
    let e2 = sample_event(t2, 10.0, "firefox.exe");
    let inserted2 = store.insert_event("b1", &e2).expect("insert e2");
    assert_eq!(store.event_count("b1", None, None).unwrap(), 2);

    let last2 = store.get_last_event("b1").unwrap().expect("last event");
    assert_eq!(last2.id, inserted2.id);

    // Delete event 1
    store.delete_event("b1", id1).expect("delete event 1");
    assert_eq!(store.event_count("b1", None, None).unwrap(), 1);

    // Delete again returns NotFound
    assert_eq!(
        store.delete_event("b1", id1).unwrap_err(),
        StoreError::NotFound
    );
}

#[test]
fn test_newest_first_ordering() {
    let store = SqliteStore::open_in_memory().expect("open in-memory store");
    store
        .create_bucket(&sample_bucket("test-order"))
        .expect("create bucket");

    let t1 = Utc.with_ymd_and_hms(2026, 1, 1, 1, 0, 0).unwrap();
    let t2 = Utc.with_ymd_and_hms(2026, 1, 1, 2, 0, 0).unwrap();
    let t3 = Utc.with_ymd_and_hms(2026, 1, 1, 3, 0, 0).unwrap();

    // Insert out of chronological order
    store
        .insert_event("test-order", &sample_event(t2, 1.0, "second"))
        .unwrap();
    store
        .insert_event("test-order", &sample_event(t1, 1.0, "first"))
        .unwrap();
    store
        .insert_event("test-order", &sample_event(t3, 1.0, "third"))
        .unwrap();

    let events = store
        .get_events("test-order", None, None, None)
        .expect("get events");
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].timestamp, t3);
    assert_eq!(events[1].timestamp, t2);
    assert_eq!(events[2].timestamp, t1);
    assert_eq!(events[0].data["app"], "third");
    assert_eq!(events[1].data["app"], "second");
    assert_eq!(events[2].data["app"], "first");
}

#[test]
fn test_start_end_limit_filters() {
    let store = SqliteStore::open_in_memory().expect("open in-memory store");
    store
        .create_bucket(&sample_bucket("filters"))
        .expect("create bucket");

    // Insert 10 events: hour 00 to 09
    for hour in 0..10 {
        let ts = Utc.with_ymd_and_hms(2026, 1, 1, hour, 0, 0).unwrap();
        store
            .insert_event("filters", &sample_event(ts, 60.0, &format!("app_{hour}")))
            .unwrap();
    }

    assert_eq!(store.event_count("filters", None, None).unwrap(), 10);

    let t3 = Utc.with_ymd_and_hms(2026, 1, 1, 3, 0, 0).unwrap();
    let t6 = Utc.with_ymd_and_hms(2026, 1, 1, 6, 0, 0).unwrap();

    // Filter start only (hour >= 3 -> 3..=9 = 7 events)
    let ev_start = store
        .get_events("filters", Some(t3), None, None)
        .expect("get start");
    assert_eq!(ev_start.len(), 7);
    assert_eq!(ev_start[0].data["app"], "app_9");
    assert_eq!(ev_start.last().unwrap().data["app"], "app_3");
    assert_eq!(store.event_count("filters", Some(t3), None).unwrap(), 7);

    // Filter end only (hour <= 6 -> 0..=6 = 7 events)
    let ev_end = store
        .get_events("filters", None, Some(t6), None)
        .expect("get end");
    assert_eq!(ev_end.len(), 7);
    assert_eq!(ev_end[0].data["app"], "app_6");
    assert_eq!(ev_end.last().unwrap().data["app"], "app_0");
    assert_eq!(store.event_count("filters", None, Some(t6)).unwrap(), 7);

    // Filter start and end (3 <= hour <= 6 -> 4 events: 6, 5, 4, 3)
    let ev_range = store
        .get_events("filters", Some(t3), Some(t6), None)
        .expect("get range");
    assert_eq!(ev_range.len(), 4);
    assert_eq!(ev_range[0].data["app"], "app_6");
    assert_eq!(ev_range[1].data["app"], "app_5");
    assert_eq!(ev_range[2].data["app"], "app_4");
    assert_eq!(ev_range[3].data["app"], "app_3");
    assert_eq!(
        store.event_count("filters", Some(t3), Some(t6)).unwrap(),
        4
    );

    // Limit only (limit 3 newest -> 9, 8, 7)
    let ev_lim = store
        .get_events("filters", None, None, Some(3))
        .expect("get limit");
    assert_eq!(ev_lim.len(), 3);
    assert_eq!(ev_lim[0].data["app"], "app_9");
    assert_eq!(ev_lim[1].data["app"], "app_8");
    assert_eq!(ev_lim[2].data["app"], "app_7");

    // Start + Limit (start = t3, limit = 2 -> 9, 8)
    let ev_start_lim = store
        .get_events("filters", Some(t3), None, Some(2))
        .expect("get start lim");
    assert_eq!(ev_start_lim.len(), 2);
    assert_eq!(ev_start_lim[0].data["app"], "app_9");
    assert_eq!(ev_start_lim[1].data["app"], "app_8");

    // Start + End + Limit (3 <= hour <= 6, limit 2 -> 6, 5)
    let ev_all = store
        .get_events("filters", Some(t3), Some(t6), Some(2))
        .expect("get start end lim");
    assert_eq!(ev_all.len(), 2);
    assert_eq!(ev_all[0].data["app"], "app_6");
    assert_eq!(ev_all[1].data["app"], "app_5");
}

#[test]
fn test_replace_last_event_only_touches_latest() {
    let store = SqliteStore::open_in_memory().expect("open in-memory store");
    store
        .create_bucket(&sample_bucket("replace-test"))
        .expect("create bucket");

    // Replace on empty bucket returns NotFound
    let dummy = sample_event(Utc::now(), 1.0, "dummy");
    assert_eq!(
        store.replace_last_event("replace-test", &dummy).unwrap_err(),
        StoreError::NotFound
    );

    let t1 = Utc.with_ymd_and_hms(2026, 1, 1, 1, 0, 0).unwrap();
    let t2 = Utc.with_ymd_and_hms(2026, 1, 1, 2, 0, 0).unwrap();
    let t3 = Utc.with_ymd_and_hms(2026, 1, 1, 3, 0, 0).unwrap();

    let e1 = store
        .insert_event("replace-test", &sample_event(t1, 10.0, "app1"))
        .unwrap();
    let e2 = store
        .insert_event("replace-test", &sample_event(t2, 20.0, "app2"))
        .unwrap();
    let e3 = store
        .insert_event("replace-test", &sample_event(t3, 30.0, "app3"))
        .unwrap();

    // Update latest event (which is e3)
    let t3_updated = Utc.with_ymd_and_hms(2026, 1, 1, 3, 5, 0).unwrap();
    let updated_req = Event::new(
        t3_updated,
        45.5,
        json!({"app": "app3_merged", "extra": true}),
    );

    let replaced = store
        .replace_last_event("replace-test", &updated_req)
        .expect("replace latest");

    assert_eq!(replaced.id, e3.id);
    assert_eq!(replaced.timestamp, t3_updated);
    assert_eq!(replaced.duration, 45.5);
    assert_eq!(replaced.data, updated_req.data);

    // Verify all events in the store
    let events = store
        .get_events("replace-test", None, None, None)
        .expect("get events");
    assert_eq!(events.len(), 3);

    // Latest is e3 with updated values
    assert_eq!(events[0].id, e3.id);
    assert_eq!(events[0].timestamp, t3_updated);
    assert_eq!(events[0].duration, 45.5);
    assert_eq!(events[0].data["app"], "app3_merged");

    // e2 is completely untouched!
    assert_eq!(events[1].id, e2.id);
    assert_eq!(events[1].timestamp, t2);
    assert_eq!(events[1].duration, 20.0);
    assert_eq!(events[1].data["app"], "app2");

    // e1 is completely untouched!
    assert_eq!(events[2].id, e1.id);
    assert_eq!(events[2].timestamp, t1);
    assert_eq!(events[2].duration, 10.0);
    assert_eq!(events[2].data["app"], "app1");
}

#[test]
fn test_delete_bucket_deletes_its_events() {
    let store = SqliteStore::open_in_memory().expect("open in-memory store");
    store
        .create_bucket(&sample_bucket("bucket_a"))
        .expect("create bucket A");
    store
        .create_bucket(&sample_bucket("bucket_b"))
        .expect("create bucket B");

    let t = Utc::now();
    store
        .insert_event("bucket_a", &sample_event(t, 1.0, "a1"))
        .unwrap();
    store
        .insert_event("bucket_a", &sample_event(t, 2.0, "a2"))
        .unwrap();
    store
        .insert_event("bucket_a", &sample_event(t, 3.0, "a3"))
        .unwrap();

    store
        .insert_event("bucket_b", &sample_event(t, 10.0, "b1"))
        .unwrap();
    store
        .insert_event("bucket_b", &sample_event(t, 20.0, "b2"))
        .unwrap();

    assert_eq!(store.event_count("bucket_a", None, None).unwrap(), 3);
    assert_eq!(store.event_count("bucket_b", None, None).unwrap(), 2);

    // Delete bucket A
    store.delete_bucket("bucket_a").expect("delete bucket A");

    // Bucket A and its events are gone
    assert_eq!(
        store.get_bucket("bucket_a").unwrap_err(),
        StoreError::NotFound
    );
    assert_eq!(
        store.get_events("bucket_a", None, None, None).unwrap_err(),
        StoreError::NotFound
    );
    assert_eq!(
        store.event_count("bucket_a", None, None).unwrap_err(),
        StoreError::NotFound
    );
    assert_eq!(
        store.get_last_event("bucket_a").unwrap_err(),
        StoreError::NotFound
    );

    // Bucket B is completely intact
    assert_eq!(store.get_bucket("bucket_b").unwrap().id, "bucket_b");
    assert_eq!(store.event_count("bucket_b", None, None).unwrap(), 2);
    let events_b = store
        .get_events("bucket_b", None, None, None)
        .expect("get events b");
    assert_eq!(events_b.len(), 2);
}

#[test]
fn test_temp_file_db_all_operations() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("subfolder").join("temp_store.db");

    let store = SqliteStore::open(&db_path).expect("open file store");

    let b = sample_bucket("file-bucket");
    store.create_bucket(&b).expect("create bucket");
    assert_eq!(store.get_bucket("file-bucket").unwrap().id, "file-bucket");

    let t1 = Utc.with_ymd_and_hms(2026, 5, 1, 10, 0, 0).unwrap();
    let e1 = store
        .insert_event("file-bucket", &sample_event(t1, 5.0, "app1"))
        .unwrap();
    let t2 = Utc.with_ymd_and_hms(2026, 5, 1, 11, 0, 0).unwrap();
    let e2 = store
        .insert_event("file-bucket", &sample_event(t2, 10.0, "app2"))
        .unwrap();

    assert_eq!(store.event_count("file-bucket", None, None).unwrap(), 2);
    let last = store.get_last_event("file-bucket").unwrap().unwrap();
    assert_eq!(last.id, e2.id);

    // replace last
    let t2_new = Utc.with_ymd_and_hms(2026, 5, 1, 11, 30, 0).unwrap();
    let rep = store
        .replace_last_event(
            "file-bucket",
            &Event::new(t2_new, 15.0, json!({"app": "app2_mod"})),
        )
        .unwrap();
    assert_eq!(rep.id, e2.id);

    // delete event
    store
        .delete_event("file-bucket", e1.id.unwrap())
        .expect("delete event 1");
    assert_eq!(store.event_count("file-bucket", None, None).unwrap(), 1);

    // delete bucket
    store
        .delete_bucket("file-bucket")
        .expect("delete file-bucket");
    assert_eq!(
        store.get_bucket("file-bucket").unwrap_err(),
        StoreError::NotFound
    );
}

#[test]
fn test_reopen_from_disk_roundtrip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("roundtrip.db");

    let created_dt: DateTime<Utc> = "2026-11-23T15:20:30.123456789Z"
        .parse::<DateTime<Utc>>()
        .unwrap();
    let t1: DateTime<Utc> = "2026-11-23T15:21:00.987654321Z"
        .parse::<DateTime<Utc>>()
        .unwrap();
    let t2: DateTime<Utc> = "2026-11-23T15:22:00.000000001Z"
        .parse::<DateTime<Utc>>()
        .unwrap();

    let dur1 = 12.345678901234;
    let dur2 = 0.0;

    let b_data = json!({"device": "laptop", "version": 42, "nested": {"a": [1, 2, 3]}});
    let e1_data = json!({"app": "code.exe", "title": "lib.rs - Tally"});
    let e2_data = json!({"status": "not-afk"});

    let id1;
    let id2;

    {
        let store = SqliteStore::open(&db_path).expect("open initial store");
        let bucket = Bucket {
            id: "roundtrip-bucket".to_string(),
            bucket_type: "currentwindow".to_string(),
            client: "aw-watcher-window".to_string(),
            hostname: "my-pc".to_string(),
            created: created_dt,
            data: b_data.clone(),
        };
        store.create_bucket(&bucket).expect("create bucket");

        let res1 = store
            .insert_event("roundtrip-bucket", &Event::new(t1, dur1, e1_data.clone()))
            .unwrap();
        let res2 = store
            .insert_event("roundtrip-bucket", &Event::new(t2, dur2, e2_data.clone()))
            .unwrap();

        id1 = res1.id.unwrap();
        id2 = res2.id.unwrap();
    } // store dropped and connection closed

    // Reopen from same path
    let store2 = SqliteStore::open(&db_path).expect("reopen store");

    // Verify bucket
    let bucket = store2
        .get_bucket("roundtrip-bucket")
        .expect("get bucket after reopen");
    assert_eq!(bucket.id, "roundtrip-bucket");
    assert_eq!(bucket.bucket_type, "currentwindow");
    assert_eq!(bucket.client, "aw-watcher-window");
    assert_eq!(bucket.hostname, "my-pc");
    assert_eq!(bucket.created, created_dt);
    assert_eq!(bucket.data, b_data);

    // Verify events: ordering, lossless timestamps and durations
    let events = store2
        .get_events("roundtrip-bucket", None, None, None)
        .expect("get events after reopen");
    assert_eq!(events.len(), 2);

    // Newest first -> e2 then e1
    assert_eq!(events[0].id, Some(id2));
    assert_eq!(events[0].timestamp, t2);
    assert_eq!(events[0].duration, dur2);
    assert_eq!(events[0].data, e2_data);

    assert_eq!(events[1].id, Some(id1));
    assert_eq!(events[1].timestamp, t1);
    assert_eq!(events[1].duration, dur1);
    assert_eq!(events[1].data, e1_data);
}

#[test]
fn test_concurrent_access() {
    let store = Arc::new(SqliteStore::open_in_memory().expect("open store"));
    store
        .create_bucket(&sample_bucket("concurrent-bucket"))
        .expect("create bucket");

    let num_threads = 8;
    let events_per_thread = 20;

    let mut handles = Vec::new();
    for thread_idx in 0..num_threads {
        let store_clone = Arc::clone(&store);
        let handle = thread::spawn(move || {
            for i in 0..events_per_thread {
                let ts = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
                    + chrono::Duration::milliseconds((thread_idx * 100 + i) as i64);
                let e = sample_event(ts, i as f64, &format!("t{thread_idx}_e{i}"));
                store_clone
                    .insert_event("concurrent-bucket", &e)
                    .expect("insert in thread");
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("thread join");
    }

    let count = store
        .event_count("concurrent-bucket", None, None)
        .expect("count");
    assert_eq!(count, num_threads * events_per_thread);

    let events = store
        .get_events("concurrent-bucket", None, None, None)
        .expect("get all");
    assert_eq!(events.len(), num_threads * events_per_thread);

    // Verify strictly sorted newest first
    for window in events.windows(2) {
        assert!(window[0].timestamp >= window[1].timestamp);
    }
}

#[test]
fn test_validation_and_not_found_errors() {
    let store = SqliteStore::open_in_memory().expect("open store");

    // Invalid bucket data (not JSON object)
    let mut invalid_b = sample_bucket("bad-data");
    invalid_b.data = json!("not an object");
    assert!(matches!(
        store.create_bucket(&invalid_b).unwrap_err(),
        StoreError::BadRequest(_)
    ));

    // Valid bucket
    store
        .create_bucket(&sample_bucket("valid-b"))
        .expect("create");

    // Invalid event data (not JSON object)
    let bad_ev = Event::new(Utc::now(), 1.0, json!([1, 2, 3]));
    assert!(matches!(
        store.insert_event("valid-b", &bad_ev).unwrap_err(),
        StoreError::BadRequest(_)
    ));

    // Operations on non-existent bucket return NotFound
    assert_eq!(
        store.get_bucket("no-such-b").unwrap_err(),
        StoreError::NotFound
    );
    assert_eq!(
        store.delete_bucket("no-such-b").unwrap_err(),
        StoreError::NotFound
    );
    assert_eq!(
        store
            .insert_event("no-such-b", &sample_event(Utc::now(), 1.0, "x"))
            .unwrap_err(),
        StoreError::NotFound
    );
    assert_eq!(
        store.get_last_event("no-such-b").unwrap_err(),
        StoreError::NotFound
    );
    assert_eq!(
        store
            .replace_last_event("no-such-b", &sample_event(Utc::now(), 1.0, "x"))
            .unwrap_err(),
        StoreError::NotFound
    );
    assert_eq!(
        store
            .get_events("no-such-b", None, None, None)
            .unwrap_err(),
        StoreError::NotFound
    );
    assert_eq!(
        store.event_count("no-such-b", None, None).unwrap_err(),
        StoreError::NotFound
    );
    assert_eq!(
        store.delete_event("no-such-b", 1).unwrap_err(),
        StoreError::NotFound
    );
}

#[test]
fn test_identical_timestamps_tie_breaking() {
    let store = SqliteStore::open_in_memory().expect("open store");
    store
        .create_bucket(&sample_bucket("tie-bucket"))
        .expect("create bucket");

    let t = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
    let e1 = store
        .insert_event("tie-bucket", &sample_event(t, 1.0, "app_tie_1"))
        .unwrap();
    let e2 = store
        .insert_event("tie-bucket", &sample_event(t, 2.0, "app_tie_2"))
        .unwrap();

    assert!(e2.id.unwrap() > e1.id.unwrap());

    // get_last_event returns e2 (latest id when timestamps match)
    let last = store
        .get_last_event("tie-bucket")
        .unwrap()
        .expect("last event");
    assert_eq!(last.id, e2.id);

    // get_events returns [e2, e1]
    let events = store
        .get_events("tie-bucket", None, None, None)
        .expect("get events");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].id, e2.id);
    assert_eq!(events[1].id, e1.id);

    // replace_last_event updates e2, keeping e1 untouched
    let rep = store
        .replace_last_event(
            "tie-bucket",
            &Event::new(t, 99.0, json!({"app": "app_tie_2_updated"})),
        )
        .unwrap();
    assert_eq!(rep.id, e2.id);

    let events_after = store
        .get_events("tie-bucket", None, None, None)
        .expect("get events after replace");
    assert_eq!(events_after[0].id, e2.id);
    assert_eq!(events_after[0].duration, 99.0);
    assert_eq!(events_after[1].id, e1.id);
    assert_eq!(events_after[1].duration, 1.0);
}

