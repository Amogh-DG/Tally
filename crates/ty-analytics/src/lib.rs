use std::collections::HashMap;
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveTime, TimeZone, Utc};
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
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    pub score: f64,
    pub secs: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchItem {
    pub kind: String,
    pub domain: String,
    pub url: Option<String>,
    pub title: Option<String>,
    pub match_text: String,
    pub secs: f64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SearchIndex {
    pub items: Vec<SearchItem>,
}

/// Checks whether an application is a supported browser (firefox.exe, chrome.exe).
pub fn is_browser(app: &str) -> bool {
    let lower = app.to_lowercase();
    lower == "firefox.exe" || lower == "chrome.exe"
}

/// Extracts the registrable domain via the public suffix list (e.g. m.youtube.com -> youtube.com).
pub fn extract_domain(url_or_host: &str) -> String {
    let trimmed = url_or_host.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let host = if let Ok(parsed) = url::Url::parse(trimmed) {
        parsed.host_str().map(|h| h.to_lowercase()).unwrap_or_default()
    } else if let Ok(parsed) = url::Url::parse(&format!("http://{}", trimmed)) {
        parsed.host_str().map(|h| h.to_lowercase()).unwrap_or_default()
    } else {
        let without_proto = if let Some(pos) = trimmed.find("://") {
            &trimmed[pos + 3..]
        } else {
            trimmed
        };
        let host_part = without_proto
            .split(&['/', ':', '?', '#'][..])
            .next()
            .unwrap_or(trimmed);
        host_part.to_lowercase()
    };

    let host = host.trim_end_matches('.');
    if host.is_empty() {
        return String::new();
    }

    if host.parse::<std::net::IpAddr>().is_ok() {
        return host.to_string();
    }

    if let Some(domain) = psl::domain(host.as_bytes()) {
        if let Ok(s) = std::str::from_utf8(domain.as_bytes()) {
            return s.to_string();
        }
    }

    if let Some(stripped) = host.strip_prefix("www.") {
        return stripped.to_string();
    }

    host.to_string()
}

/// Parses a day_start setting (e.g. "00:00" or "04:00") into a NaiveTime.
pub fn parse_day_start(day_start: &str) -> NaiveTime {
    if let Ok(time) = NaiveTime::parse_from_str(day_start, "%H:%M") {
        return time;
    }
    if let Ok(time) = NaiveTime::parse_from_str(day_start, "%H:%M:%S") {
        return time;
    }
    let parts: Vec<&str> = day_start.split(':').collect();
    if parts.len() >= 2 {
        if let (Ok(h), Ok(m)) = (parts[0].trim().parse::<u32>(), parts[1].trim().parse::<u32>()) {
            if let Some(t) = NaiveTime::from_hms_opt(h, m, 0) {
                return t;
            }
        }
    }
    NaiveTime::from_hms_opt(0, 0, 0).unwrap()
}

/// Determines the logical local date for a given UTC timestamp and day_start time.
pub fn date_for_time(utc_dt: DateTime<Utc>, day_start_time: NaiveTime) -> NaiveDate {
    let local_dt = utc_dt.with_timezone(&Local);
    if local_dt.time() >= day_start_time {
        local_dt.date_naive()
    } else {
        local_dt.date_naive() - Duration::days(1)
    }
}

/// Converts a local naive date and time to UTC DateTime<Utc>.
pub fn local_to_utc(naive: chrono::NaiveDateTime) -> DateTime<Utc> {
    match Local.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => dt.with_timezone(&Utc),
        chrono::LocalResult::Ambiguous(dt1, _) => dt1.with_timezone(&Utc),
        chrono::LocalResult::None => {
            let shifted = naive + Duration::hours(1);
            Local.from_local_datetime(&shifted)
                .earliest()
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|| naive.and_utc())
        }
    }
}

fn merge_intervals(mut intervals: Vec<(DateTime<Utc>, DateTime<Utc>)>) -> Vec<(DateTime<Utc>, DateTime<Utc>)> {
    if intervals.is_empty() {
        return Vec::new();
    }
    intervals.sort_by_key(|i| i.0);
    let mut merged = Vec::new();
    let mut current = intervals[0];
    for next in intervals.into_iter().skip(1) {
        if next.0 <= current.1 {
            if next.1 > current.1 {
                current.1 = next.1;
            }
        } else {
            merged.push(current);
            current = next;
        }
    }
    merged.push(current);
    merged
}

fn get_not_afk_intervals(afk_events: &[Event]) -> Vec<(DateTime<Utc>, DateTime<Utc>)> {
    let mut raw = Vec::new();
    for ev in afk_events {
        if ev.duration <= 0.0 {
            continue;
        }
        let status = ev.data.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if status.eq_ignore_ascii_case("not-afk") || status.eq_ignore_ascii_case("not_afk") {
            let start = ev.timestamp;
            let end = ev.end_time();
            if start < end {
                raw.push((start, end));
            }
        }
    }
    merge_intervals(raw)
}

#[derive(Debug, Clone)]
struct ActiveWindowChunk {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    app: String,
    title: String,
}

