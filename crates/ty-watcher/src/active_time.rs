use chrono::{Duration, Local};
use std::time::Duration as StdDuration;
use ty_analytics::TimeRange;
use ty_types::Event;

/// Formats active seconds into a human-readable string for the tray menu.
///
/// Examples:
/// - `13320.0` -> `"Today: 3h 42m"`
/// - `900.0` -> `"Today: 15m"`
/// - `0.0` -> `"Today: 0m"`
pub fn format_active_time(seconds: f64) -> String {
    let total = seconds.round().max(0.0) as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;

    if hours > 0 {
        format!("Today: {}h {}m", hours, minutes)
    } else {
        format!("Today: {}m", minutes)
    }
}

/// Computes the 24-hour TimeRange corresponding to "today" in local time given a day_start string.
pub fn compute_today_range(day_start: &str) -> TimeRange {
    let day_start_time = ty_analytics::parse_day_start(day_start);
    let now_local = Local::now().naive_local();

    let start_date = if now_local.time() >= day_start_time {
        now_local.date()
    } else {
        now_local.date() - Duration::days(1)
    };

    let start_naive = start_date.and_time(day_start_time);
    let end_naive = start_naive + Duration::days(1);

    let start = ty_analytics::local_to_utc(start_naive);
    let end = ty_analytics::local_to_utc(end_naive);

    TimeRange { start, end }
}

/// Fetches events from an HTTP server bucket within the specified time range.
pub fn fetch_bucket_events(
    agent: &ureq::Agent,
    base_url: &str,
    bucket_id: &str,
    range: &TimeRange,
) -> Result<Vec<Event>, Box<dyn std::error::Error + Send + Sync>> {
    let start_str = range.start.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let end_str = range.end.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let url = format!(
        "{}/api/0/buckets/{}/events?start={}&end={}",
        base_url.trim_end_matches('/'),
        bucket_id,
        start_str,
        end_str
    );

    let resp = agent.get(&url).call()?;
    let events: Vec<Event> = resp.into_json()?;
    Ok(events)
}

/// Computes today's active seconds by querying the server's window and AFK buckets
/// and aggregating via `ty_analytics::build_day_report`.
pub fn compute_today_active_seconds(
    base_url: &str,
    window_bucket: &str,
    afk_bucket: &str,
    day_start: &str,
) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(StdDuration::from_secs(3))
        .timeout_read(StdDuration::from_secs(3))
        .timeout_write(StdDuration::from_secs(3))
        .build();

    let range = compute_today_range(day_start);

    let window_events = fetch_bucket_events(&agent, base_url, window_bucket, &range).unwrap_or_default();
    let afk_events = fetch_bucket_events(&agent, base_url, afk_bucket, &range).unwrap_or_default();

    let report = ty_analytics::build_day_report(&window_events, &afk_events, &[], range, day_start);
    Ok(report.active_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_active_time() {
        assert_eq!(format_active_time(0.0), "Today: 0m");
        assert_eq!(format_active_time(45.0), "Today: 0m"); // < 60s
        assert_eq!(format_active_time(60.0), "Today: 1m");
        assert_eq!(format_active_time(900.0), "Today: 15m");
        assert_eq!(format_active_time(3600.0), "Today: 1h 0m");
        assert_eq!(format_active_time(13320.0), "Today: 3h 42m"); // 3 * 3600 + 42 * 60 = 10800 + 2520 = 13320
        assert_eq!(format_active_time(-50.0), "Today: 0m");
    }

    #[test]
    fn test_compute_today_range_span() {
        let range = compute_today_range("00:00");
        let dur = range.end - range.start;
        assert_eq!(dur.num_hours(), 24);
    }
}
