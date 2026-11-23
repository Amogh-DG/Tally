use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Core bucket specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BucketSpec {
    pub id: String,
    #[serde(rename = "type")]
    pub bucket_type: String,
    pub client: String,
    pub hostname: String,
}

/// Bucket representation in the store.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bucket {
    pub id: String,
    #[serde(rename = "type")]
    pub bucket_type: String,
    pub client: String,
    pub hostname: String,
    pub created: DateTime<Utc>,
    #[serde(default = "default_object")]
    pub data: serde_json::Value,
}

fn default_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

/// An event in a bucket. A heartbeat is an Event with duration 0.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub timestamp: DateTime<Utc>,
    pub duration: f64,
    pub data: serde_json::Value,
}

impl Event {
    pub fn new(timestamp: DateTime<Utc>, duration: f64, data: serde_json::Value) -> Self {
        Self {
            id: None,
            timestamp,
            duration,
            data,
        }
    }

    pub fn heartbeat(timestamp: DateTime<Utc>, data: serde_json::Value) -> Self {
        Self::new(timestamp, 0.0, data)
    }

    /// Calculate the end timestamp of this event.
    pub fn end_time(&self) -> DateTime<Utc> {
        let millis = (self.duration * 1000.0).round() as i64;
        self.timestamp + chrono::Duration::milliseconds(millis)
    }
}

/// Errors returned by the store.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StoreError {
    #[error("Not found")]
    NotFound,
    #[error("Already exists")]
    AlreadyExists,
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Blocking, Send + Sync store trait for buckets and events.
pub trait Store: Send + Sync {
    /// Creates a bucket. Returns Err(StoreError::AlreadyExists) if already present.
    fn create_bucket(&self, bucket: &Bucket) -> Result<(), StoreError>;

    /// Gets a bucket by id or returns Err(StoreError::NotFound).
    fn get_bucket(&self, id: &str) -> Result<Bucket, StoreError>;

    /// Lists all buckets as a map of bucket id to Bucket.
    fn list_buckets(&self) -> Result<HashMap<String, Bucket>, StoreError>;

    /// Deletes a bucket by id or returns Err(StoreError::NotFound).
    fn delete_bucket(&self, id: &str) -> Result<(), StoreError>;

    /// Inserts an event into a bucket and returns the event with its assigned id.
    fn insert_event(&self, bucket_id: &str, event: &Event) -> Result<Event, StoreError>;

    /// Retrieves the event with the latest timestamp in the bucket, or Ok(None) if empty.
    /// Returns Err(StoreError::NotFound) if the bucket does not exist.
    fn get_last_event(&self, bucket_id: &str) -> Result<Option<Event>, StoreError>;

    /// Overwrites the latest event in place (used when a heartbeat merges).
    /// Returns the updated event.
    fn replace_last_event(&self, bucket_id: &str, event: &Event) -> Result<Event, StoreError>;

    /// Queries events in a bucket, newest first, with optional start, end, and limit.
    fn get_events(
        &self,
        bucket_id: &str,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Result<Vec<Event>, StoreError>;

    /// Counts events in a bucket within an optional start and end time range.
    fn event_count(
        &self,
        bucket_id: &str,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Result<usize, StoreError>;

    /// Deletes a specific event from a bucket or returns Err(StoreError::NotFound).
    fn delete_event(&self, bucket_id: &str, event_id: i64) -> Result<(), StoreError>;
}