fn get_active_window_chunks(
    window_events: &[Event],
    afk_events: &[Event],
    range: &TimeRange,
) -> Vec<ActiveWindowChunk> {
    if range.start >= range.end {
        return Vec::new();
    }
    let not_afk = get_not_afk_intervals(afk_events);
    if not_afk.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    for wev in window_events {
        if wev.duration <= 0.0 {
            continue;
        }
        let w_start = wev.timestamp;
        let w_end = wev.end_time();
        if w_start >= w_end {
            continue;
        }
        let app = wev.data.get("app").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
        let title = wev.data.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();

        for &(a_start, a_end) in &not_afk {
            let start = w_start.max(a_start).max(range.start);
            let end = w_end.min(a_end).min(range.end);
            if start < end {
                chunks.push(ActiveWindowChunk {
                    start,
                    end,
                    app: app.clone(),
                    title: title.clone(),
                });
            }
        }
    }

    chunks.sort_by_key(|c| c.start);
    chunks
}

#[derive(Debug, Clone)]
struct ActiveWebChunk {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    _app: String,
    domain: String,
    url: String,
    title: String,
}

fn get_active_web_chunks(
    browser_chunks: &[ActiveWindowChunk],
    web_events: &[Event],
) -> Vec<ActiveWebChunk> {
    let mut chunks = Vec::new();

    for wev in web_events {
        if wev.duration <= 0.0 {
            continue;
        }
        let web_start = wev.timestamp;
        let web_end = wev.end_time();
        if web_start >= web_end {
            continue;
        }

        let url = wev.data.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let title = wev.data.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let domain = extract_domain(&url);

        for bc in browser_chunks {
            let start = web_start.max(bc.start);
            let end = web_end.min(bc.end);
            if start < end {
                let effective_title = if !title.is_empty() {
                    title.clone()
                } else if !bc.title.is_empty() {
                    bc.title.clone()
                } else {
                    domain.clone()
                };
                chunks.push(ActiveWebChunk {
                    start,
                    end,
                    _app: bc.app.clone(),
                    domain: domain.clone(),
                    url: url.clone(),
                    title: effective_title,
                });
            }
        }
    }

    chunks.sort_by_key(|c| c.start);
    chunks
}

fn merge_timeline_blocks(blocks: Vec<TimelineBlock>) -> Vec<TimelineBlock> {
    if blocks.is_empty() {
        return Vec::new();
    }
    let mut merged = Vec::new();
    let mut current = blocks[0].clone();

    for next in blocks.into_iter().skip(1) {
        if current.kind == next.kind
            && current.app == next.app
            && current.label == next.label
            && current.end == next.start
        {
            current.end = next.end;
        } else {
            merged.push(current);
            current = next;
        }
    }
    merged.push(current);
    merged
}

