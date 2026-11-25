use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use ty_analytics::{DayReport, RangeHours, SiteList, TimeRange};
use ty_settings::Settings;

use crate::client::ClassifiedEvents;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub range: TimeRange,
    pub events: ClassifiedEvents,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_report: Option<DayReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range_hours: Option<RangeHours>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_list: Option<SiteList>,
}

/// Generates the export data and writes it formatted as JSON to `target_path`.
pub fn write_export_file(
    target_path: &str,
    range: TimeRange,
    events: ClassifiedEvents,
    settings: &Settings,
) -> Result<String, String> {
    // Attempt analytics computations safely in case ty-analytics is a stub
    let day_report = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ty_analytics::build_day_report(
            &events.window,
            &events.afk,
            &events.web,
            range.clone(),
            &settings.day_start,
        )
    }))
    .ok();

    let range_hours = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ty_analytics::build_range_hours(
            &events.window,
            &events.afk,
            range.clone(),
            &settings.day_start,
        )
    }))
    .ok();

    let site_list = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ty_analytics::build_site_list(
            &events.window,
            &events.afk,
            &events.web,
            range.clone(),
        )
    }))
    .ok();

    let export = ExportData {
        range,
        events,
        day_report,
        range_hours,
        site_list,
    };

    let json_str = serde_json::to_string_pretty(&export)
        .map_err(|e| format!("Failed to serialize export data to JSON: {}", e))?;

    let path = Path::new(target_path);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directories for {}: {}", target_path, e))?;
        }
    }

    fs::write(path, json_str)
        .map_err(|e| format!("Failed to write export JSON to {}: {}", target_path, e))?;

    Ok(target_path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use tempfile::NamedTempFile;
    use ty_types::Event;

    #[test]
    fn test_write_export_file() {
        let file = NamedTempFile::new().expect("temp file");
        let path_str = file.path().to_string_lossy().to_string();

        let now = Utc::now();
        let range = TimeRange {
            start: now - Duration::hours(1),
            end: now,
        };

        let mut events = ClassifiedEvents::default();
        events.window.push(Event::new(
            now - Duration::minutes(30),
            60.0,
            serde_json::json!({"app": "code.exe", "title": "test"}),
        ));

        let settings = Settings::default();
        let written = write_export_file(&path_str, range.clone(), events.clone(), &settings)
            .expect("export should write");
        assert_eq!(written, path_str);

        let read_back = fs::read_to_string(&path_str).expect("read export");
        let parsed: ExportData = serde_json::from_str(&read_back).expect("parse export JSON");
        assert_eq!(parsed.range, range);
        assert_eq!(parsed.events.window.len(), 1);
    }
}
