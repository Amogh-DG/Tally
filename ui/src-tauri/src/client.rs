use std::collections::HashMap;
use std::time::Duration;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ty_types::{Bucket, Event};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassifiedEvents {
    pub window: Vec<Event>,
    pub afk: Vec<Event>,
    pub web: Vec<Event>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BucketCategories {
    pub window_bucket_ids: Vec<String>,
    pub afk_bucket_ids: Vec<String>,
    pub web_bucket_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TallyHttpClient {
    client: reqwest::Client,
    base_url: String,
}

impl TallyHttpClient {
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_default();
        let trimmed_url = base_url.trim_end_matches('/').to_string();
        Self {
            client,
            base_url: trimmed_url,
        }
    }

    pub fn from_port(port: u16) -> Self {
        Self::new(format!("http://127.0.0.1:{}", port))
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Checks whether the server is reachable and responds with 200 OK on /api/0/info.
    pub async fn check_server_status(&self) -> bool {
        let url = format!("{}/api/0/info", self.base_url);
        match self.client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Fetches all buckets from GET /api/0/buckets/.
    pub async fn fetch_buckets(&self) -> Result<HashMap<String, Bucket>, String> {
        let url = format!("{}/api/0/buckets/", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("HTTP request to {} failed: {}", url, e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "GET {} returned error status {}: {}",
                url, status, body
            ));
        }

        resp.json::<HashMap<String, Bucket>>()
            .await
            .map_err(|e| format!("Failed to parse buckets JSON from {}: {}", url, e))
    }

    /// Classifies bucket IDs into window, AFK, and web buckets.
    pub fn classify_buckets(buckets: &HashMap<String, Bucket>) -> BucketCategories {
        let mut categories = BucketCategories::default();
        for (id, bucket) in buckets {
            if bucket.bucket_type == "currentwindow" || id.starts_with("aw-watcher-window") {
                categories.window_bucket_ids.push(id.clone());
            } else if bucket.bucket_type == "afkstatus" || id.starts_with("aw-watcher-afk") {
                categories.afk_bucket_ids.push(id.clone());
            } else if bucket.bucket_type == "web.tab.current" || id.starts_with("aw-watcher-web") {
                categories.web_bucket_ids.push(id.clone());
            }
        }
        // Ensure deterministic order
        categories.window_bucket_ids.sort();
        categories.afk_bucket_ids.sort();
        categories.web_bucket_ids.sort();
        categories
    }

    /// Fetches events from GET /api/0/buckets/{id}/events with optional start, end, and limit=-1.
    pub async fn fetch_events(
        &self,
        bucket_id: &str,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Result<Vec<Event>, String> {
        let url = format!("{}/api/0/buckets/{}/events", self.base_url, bucket_id);
        let mut req = self.client.get(&url).query(&[("limit", "-1")]);

        if let Some(s) = start {
            req = req.query(&[("start", s.to_rfc3339())]);
        }
        if let Some(e) = end {
            req = req.query(&[("end", e.to_rfc3339())]);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| format!("HTTP request to {} failed: {}", url, e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "GET {} returned error status {}: {}",
                url, status, body
            ));
        }

        resp.json::<Vec<Event>>()
            .await
            .map_err(|e| format!("Failed to parse events JSON for bucket {}: {}", bucket_id, e))
    }

    /// Discovers buckets and fetches events across all window, AFK, and web buckets.
    pub async fn fetch_all_events(
        &self,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Result<ClassifiedEvents, String> {
        let buckets = self.fetch_buckets().await?;
        let categories = Self::classify_buckets(&buckets);

        let mut classified = ClassifiedEvents::default();

        for bucket_id in &categories.window_bucket_ids {
            let mut events = self.fetch_events(bucket_id, start, end).await?;
            classified.window.append(&mut events);
        }

        for bucket_id in &categories.afk_bucket_ids {
            let mut events = self.fetch_events(bucket_id, start, end).await?;
            classified.afk.append(&mut events);
        }

        for bucket_id in &categories.web_bucket_ids {
            let mut events = self.fetch_events(bucket_id, start, end).await?;
            classified.web.append(&mut events);
        }

        Ok(classified)
    }
}