/// Builds a day report aggregating active time, timeline blocks, top apps, and top sites.
pub fn build_day_report(
    window_events: &[Event],
    afk_events: &[Event],
    web_events: &[Event],
    range: TimeRange,
    day_start: &str,
) -> DayReport {
    let day_start_time = parse_day_start(day_start);
    let date_str = date_for_time(range.start, day_start_time).format("%Y-%m-%d").to_string();

    let active_chunks = get_active_window_chunks(window_events, afk_events, &range);

    let active_secs: f64 = active_chunks
        .iter()
        .map(|c| (c.end - c.start).num_milliseconds() as f64 / 1000.0)
        .sum();

    // Top apps
    let mut app_map: HashMap<String, f64> = HashMap::new();
    for c in &active_chunks {
        let secs = (c.end - c.start).num_milliseconds() as f64 / 1000.0;
        *app_map.entry(c.app.clone()).or_insert(0.0) += secs;
    }
    let mut top_apps: Vec<TopApp> = app_map
        .into_iter()
        .map(|(app, secs)| TopApp { app, secs })
        .collect();
    top_apps.sort_by(|a, b| {
        b.secs.partial_cmp(&a.secs)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.app.cmp(&b.app))
    });

    let mut raw_timeline: Vec<TimelineBlock> = Vec::new();
    let mut top_sites: Vec<TopSite>;

    if web_events.is_empty() {
        // Title-only mode: browser time grouped by app, pages are window titles
        let mut site_map: HashMap<String, f64> = HashMap::new();
        for c in &active_chunks {
            let secs = (c.end - c.start).num_milliseconds() as f64 / 1000.0;
            if is_browser(&c.app) {
                *site_map.entry(c.app.clone()).or_insert(0.0) += secs;
                let label = if !c.title.is_empty() {
                    c.title.clone()
                } else {
                    c.app.clone()
                };
                raw_timeline.push(TimelineBlock {
                    start: c.start,
                    end: c.end,
                    kind: "site".to_string(),
                    label,
                    app: c.app.clone(),
                });
            } else {
                let label = if !c.title.is_empty() {
                    c.title.clone()
                } else {
                    c.app.clone()
                };
                raw_timeline.push(TimelineBlock {
                    start: c.start,
                    end: c.end,
                    kind: "app".to_string(),
                    label,
                    app: c.app.clone(),
                });
            }
        }

        top_sites = site_map
            .into_iter()
            .map(|(domain, secs)| TopSite { domain, secs })
            .collect();
    } else {
        // Full mode with web events
        let browser_chunks: Vec<ActiveWindowChunk> = active_chunks
            .iter()
            .filter(|c| is_browser(&c.app))
            .cloned()
            .collect();
        let web_chunks = get_active_web_chunks(&browser_chunks, web_events);

        let mut site_map: HashMap<String, f64> = HashMap::new();
        for wc in &web_chunks {
            let secs = (wc.end - wc.start).num_milliseconds() as f64 / 1000.0;
            *site_map.entry(wc.domain.clone()).or_insert(0.0) += secs;
        }

        for c in &active_chunks {
            if is_browser(&c.app) {
                // Find all web visits overlapping this browser window chunk
                let overlapping: Vec<&ActiveWebChunk> = web_chunks
                    .iter()
                    .filter(|wc| wc.start < c.end && wc.end > c.start)
                    .collect();

                if overlapping.is_empty() {
                    let label = if !c.title.is_empty() {
                        c.title.clone()
                    } else {
                        c.app.clone()
                    };
                    raw_timeline.push(TimelineBlock {
                        start: c.start,
                        end: c.end,
                        kind: "app".to_string(),
                        label,
                        app: c.app.clone(),
                    });
                } else {
                    let mut cur = c.start;
                    for wc in overlapping {
                        let ws = wc.start.max(cur);
                        let we = wc.end.min(c.end);
                        if ws > cur {
                            let label = if !c.title.is_empty() {
                                c.title.clone()
                            } else {
                                c.app.clone()
                            };
                            raw_timeline.push(TimelineBlock {
                                start: cur,
                                end: ws,
                                kind: "app".to_string(),
                                label,
                                app: c.app.clone(),
                            });
                        }
                        if ws < we {
                            raw_timeline.push(TimelineBlock {
                                start: ws,
                                end: we,
                                kind: "site".to_string(),
                                label: wc.domain.clone(),
                                app: c.app.clone(),
                            });
                            cur = we;
                        }
                    }
                    if cur < c.end {
                        let label = if !c.title.is_empty() {
                            c.title.clone()
                        } else {
                            c.app.clone()
                        };
                        raw_timeline.push(TimelineBlock {
                            start: cur,
                            end: c.end,
                            kind: "app".to_string(),
                            label,
                            app: c.app.clone(),
                        });
                    }
                }
            } else {
                let label = if !c.title.is_empty() {
                    c.title.clone()
                } else {
                    c.app.clone()
                };
                raw_timeline.push(TimelineBlock {
                    start: c.start,
                    end: c.end,
                    kind: "app".to_string(),
                    label,
                    app: c.app.clone(),
                });
            }
        }

        top_sites = site_map
            .into_iter()
            .map(|(domain, secs)| TopSite { domain, secs })
            .collect();
    }

    top_sites.sort_by(|a, b| {
        b.secs.partial_cmp(&a.secs)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.domain.cmp(&b.domain))
    });

    raw_timeline.sort_by_key(|t| t.start);
    let timeline = merge_timeline_blocks(raw_timeline);

    DayReport {
        date: date_str,
        active_secs,
        timeline,
        top_apps,
        top_sites,
    }
}

/// Builds hours active per day over a given time range.
pub fn build_range_hours(
    window_events: &[Event],
    afk_events: &[Event],
    range: TimeRange,
    day_start: &str,
) -> RangeHours {
    if range.start >= range.end {
        return Vec::new();
    }

    let day_start_time = parse_day_start(day_start);
    let start_date = date_for_time(range.start, day_start_time);
    let last_instant = range.end - Duration::milliseconds(1);
    let end_date = date_for_time(last_instant, day_start_time);

    let mut result = Vec::new();
    let mut current_date = start_date;

    while current_date <= end_date {
        let day_start_utc = local_to_utc(current_date.and_time(day_start_time));
        let next_date = current_date + Duration::days(1);
        let day_end_utc = local_to_utc(next_date.and_time(day_start_time));

        let clamped_start = day_start_utc.max(range.start);
        let clamped_end = day_end_utc.min(range.end);

        let active_secs = if clamped_start < clamped_end {
            let day_range = TimeRange {
                start: clamped_start,
                end: clamped_end,
            };
            let chunks = get_active_window_chunks(window_events, afk_events, &day_range);
            chunks
                .iter()
                .map(|c| (c.end - c.start).num_milliseconds() as f64 / 1000.0)
                .sum()
        } else {
            0.0
        };

        result.push(RangeHoursItem {
            date: current_date.format("%Y-%m-%d").to_string(),
            active_secs,
        });

        current_date = next_date;
    }

    result
}

