use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use ty_analytics::TimeRange;

/// Parses a "HH:MM" string into hours and minutes, defaulting to (0, 0) if invalid.
pub fn parse_day_start(day_start: &str) -> (u32, u32) {
    let parts: Vec<&str> = day_start.trim().split(':').collect();
    if parts.len() == 2 {
        if let (Ok(h), Ok(m)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
            if h < 24 && m < 60 {
                return (h, m);
            }
        }
    }
    (0, 0)
}

/// Converts a NaiveDateTime in local time to a UTC DateTime, safely handling DST ambiguities.
pub fn local_naive_to_utc(naive: NaiveDateTime) -> DateTime<Utc> {
    match Local.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => dt.with_timezone(&Utc),
        chrono::LocalResult::Ambiguous(dt1, _) => dt1.with_timezone(&Utc),
        chrono::LocalResult::None => {
            // Gap during spring-forward transition: shift forward by 1 hour
            match Local.from_local_datetime(&(naive + Duration::hours(1))) {
                chrono::LocalResult::Single(dt) => dt.with_timezone(&Utc),
                chrono::LocalResult::Ambiguous(dt1, _) => dt1.with_timezone(&Utc),
                chrono::LocalResult::None => DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc),
            }
        }
    }
}

/// Computes a full 24-hour day range for a given date string and day_start in local time.
/// Supports "YYYY-MM-DD" as well as RFC3339 timestamps.
pub fn parse_day_range(date_str: &str, day_start_str: &str) -> Result<TimeRange, String> {
    let (h, m) = parse_day_start(day_start_str);

    let naive_date = if let Ok(date) = NaiveDate::parse_from_str(date_str.trim(), "%Y-%m-%d") {
        date
    } else if let Ok(dt) = DateTime::parse_from_rfc3339(date_str.trim()) {
        dt.with_timezone(&Local).date_naive()
    } else {
        return Err(format!(
            "Invalid date '{}'. Expected 'YYYY-MM-DD' or RFC 3339 timestamp.",
            date_str
        ));
    };

    let start_naive = naive_date
        .and_hms_opt(h, m, 0)
        .ok_or_else(|| format!("Invalid time {}:{} for date {}", h, m, naive_date))?;
    let end_naive = (naive_date + Duration::days(1))
        .and_hms_opt(h, m, 0)
        .ok_or_else(|| format!("Invalid time {}:{} for date {}", h, m, naive_date))?;

    let start = local_naive_to_utc(start_naive);
    let end = local_naive_to_utc(end_naive);

    Ok(TimeRange { start, end })
}

/// Parses `from` and `to` strings into a TimeRange.
/// Both strings can be either "YYYY-MM-DD" or RFC 3339 timestamps.
/// For date strings, `from` starts at `day_start` on that day, and `to` ends at `day_start` on the next day.
pub fn parse_range(from_str: &str, to_str: &str, day_start_str: &str) -> Result<TimeRange, String> {
    let (h, m) = parse_day_start(day_start_str);

    let start = if let Ok(dt) = DateTime::parse_from_rfc3339(from_str.trim()) {
        dt.with_timezone(&Utc)
    } else if let Ok(date) = NaiveDate::parse_from_str(from_str.trim(), "%Y-%m-%d") {
        let naive = date
            .and_hms_opt(h, m, 0)
            .ok_or_else(|| format!("Invalid start time {}:{} for date {}", h, m, date))?;
        local_naive_to_utc(naive)
    } else {
        return Err(format!(
            "Invalid 'from' timestamp '{}'. Expected 'YYYY-MM-DD' or RFC 3339.",
            from_str
        ));
    };

    let end = if let Ok(dt) = DateTime::parse_from_rfc3339(to_str.trim()) {
        dt.with_timezone(&Utc)
    } else if let Ok(date) = NaiveDate::parse_from_str(to_str.trim(), "%Y-%m-%d") {
        let naive = (date + Duration::days(1))
            .and_hms_opt(h, m, 0)
            .ok_or_else(|| format!("Invalid end time {}:{} for date {}", h, m, date))?;
        local_naive_to_utc(naive)
    } else {
        return Err(format!(
            "Invalid 'to' timestamp '{}'. Expected 'YYYY-MM-DD' or RFC 3339.",
            to_str
        ));
    };

    if start > end {
        return Err(format!(
            "Invalid range: 'from' ({}) is after 'to' ({}).",
            start.to_rfc3339(),
            end.to_rfc3339()
        ));
    }

    Ok(TimeRange { start, end })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_day_start() {
        assert_eq!(parse_day_start("00:00"), (0, 0));
        assert_eq!(parse_day_start("04:30"), (4, 30));
        assert_eq!(parse_day_start("23:59"), (23, 59));
        assert_eq!(parse_day_start("24:00"), (0, 0)); // invalid hour fallback
        assert_eq!(parse_day_start("invalid"), (0, 0));
        assert_eq!(parse_day_start(""), (0, 0));
    }

    #[test]
    fn test_parse_day_range_date() {
        let range = parse_day_range("2026-11-23", "00:00").expect("valid date range");
        let duration = range.end - range.start;
        // Exactly 24 hours (unless DST change occurred on that exact day)
        assert!(duration.num_hours() >= 23 && duration.num_hours() <= 25);
    }

    #[test]
    fn test_parse_day_range_rfc3339() {
        let range = parse_day_range("2026-11-23T14:30:00Z", "00:00").expect("valid rfc3339");
        let duration = range.end - range.start;
        assert!(duration.num_hours() >= 23 && duration.num_hours() <= 25);
    }

    #[test]
    fn test_parse_day_range_invalid() {
        assert!(parse_day_range("not-a-date", "00:00").is_err());
    }

    #[test]
    fn test_parse_range_dates() {
        let range = parse_range("2026-11-20", "2026-11-23", "04:00").expect("valid range");
        assert!(range.start < range.end);
        // 4 days total (Nov 20, 21, 22, 23) = ~96 hours
        let hours = (range.end - range.start).num_hours();
        assert!(hours >= 95 && hours <= 97);
    }

    #[test]
    fn test_parse_range_rfc3339() {
        let range = parse_range(
            "2026-11-20T00:00:00Z",
            "2026-11-23T12:00:00Z",
            "00:00",
        )
        .expect("valid range");
        assert_eq!(range.start, DateTime::parse_from_rfc3339("2026-11-20T00:00:00Z").unwrap().with_timezone(&Utc));
        assert_eq!(range.end, DateTime::parse_from_rfc3339("2026-11-23T12:00:00Z").unwrap().with_timezone(&Utc));
    }

    #[test]
    fn test_parse_range_inverted_errors() {
        let res = parse_range(
            "2026-11-25T00:00:00Z",
            "2026-11-20T00:00:00Z",
            "00:00",
        );
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("after 'to'"));
    }
}
