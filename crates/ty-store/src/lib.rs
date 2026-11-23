use std::collections::HashMap;
use std::path::Path;
use chrono::{DateTime, Utc};
use ty_types::{Bucket, Event, Store, StoreError};

pub struct SqliteStore;

impl SqliteStore {
    pub fn open<P: AsRef<Path>>(_path: P) -> Result<Self, StoreError> {
        todo!("SqliteStore::open is not yet implemented")
    }

    pub fn open_in_memory() -> Result<Self, StoreError> {
        todo!("SqliteStore::open_in_memory is not yet implemented")
    }
}

impl Store for SqliteStore {
    fn create_bucket(&self, _bucket: &Bucket) -> Result<(), StoreError> {
        todo!()
    }

    fn get_bucket(&self, _id: &str) -> Result<Bucket, StoreError> {
        todo!()
    }

    fn list_buckets(&self) -> Result<HashMap<String, Bucket>, StoreError> {
        todo!()
    }

    fn delete_bucket(&self, _id: &str) -> Result<(), StoreError> {
        todo!()
    }

    fn insert_event(&self, _bucket_id: &str, _event: &Event) -> Result<Event, StoreError> {
        todo!()
    }

    fn get_last_event(&self, _bucket_id: &str) -> Result<Option<Event>, StoreError> {
        todo!()
    }

    fn replace_last_event(&self, _bucket_id: &str, _event: &Event) -> Result<Event, StoreError> {
        todo!()
    }

    fn get_events(
        &self,
        _bucket_id: &str,
        _start: Option<DateTime<Utc>>,
        _end: Option<DateTime<Utc>>,
        _limit: Option<usize>,
    ) -> Result<Vec<Event>, StoreError> {
        todo!()
    }

    fn event_count(
        &self,
        _bucket_id: &str,
        _start: Option<DateTime<Utc>>,
        _end: Option<DateTime<Utc>>,
    ) -> Result<usize, StoreError> {
        todo!()
    }

    fn delete_event(&self, _bucket_id: &str, _event_id: i64) -> Result<(), StoreError> {
        todo!()
    }
}