/// Builds the site list with sub-pages, sorted by active seconds descending.
pub fn build_site_list(
    window_events: &[Event],
    afk_events: &[Event],
    web_events: &[Event],
    range: TimeRange,
) -> SiteList {
    let active_chunks = get_active_window_chunks(window_events, afk_events, &range);
    let browser_chunks: Vec<ActiveWindowChunk> = active_chunks
        .into_iter()
        .filter(|c| is_browser(&c.app))
        .collect();

    if browser_chunks.is_empty() {
        return Vec::new();
    }

    if web_events.is_empty() {
        // Title-only mode: browser time grouped by app, pages are window titles
        let mut app_pages: HashMap<String, HashMap<String, f64>> = HashMap::new();
        for bc in &browser_chunks {
            let secs = (bc.end - bc.start).num_milliseconds() as f64 / 1000.0;
            let title = if !bc.title.is_empty() {
                bc.title.clone()
            } else {
                bc.app.clone()
            };
            let pages = app_pages.entry(bc.app.clone()).or_default();
            *pages.entry(title).or_insert(0.0) += secs;
        }

        let mut sites = Vec::new();
        for (domain, pages_map) in app_pages {
            let mut pages = Vec::new();
            let mut total_secs = 0.0;
            for (title, secs) in pages_map {
                total_secs += secs;
                pages.push(PageItem {
                    url: title.clone(),
                    title,
                    secs,
                });
            }
            pages.sort_by(|a, b| {
                b.secs.partial_cmp(&a.secs)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.title.cmp(&b.title))
            });
            sites.push(SiteItem {
                domain,
                secs: total_secs,
                pages,
            });
        }

        sites.sort_by(|a, b| {
            b.secs.partial_cmp(&a.secs)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.domain.cmp(&b.domain))
        });
        return sites;
    }

    // Full mode with web events
    let web_chunks = get_active_web_chunks(&browser_chunks, web_events);
    let mut domain_pages: HashMap<String, HashMap<String, (String, f64)>> = HashMap::new();

    for wc in &web_chunks {
        let secs = (wc.end - wc.start).num_milliseconds() as f64 / 1000.0;
        let pages = domain_pages.entry(wc.domain.clone()).or_default();
        let entry = pages.entry(wc.url.clone()).or_insert_with(|| (wc.title.clone(), 0.0));
        entry.1 += secs;
        if !wc.title.is_empty() && (entry.0.is_empty() || entry.0 == wc.domain) {
            entry.0 = wc.title.clone();
        }
    }

    let mut sites = Vec::new();
    for (domain, pages_map) in domain_pages {
        let mut pages = Vec::new();
        let mut total_secs = 0.0;
        for (url, (title, secs)) in pages_map {
            total_secs += secs;
            pages.push(PageItem { url, title, secs });
        }
        pages.sort_by(|a, b| {
            b.secs.partial_cmp(&a.secs)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.url.cmp(&b.url))
        });
        sites.push(SiteItem {
            domain,
            secs: total_secs,
            pages,
        });
    }

    sites.sort_by(|a, b| {
        b.secs.partial_cmp(&a.secs)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.domain.cmp(&b.domain))
    });

    sites
}

/// Builds an in-memory fuzzy search index from a site list.
pub fn build_search_index(site_list: &SiteList) -> SearchIndex {
    let mut items = Vec::new();
    for site in site_list {
        items.push(SearchItem {
            kind: "site".to_string(),
            domain: site.domain.clone(),
            url: None,
            title: None,
            match_text: site.domain.clone(),
            secs: site.secs,
        });
        for page in &site.pages {
            items.push(SearchItem {
                kind: "page".to_string(),
                domain: site.domain.clone(),
                url: Some(page.url.clone()),
                title: Some(page.title.clone()),
                match_text: page.title.clone(),
                secs: page.secs,
            });
        }
    }
    SearchIndex { items }
}

