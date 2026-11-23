use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ty_types::Event;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineBlock {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub kind: String, // "app" or "site"
    pub label: String,
    pub app: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopApp {
    pub app: String,
    pub secs: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopSite {
    pub domain: String,
    pub secs: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DayReport {
    pub date: String,
    pub active_secs: f64,
    pub timeline: Vec<TimelineBlock>,
    pub top_apps: Vec<TopApp>,
    pub top_sites: Vec<TopSite>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RangeHoursItem {
    pub date: String,
    pub active_secs: f64,
}

pub type RangeHours = Vec<RangeHoursItem>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageItem {
    pub url: String,
    pub title: String,
    pub secs: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteItem {
    pub domain: String,
    pub secs: f64,
    pub pages: Vec<PageItem>,
}

pub type SiteList = Vec<SiteItem>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hit {
    pub kind: String, // "site" or "page"
    pub domain: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub score: f64,
    pub secs: f64,
}

#[derive(Default)]
pub struct SearchIndex;

/// Builds a day report aggregating active time, timeline blocks, top apps, and top sites.
pub fn build_day_report(
    _window_events: &[Event],
    _afk_events: &[Event],
    _web_events: &[Event],
    _range: TimeRange,
    _day_start: &str,
) -> DayReport {
    todo!("ty_analytics::build_day_report not yet implemented")
}

/// Builds hours active per day over a given time range.
pub fn build_range_hours(
    _window_events: &[Event],
    _afk_events: &[Event],
    _range: TimeRange,
    _day_start: &str,
) -> RangeHours {
    todo!("ty_analytics::build_range_hours not yet implemented")
}

/// Builds the site list with sub-pages, sorted by active seconds descending.
pub fn build_site_list(
    _window_events: &[Event],
    _afk_events: &[Event],
    _web_events: &[Event],
    _range: TimeRange,
) -> SiteList {
    todo!("ty_analytics::build_site_list not yet implemented")
}

/// Builds an in-memory fuzzy search index from a site list.
pub fn build_search_index(_site_list: &SiteList) -> SearchIndex {
    todo!("ty_analytics::build_search_index not yet implemented")
}

/// Executes a fuzzy search against the search index.
pub fn search(
    _index: &SearchIndex,
    _query: &str,
    _mode: &str,
    _site: Option<&str>,
) -> Vec<Hit> {
    todo!("ty_analytics::search not yet implemented")
}
