use chrono::Duration;
use ty_types::Event;

/// Computes the recommended pulsetime for a given polling interval in seconds.
///
/// Formula: `max(poll * 1.5, poll + 1.0)`.
pub fn pulsetime_for_poll(poll_secs: f64) -> f64 {
    f64::max(poll_secs * 1.5, poll_secs + 1.0)
}

/// Attempts to merge a heartbeat into the last event.
///
/// Returns a merged event only if all of these hold:
/// 1. The `data` is exactly equal.
/// 2. The heartbeat timestamp is not earlier than `last.timestamp`.
/// 3. The heartbeat timestamp is within `last_end + pulsetime`.
///
/// The merged event keeps `last.timestamp` and `last.id`.
/// Its end is `max(last_end, heartbeat.timestamp + heartbeat.duration)`.
/// Its duration is `end - start`.
pub fn merge_heartbeat(last: &Event, heartbeat: &Event, pulsetime: f64) -> Option<Event> {
    if pulsetime < 0.0 {
        return None;
    }

    // 1. Data must be exactly equal
    if last.data != heartbeat.data {
        return None;
    }

    // 2. Heartbeat timestamp must not be earlier than last.timestamp
    if heartbeat.timestamp < last.timestamp {
        return None;
    }

    // Calculate last_end
    let last_dur_nanos = (last.duration * 1_000_000_000.0).round() as i64;
    let last_end = last.timestamp + Duration::nanoseconds(last_dur_nanos);

    // 3. Heartbeat timestamp must be within last_end + pulsetime
    let pulsetime_nanos = (pulsetime * 1_000_000_000.0).round() as i64;
    let max_allowed = last_end + Duration::nanoseconds(pulsetime_nanos);

    if heartbeat.timestamp > max_allowed {
        return None;
    }

    // Calculate heartbeat_end
    let hb_dur_nanos = (heartbeat.duration * 1_000_000_000.0).round() as i64;
    let hb_end = heartbeat.timestamp + Duration::nanoseconds(hb_dur_nanos);

    // End is max of last_end and hb_end
    let merged_end = std::cmp::max(last_end, hb_end);

    // Duration is end minus start
    let total_nanos = (merged_end - last.timestamp).num_nanoseconds().unwrap_or(0);
    let merged_duration = total_nanos as f64 / 1_000_000_000.0;

    Some(Event {
        id: last.id,
        timestamp: last.timestamp,
        duration: merged_duration,
        data: last.data.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_event(time_str: &str, duration: f64, data: serde_json::Value) -> Event {
        let naive = chrono::NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S").unwrap();
        Event {
            id: Some(1),
            timestamp: naive.and_utc(),
            duration,
            data,
        }
    }

    #[test]
    fn test_pulsetime_for_poll() {
        assert_eq!(pulsetime_for_poll(1.0), 2.0); // max(1.5, 2.0) = 2.0
        assert_eq!(pulsetime_for_poll(5.0), 7.5); // max(7.5, 6.0) = 7.5
        assert_eq!(pulsetime_for_poll(2.0), 3.0); // max(3.0, 3.0) = 3.0
        assert_eq!(pulsetime_for_poll(0.0), 1.0); // max(0.0, 1.0) = 1.0
    }

    #[test]
    fn test_equal_data_merges() {
        let last = make_event("2026-11-23 10:00:00", 0.0, json!({"app": "code.exe"}));
        let hb = make_event("2026-11-23 10:00:01", 0.0, json!({"app": "code.exe"}));

        let merged = merge_heartbeat(&last, &hb, 2.0).expect("should merge");
        assert_eq!(merged.timestamp, last.timestamp);
        assert_eq!(merged.id, last.id);
        assert_eq!(merged.duration, 1.0);
        assert_eq!(merged.data, json!({"app": "code.exe"}));
    }

    #[test]
    fn test_different_data_does_not_merge() {
        let last = make_event("2026-11-23 10:00:00", 1.0, json!({"app": "code.exe"}));
        let hb = make_event("2026-11-23 10:00:01", 0.0, json!({"app": "firefox.exe"}));

        let merged = merge_heartbeat(&last, &hb, 2.0);
        assert!(merged.is_none(), "different data must not merge");
    }

    #[test]
    fn test_timestamp_earlier_than_last_start_does_not_merge() {
        let last = make_event("2026-11-23 10:00:05", 2.0, json!({"app": "code.exe"}));
        let hb = make_event("2026-11-23 10:00:04", 0.0, json!({"app": "code.exe"}));

        let merged = merge_heartbeat(&last, &hb, 2.0);
        assert!(merged.is_none(), "earlier timestamp must not merge");
    }

    #[test]
    fn test_gap_beyond_pulsetime_does_not_merge() {
        let last = make_event("2026-11-23 10:00:00", 5.0, json!({"app": "code.exe"}));
        // last_end is 10:00:05. With pulsetime = 2.0, max allowed is 10:00:07.
        let hb = make_event("2026-11-23 10:00:08", 0.0, json!({"app": "code.exe"}));

        let merged = merge_heartbeat(&last, &hb, 2.0);
        assert!(merged.is_none(), "gap beyond pulsetime must not merge");

        // Edge case: exactly at 10:00:07 should merge
        let hb_boundary = make_event("2026-11-23 10:00:07", 0.0, json!({"app": "code.exe"}));
        assert!(merge_heartbeat(&last, &hb_boundary, 2.0).is_some());
    }

    #[test]
    fn test_end_max_rule_and_duration() {
        // Case 1: Heartbeat ends AFTER last_end
        let last = make_event("2026-11-23 10:00:00", 2.0, json!({"app": "code.exe"})); // ends at 10:00:02
        let hb = make_event("2026-11-23 10:00:01", 3.0, json!({"app": "code.exe"}));   // ends at 10:00:04

        let merged = merge_heartbeat(&last, &hb, 2.0).expect("should merge");
        assert_eq!(merged.timestamp, last.timestamp);
        assert_eq!(merged.duration, 4.0); // 10:00:04 - 10:00:00 = 4s

        // Case 2: Heartbeat ends BEFORE last_end (heartbeat with zero or small duration inside last span)
        let last2 = make_event("2026-11-23 10:00:00", 10.0, json!({"app": "code.exe"})); // ends at 10:00:10
        let hb2 = make_event("2026-11-23 10:00:05", 1.0, json!({"app": "code.exe"}));     // ends at 10:00:06

        let merged2 = merge_heartbeat(&last2, &hb2, 2.0).expect("should merge");
        assert_eq!(merged2.timestamp, last2.timestamp);
        assert_eq!(merged2.duration, 10.0); // max(10, 6) = 10s
    }

    #[test]
    fn test_negative_pulsetime() {
        let last = make_event("2026-11-23 10:00:00", 0.0, json!({"app": "code.exe"}));
        let hb = make_event("2026-11-23 10:00:01", 0.0, json!({"app": "code.exe"}));

        assert!(merge_heartbeat(&last, &hb, -1.0).is_none());
    }
}