/// Executes a fuzzy search against the search index.
pub fn search(
    index: &SearchIndex,
    query: &str,
    mode: &str,
    site: Option<&str>,
) -> Vec<Hit> {
    let q = query.trim();
    if q.is_empty() {
        return Vec::new();
    }

    let pattern = nucleo::pattern::Pattern::parse(
        q,
        nucleo::pattern::CaseMatching::Ignore,
        nucleo::pattern::Normalization::Smart,
    );
    let mut matcher = nucleo::Matcher::new(nucleo::Config::DEFAULT);
    let mut buf = Vec::new();

    let max_score = pattern
        .score(nucleo::Utf32Str::new(q, &mut buf), &mut matcher)
        .unwrap_or(1)
        .max(1);

    let mut hits = Vec::new();

    for item in &index.items {
        match mode {
            "sites" => {
                if item.kind != "site" {
                    continue;
                }
            }
            "titles" => {
                if item.kind != "page" {
                    continue;
                }
            }
            "both" => {}
            _ => {}
        }

        if let Some(scope_site) = site {
            if !item.domain.eq_ignore_ascii_case(scope_site) {
                continue;
            }
        }

        let score_opt = pattern
            .score(nucleo::Utf32Str::new(&item.match_text, &mut buf), &mut matcher)
            .or_else(|| {
                if let Some(ref u) = item.url {
                    pattern.score(nucleo::Utf32Str::new(u, &mut buf), &mut matcher)
                } else {
                    None
                }
            });

        if let Some(raw_score) = score_opt {
            let normalized = (raw_score as f64 / max_score as f64).min(1.0);
            let score = (normalized * 100.0).round() / 100.0;
            hits.push(Hit {
                kind: item.kind.clone(),
                domain: item.domain.clone(),
                url: item.url.clone(),
                title: item.title.clone(),
                score,
                secs: item.secs,
            });
        }
    }

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.secs.partial_cmp(&a.secs).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| a.domain.cmp(&b.domain))
    });

    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc_dt(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    fn make_event(ts: &str, duration: f64, data: serde_json::Value) -> Event {
        Event::new(utc_dt(ts), duration, data)
    }

    #[test]
    fn test_deserialize_fixtures() {
        let day_report_json = include_str!("../../../docs/fixtures/day_report.json");
        let day_report: DayReport = serde_json::from_str(day_report_json).unwrap();
        assert_eq!(day_report.date, "2026-11-23");
        assert_eq!(day_report.active_secs, 28800.0);
        assert_eq!(day_report.timeline.len(), 3);
        assert_eq!(day_report.top_apps.len(), 2);
        assert_eq!(day_report.top_sites.len(), 2);

        let range_hours_json = include_str!("../../../docs/fixtures/range_hours.json");
        let range_hours: RangeHours = serde_json::from_str(range_hours_json).unwrap();
        assert_eq!(range_hours.len(), 7);
        assert_eq!(range_hours[6].date, "2026-11-23");
        assert_eq!(range_hours[6].active_secs, 28800.0);

        let site_list_json = include_str!("../../../docs/fixtures/site_list.json");
        let site_list: SiteList = serde_json::from_str(site_list_json).unwrap();
        assert_eq!(site_list.len(), 2);
        assert_eq!(site_list[0].domain, "github.com");
        assert_eq!(site_list[0].secs, 5400.0);
        assert_eq!(site_list[0].pages.len(), 2);

        let search_hits_json = include_str!("../../../docs/fixtures/search_hits.json");
        let search_hits: Vec<Hit> = serde_json::from_str(search_hits_json).unwrap();
        assert_eq!(search_hits.len(), 3);
        assert_eq!(search_hits[0].kind, "site");
        assert_eq!(search_hits[0].domain, "github.com");
        assert_eq!(search_hits[0].url, None);
        assert_eq!(search_hits[1].kind, "page");
        assert_eq!(search_hits[1].domain, "github.com");
        assert_eq!(search_hits[1].url, Some("https://github.com/rust-lang/rust".to_string()));
    }

    #[test]
    fn test_domain_merging() {
        assert_eq!(extract_domain("https://m.youtube.com/watch?v=123"), "youtube.com");
        assert_eq!(extract_domain("https://www.youtube.com/feed/subscriptions"), "youtube.com");
        assert_eq!(extract_domain("https://youtube.com/"), "youtube.com");
        assert_eq!(extract_domain("https://docs.rs/serde/latest/serde/"), "docs.rs");
        assert_eq!(extract_domain("https://sub.docs.rs/"), "docs.rs");
        assert_eq!(extract_domain("http://localhost:5600"), "localhost");
        assert_eq!(extract_domain("http://127.0.0.1:8080/foo"), "127.0.0.1");

        let range = TimeRange {
            start: utc_dt("2026-11-23T10:00:00Z"),
            end: utc_dt("2026-11-23T11:00:00Z"),
        };
        let window_events = vec![
            make_event("2026-11-23T10:00:00Z", 3600.0, serde_json::json!({
                "app": "firefox.exe",
                "title": "YouTube"
            })),
        ];
        let afk_events = vec![
            make_event("2026-11-23T10:00:00Z", 3600.0, serde_json::json!({
                "status": "not-afk"
            })),
        ];
        let web_events = vec![
            make_event("2026-11-23T10:00:00Z", 600.0, serde_json::json!({
                "url": "https://m.youtube.com/watch?v=1",
                "title": "Video 1"
            })),
            make_event("2026-11-23T10:10:00Z", 1200.0, serde_json::json!({
                "url": "https://www.youtube.com/watch?v=2",
                "title": "Video 2"
            })),
            make_event("2026-11-23T10:30:00Z", 600.0, serde_json::json!({
                "url": "https://youtube.com/watch?v=3",
                "title": "Video 3"
            })),
        ];

        let site_list = build_site_list(&window_events, &afk_events, &web_events, range);
        assert_eq!(site_list.len(), 1);
        assert_eq!(site_list[0].domain, "youtube.com");
        assert_eq!(site_list[0].secs, 2400.0);
        assert_eq!(site_list[0].pages.len(), 3);
    }

    #[test]
    fn test_afk_intersection() {
        let range = TimeRange {
            start: utc_dt("2026-11-23T10:00:00Z"),
            end: utc_dt("2026-11-23T11:00:00Z"),
        };
        let window_events = vec![
            make_event("2026-11-23T10:00:00Z", 3600.0, serde_json::json!({
                "app": "code.exe",
                "title": "main.rs"
            })),
        ];
        // 10:00-10:20 not-afk (1200s), 10:20-10:45 afk (1500s), 10:45-11:15 not-afk (1800s)
        let afk_events = vec![
            make_event("2026-11-23T10:00:00Z", 1200.0, serde_json::json!({ "status": "not-afk" })),
            make_event("2026-11-23T10:20:00Z", 1500.0, serde_json::json!({ "status": "afk" })),
            make_event("2026-11-23T10:45:00Z", 1800.0, serde_json::json!({ "status": "not-afk" })),
        ];

        let report = build_day_report(&window_events, &afk_events, &[], range, "00:00");
        // Active time should be [10:00, 10:20] = 1200s + [10:45, 11:00] = 900s -> 2100s
        assert_eq!(report.active_secs, 2100.0);
        assert_eq!(report.top_apps.len(), 1);
        assert_eq!(report.top_apps[0].app, "code.exe");
        assert_eq!(report.top_apps[0].secs, 2100.0);
        assert_eq!(report.timeline.len(), 2);
        assert_eq!(report.timeline[0].start, utc_dt("2026-11-23T10:00:00Z"));
        assert_eq!(report.timeline[0].end, utc_dt("2026-11-23T10:20:00Z"));
        assert_eq!(report.timeline[1].start, utc_dt("2026-11-23T10:45:00Z"));
        assert_eq!(report.timeline[1].end, utc_dt("2026-11-23T11:00:00Z"));
    }

    #[test]
    fn test_browser_focus_intersection() {
        let range = TimeRange {
            start: utc_dt("2026-11-23T10:00:00Z"),
            end: utc_dt("2026-11-23T13:00:00Z"),
        };
        let window_events = vec![
            make_event("2026-11-23T10:00:00Z", 3600.0, serde_json::json!({
                "app": "code.exe",
                "title": "main.rs"
            })),
            make_event("2026-11-23T11:00:00Z", 3600.0, serde_json::json!({
                "app": "firefox.exe",
                "title": "Mozilla Firefox"
            })),
            make_event("2026-11-23T12:00:00Z", 3600.0, serde_json::json!({
                "app": "code.exe",
                "title": "main.rs"
            })),
        ];
        let afk_events = vec![
            make_event("2026-11-23T10:00:00Z", 10800.0, serde_json::json!({
                "status": "not-afk"
            })),
        ];
        // Web event active from 10:30 to 12:30 (spanning code.exe, firefox.exe, code.exe)
        let web_events = vec![
            make_event("2026-11-23T10:30:00Z", 7200.0, serde_json::json!({
                "url": "https://github.com/rust-lang/rust",
                "title": "rust-lang/rust"
            })),
        ];

        let site_list = build_site_list(&window_events, &afk_events, &web_events, range.clone());
        assert_eq!(site_list.len(), 1);
        assert_eq!(site_list[0].domain, "github.com");
        // Page time must ONLY be the intersection with firefox.exe (11:00 to 12:00 = 3600s)
        assert_eq!(site_list[0].secs, 3600.0);
        assert_eq!(site_list[0].pages[0].secs, 3600.0);

        let report = build_day_report(&window_events, &afk_events, &web_events, range, "00:00");
        assert_eq!(report.active_secs, 10800.0);
        assert_eq!(report.top_sites.len(), 1);
        assert_eq!(report.top_sites[0].domain, "github.com");
        assert_eq!(report.top_sites[0].secs, 3600.0);
    }

    #[test]
    fn test_title_only_mode() {
        let range = TimeRange {
            start: utc_dt("2026-11-23T09:00:00Z"),
            end: utc_dt("2026-11-23T12:00:00Z"),
        };
        let window_events = vec![
            make_event("2026-11-23T09:00:00Z", 3600.0, serde_json::json!({
                "app": "firefox.exe",
                "title": "GitHub - Where the world builds software"
            })),
            make_event("2026-11-23T10:00:00Z", 3600.0, serde_json::json!({
                "app": "firefox.exe",
                "title": "Rust Programming Language"
            })),
            make_event("2026-11-23T11:00:00Z", 3600.0, serde_json::json!({
                "app": "chrome.exe",
                "title": "Docs"
            })),
        ];
        let afk_events = vec![
            make_event("2026-11-23T09:00:00Z", 10800.0, serde_json::json!({
                "status": "not-afk"
            })),
        ];

        // Empty web events -> triggers title-only mode
        let site_list = build_site_list(&window_events, &afk_events, &[], range.clone());
        assert_eq!(site_list.len(), 2);
        assert_eq!(site_list[0].domain, "firefox.exe");
        assert_eq!(site_list[0].secs, 7200.0);
        assert_eq!(site_list[0].pages.len(), 2);
        assert_eq!(site_list[0].pages[0].title, "GitHub - Where the world builds software");
        assert_eq!(site_list[0].pages[0].secs, 3600.0);
        assert_eq!(site_list[1].domain, "chrome.exe");
        assert_eq!(site_list[1].secs, 3600.0);

        let report = build_day_report(&window_events, &afk_events, &[], range, "00:00");
        assert_eq!(report.active_secs, 10800.0);
        assert_eq!(report.top_sites.len(), 2);
        assert_eq!(report.top_sites[0].domain, "firefox.exe");
        assert_eq!(report.top_sites[0].secs, 7200.0);
        assert_eq!(report.top_sites[1].domain, "chrome.exe");
        assert_eq!(report.top_sites[1].secs, 3600.0);
    }

    #[test]
    fn test_day_boundaries_non_midnight() {
        let day_start = "04:00";
        let day_start_time = parse_day_start(day_start);

        // Test date_for_time
        let d1_utc = local_to_utc(NaiveDate::from_ymd_opt(2026, 11, 23).unwrap().and_hms_opt(3, 30, 0).unwrap());
        assert_eq!(date_for_time(d1_utc, day_start_time), NaiveDate::from_ymd_opt(2026, 11, 22).unwrap());

        let d2_utc = local_to_utc(NaiveDate::from_ymd_opt(2026, 11, 23).unwrap().and_hms_opt(4, 30, 0).unwrap());
        assert_eq!(date_for_time(d2_utc, day_start_time), NaiveDate::from_ymd_opt(2026, 11, 23).unwrap());

        // Test crossing day boundary
        let start_local = NaiveDate::from_ymd_opt(2026, 11, 23).unwrap().and_hms_opt(3, 0, 0).unwrap();
        let end_local = NaiveDate::from_ymd_opt(2026, 11, 23).unwrap().and_hms_opt(5, 0, 0).unwrap();

        let window_events = vec![
            Event::new(local_to_utc(start_local), 7200.0, serde_json::json!({
                "app": "code.exe",
                "title": "editor"
            })),
        ];
        let afk_events = vec![
            Event::new(local_to_utc(start_local), 7200.0, serde_json::json!({
                "status": "not-afk"
            })),
        ];

        let range = TimeRange {
            start: local_to_utc(start_local),
            end: local_to_utc(end_local),
        };

        let range_hours = build_range_hours(&window_events, &afk_events, range, day_start);
        assert_eq!(range_hours.len(), 2);
        assert_eq!(range_hours[0].date, "2026-11-22");
        assert_eq!(range_hours[0].active_secs, 3600.0); // 03:00 to 04:00
        assert_eq!(range_hours[1].date, "2026-11-23");
        assert_eq!(range_hours[1].active_secs, 3600.0); // 04:00 to 05:00
    }

    #[test]
    fn test_all_four_builders() {
        let range = TimeRange {
            start: utc_dt("2026-11-23T09:00:00Z"),
            end: utc_dt("2026-11-23T17:00:00Z"),
        };
        let window_events = vec![
            make_event("2026-11-23T09:00:00Z", 10800.0, serde_json::json!({
                "app": "code.exe",
                "title": "Visual Studio Code"
            })),
            make_event("2026-11-23T12:00:00Z", 7200.0, serde_json::json!({
                "app": "firefox.exe",
                "title": "Mozilla Firefox"
            })),
            make_event("2026-11-23T14:00:00Z", 10800.0, serde_json::json!({
                "app": "code.exe",
                "title": "Visual Studio Code"
            })),
        ];
        let afk_events = vec![
            make_event("2026-11-23T09:00:00Z", 28800.0, serde_json::json!({
                "status": "not-afk"
            })),
        ];
        let web_events = vec![
            make_event("2026-11-23T12:00:00Z", 5400.0, serde_json::json!({
                "url": "https://github.com/rust-lang/rust",
                "title": "rust-lang/rust: Empowering everyone to build reliable and efficient software."
            })),
            make_event("2026-11-23T13:30:00Z", 1800.0, serde_json::json!({
                "url": "https://docs.rs/serde",
                "title": "serde - Rust"
            })),
        ];

        // 1. build_day_report
        let day_report = build_day_report(&window_events, &afk_events, &web_events, range.clone(), "00:00");
        assert_eq!(day_report.active_secs, 28800.0);
        assert_eq!(day_report.top_apps.len(), 2);
        assert_eq!(day_report.top_apps[0].app, "code.exe");
        assert_eq!(day_report.top_apps[0].secs, 21600.0);
        assert_eq!(day_report.top_apps[1].app, "firefox.exe");
        assert_eq!(day_report.top_apps[1].secs, 7200.0);
        assert_eq!(day_report.top_sites.len(), 2);
        assert_eq!(day_report.top_sites[0].domain, "github.com");
        assert_eq!(day_report.top_sites[0].secs, 5400.0);
        assert_eq!(day_report.top_sites[1].domain, "docs.rs");
        assert_eq!(day_report.top_sites[1].secs, 1800.0);

        // 2. build_range_hours
        let range_hours = build_range_hours(&window_events, &afk_events, range.clone(), "00:00");
        assert_eq!(range_hours.len(), 1);
        assert_eq!(range_hours[0].active_secs, 28800.0);

        // 3. build_site_list
        let site_list = build_site_list(&window_events, &afk_events, &web_events, range);
        assert_eq!(site_list.len(), 2);
        assert_eq!(site_list[0].domain, "github.com");
        assert_eq!(site_list[0].secs, 5400.0);
        assert_eq!(site_list[1].domain, "docs.rs");
        assert_eq!(site_list[1].secs, 1800.0);

        // 4. build_search_index
        let search_index = build_search_index(&site_list);
        assert_eq!(search_index.items.len(), 4); // 2 sites + 2 pages
    }

    #[test]
    fn test_fuzzy_search_modes_and_scopes() {
        let site_list: SiteList = serde_json::from_str(include_str!("../../../docs/fixtures/site_list.json")).unwrap();
        let index = build_search_index(&site_list);

        // 1. mode "sites" without site scope
        let hits_sites = search(&index, "git", "sites", None);
        assert!(!hits_sites.is_empty());
        assert_eq!(hits_sites[0].kind, "site");
        assert_eq!(hits_sites[0].domain, "github.com");

        // 2. mode "titles" without site scope
        let hits_titles = search(&index, "serde", "titles", None);
        assert!(!hits_titles.is_empty());
        assert_eq!(hits_titles[0].kind, "page");
        assert_eq!(hits_titles[0].domain, "docs.rs");
        assert!(hits_titles[0].title.as_ref().unwrap().contains("serde"));

        // 3. mode "both" without site scope
        let hits_both = search(&index, "github", "both", None);
        assert!(!hits_both.is_empty());
        assert!(hits_both.iter().any(|h| h.kind == "site" && h.domain == "github.com"));
        assert!(hits_both.iter().any(|h| h.kind == "page" && h.domain == "github.com"));

        // 4. mode "titles" WITH site scope (scoped to github.com)
        let hits_scoped = search(&index, "rust", "titles", Some("github.com"));
        assert!(!hits_scoped.is_empty());
        assert!(hits_scoped.iter().all(|h| h.domain == "github.com"));
        assert_eq!(hits_scoped[0].title.as_ref().unwrap(), "rust-lang/rust: Empowering everyone to build reliable and efficient software.");

        // Scoped to docs.rs -> should find chrono or serde, not github
        let hits_docs = search(&index, "rust", "titles", Some("docs.rs"));
        assert!(!hits_docs.is_empty());
        assert!(hits_docs.iter().all(|h| h.domain == "docs.rs"));

        // 5. mode "both" WITH site scope (scoped to docs.rs)
        let hits_both_scoped = search(&index, "docs", "both", Some("docs.rs"));
        assert!(!hits_both_scoped.is_empty());
        assert!(hits_both_scoped.iter().all(|h| h.domain == "docs.rs"));

        // Scoped to github.com when searching "docs" -> should find nothing
        let hits_empty = search(&index, "docs", "both", Some("github.com"));
        assert!(hits_empty.is_empty());
    }

    #[test]
    fn test_roundtrip_serialization() {
        let range = TimeRange {
            start: utc_dt("2026-11-23T09:00:00Z"),
            end: utc_dt("2026-11-23T17:00:00Z"),
        };
        let window_events = vec![
            make_event("2026-11-23T09:00:00Z", 10800.0, serde_json::json!({
                "app": "code.exe",
                "title": "Visual Studio Code"
            })),
            make_event("2026-11-23T12:00:00Z", 7200.0, serde_json::json!({
                "app": "chrome.exe",
                "title": "Google Chrome"
            })),
        ];
        let afk_events = vec![
            make_event("2026-11-23T09:00:00Z", 28800.0, serde_json::json!({
                "status": "not-afk"
            })),
        ];
        let web_events = vec![
            make_event("2026-11-23T12:00:00Z", 7200.0, serde_json::json!({
                "url": "https://github.com/rust-lang/rust",
                "title": "rust-lang/rust"
            })),
        ];

        let report = build_day_report(&window_events, &afk_events, &web_events, range.clone(), "00:00");
        let report_json = serde_json::to_string(&report).unwrap();
        let report_deser: DayReport = serde_json::from_str(&report_json).unwrap();
        assert_eq!(report, report_deser);

        let range_hours = build_range_hours(&window_events, &afk_events, range.clone(), "00:00");
        let hours_json = serde_json::to_string(&range_hours).unwrap();
        let hours_deser: RangeHours = serde_json::from_str(&hours_json).unwrap();
        assert_eq!(range_hours, hours_deser);

        let site_list = build_site_list(&window_events, &afk_events, &web_events, range);
        let list_json = serde_json::to_string(&site_list).unwrap();
        let list_deser: SiteList = serde_json::from_str(&list_json).unwrap();
        assert_eq!(site_list, list_deser);

        let index = build_search_index(&site_list);
        let hits = search(&index, "rust", "both", None);
        let hits_json = serde_json::to_string(&hits).unwrap();
        let hits_deser: Vec<Hit> = serde_json::from_str(&hits_json).unwrap();
        assert_eq!(hits, hits_deser);
    }

    #[test]
    fn test_empty_and_edge_inputs() {
        let range = TimeRange {
            start: utc_dt("2026-11-23T09:00:00Z"),
            end: utc_dt("2026-11-23T17:00:00Z"),
        };
        // No window events
        let report = build_day_report(&[], &[], &[], range.clone(), "00:00");
        assert_eq!(report.active_secs, 0.0);
        assert!(report.timeline.is_empty());
        assert!(report.top_apps.is_empty());
        assert!(report.top_sites.is_empty());

        let list = build_site_list(&[], &[], &[], range.clone());
        assert!(list.is_empty());

        let index = build_search_index(&list);
        let hits = search(&index, "anything", "both", None);
        assert!(hits.is_empty());

        // Empty query
        let populated_list: SiteList = serde_json::from_str(include_str!("../../../docs/fixtures/site_list.json")).unwrap();
        let populated_index = build_search_index(&populated_list);
        assert!(search(&populated_index, "", "both", None).is_empty());
        assert!(search(&populated_index, "   ", "both", None).is_empty());
    }
}
