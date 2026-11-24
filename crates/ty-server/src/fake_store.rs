use std::collections::HashMap;
use std::sync::RwLock;
use chrono::{DateTime, Utc};
use ty_types::{Bucket, Event, Store, StoreError};

#[derive(Debug)]
struct BucketData {
    bucket: Bucket,
    events: Vec<Event>,
}

#[derive(Debug)]
pub struct InMemoryStore {
    data: RwLock<HashMap<String, BucketData>>,
    next_id: RwLock<i64>,
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
            next_id: RwLock::new(1),
        }
    }

    fn generate_id(&self) -> i64 {
        let mut id = self.next_id.write().unwrap();
        let val = *id;
        *id += 1;
        val
    }
}

impl Store for InMemoryStore {
    fn create_bucket(&self, bucket: &Bucket) -> Result<(), StoreError> {
        let mut data = self.data.write().unwrap();
        if data.contains_key(&bucket.id) {
            return Err(StoreError::AlreadyExists);
        }
        data.insert(
            bucket.id.clone(),
            BucketData {
                bucket: bucket.clone(),
                events: Vec::new(),
            },
        );
        Ok(())
    }

    fn get_bucket(&self, id: &str) -> Result<Bucket, StoreError> {
        let data = self.data.read().unwrap();
        data.get(id)
            .map(|b| b.bucket.clone())
            .ok_or(StoreError::NotFound)
    }

    fn list_buckets(&self) -> Result<HashMap<String, Bucket>, StoreError> {
        let data = self.data.read().unwrap();
        let mut map = HashMap::new();
        for (id, b) in data.iter() {
            map.insert(id.clone(), b.bucket.clone());
        }
        Ok(map)
    }

    fn delete_bucket(&self, id: &str) -> Result<(), StoreError> {
        let mut data = self.data.write().unwrap();
        data.remove(id).map(|_| ()).ok_or(StoreError::NotFound)
    }

    fn insert_event(&self, bucket_id: &str, event: &Event) -> Result<Event, StoreError> {
        let mut data = self.data.write().unwrap();
        let bucket_data = data.get_mut(bucket_id).ok_or(StoreError::NotFound)?;

        let mut ev = event.clone();
        if ev.id.is_none() {
            ev.id = Some(self.generate_id());
        }
        bucket_data.events.push(ev.clone());
        Ok(ev)
    }

    fn get_last_event(&self, bucket_id: &str) -> Result<Option<Event>, StoreError> {
        let data = self.data.read().unwrap();
        let bucket_data = data.get(bucket_id).ok_or(StoreError::NotFound)?;

        let last = bucket_data
            .events
            .iter()
            .max_by_key(|e| e.timestamp)
            .cloned();
        Ok(last)
    }

    fn replace_last_event(&self, bucket_id: &str, event: &Event) -> Result<Event, StoreError> {
        let mut data = self.data.write().unwrap();
        let bucket_data = data.get_mut(bucket_id).ok_or(StoreError::NotFound)?;

        if bucket_data.events.is_empty() {
            return Err(StoreError::NotFound);
        }

        // Find index of event with latest timestamp
        let mut max_idx = 0;
        let mut max_time = bucket_data.events[0].timestamp;
        for (i, ev) in bucket_data.events.iter().enumerate().skip(1) {
            if ev.timestamp > max_time {
                max_time = ev.timestamp;
                max_idx = i;
            }
        }

        let old_id = bucket_data.events[max_idx].id;
        let mut updated = event.clone();
        if updated.id.is_none() {
            updated.id = old_id;
        }
        bucket_data.events[max_idx] = updated.clone();
        Ok(updated)
    }

    fn get_events(
        &self,
        bucket_id: &str,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Result<Vec<Event>, StoreError> {
        let data = self.data.read().unwrap();
        let bucket_data = data.get(bucket_id).ok_or(StoreError::NotFound)?;

        let mut filtered: Vec<Event> = bucket_data
            .events
            .iter()
            .filter(|e| {
                if let Some(s) = start {
                    if e.timestamp < s {
                        return false;
                    }
                }
                if let Some(en) = end {
                    if e.timestamp > en {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        // Newest first
        filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        if let Some(lim) = limit {
            filtered.truncate(lim);
        }

        Ok(filtered)
    }

    fn event_count(
        &self,
        bucket_id: &str,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Result<usize, StoreError> {
        let data = self.data.read().unwrap();
        let bucket_data = data.get(bucket_id).ok_or(StoreError::NotFound)?;

        let count = bucket_data
            .events
            .iter()
            .filter(|e| {
                if let Some(s) = start {
                    if e.timestamp < s {
                        return false;
                    }
                }
                if let Some(en) = end {
                    if e.timestamp > en {
                        return false;
                    }
                }
                true
            })
            .count();

        Ok(count)
    }

    fn delete_event(&self, bucket_id: &str, event_id: i64) -> Result<(), StoreError> {
        let mut data = self.data.write().unwrap();
        let bucket_data = data.get_mut(bucket_id).ok_or(StoreError::NotFound)?;

        let init_len = bucket_data.events.len();
        bucket_data.events.retain(|e| e.id != Some(event_id));

        if bucket_data.events.len() == init_len {
            Err(StoreError::NotFound)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use serde_json::json;

    #[test]
    fn test_in_memory_store_buckets() {
        let store = InMemoryStore::new();
        let b1 = Bucket {
            id: "b1".into(),
            bucket_type: "test".into(),
            client: "cli".into(),
            hostname: "host".into(),
            created: Utc::now(),
            data: json!({}),
        };

        // Create
        assert!(store.create_bucket(&b1).is_ok());
        // Duplicate create
        assert_eq!(store.create_bucket(&b1), Err(StoreError::AlreadyExists));

        // Get
        let fetched = store.get_bucket("b1").unwrap();
        assert_eq!(fetched.id, "b1");
        assert_eq!(store.get_bucket("non-existent"), Err(StoreError::NotFound));

        // List
        let list = store.list_buckets().unwrap();
        assert_eq!(list.len(), 1);
        assert!(list.contains_key("b1"));

        // Delete
        assert!(store.delete_bucket("b1").is_ok());
        assert_eq!(store.delete_bucket("b1"), Err(StoreError::NotFound));
    }

    #[test]
    fn test_in_memory_store_events() {
        let store = InMemoryStore::new();
        let b1 = Bucket {
            id: "b1".into(),
            bucket_type: "test".into(),
            client: "cli".into(),
            hostname: "host".into(),
            created: Utc::now(),
            data: json!({}),
        };
        store.create_bucket(&b1).unwrap();

        let t0 = Utc::now();
        let ev1 = Event::new(t0, 1.0, json!({"n": 1}));
        let ev2 = Event::new(t0 + Duration::seconds(2), 1.0, json!({"n": 2}));

        let inserted1 = store.insert_event("b1", &ev1).unwrap();
        let inserted2 = store.insert_event("b1", &ev2).unwrap();
        assert_eq!(inserted1.id, Some(1));
        assert_eq!(inserted2.id, Some(2));

        // Last event is ev2
        let last = store.get_last_event("b1").unwrap().unwrap();
        assert_eq!(last.id, Some(2));

        // Replace last event
        let ev2_updated = Event::new(t0 + Duration::seconds(2), 5.0, json!({"n": 2}));
        let replaced = store.replace_last_event("b1", &ev2_updated).unwrap();
        assert_eq!(replaced.duration, 5.0);
        assert_eq!(replaced.id, Some(2));

        // Get events (newest first)
        let events = store.get_events("b1", None, None, None).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].data["n"], 2);
        assert_eq!(events[1].data["n"], 1);

        // Count
        assert_eq!(store.event_count("b1", None, None).unwrap(), 2);

        // Delete event
        assert!(store.delete_event("b1", 1).is_ok());
        assert_eq!(store.event_count("b1", None, None).unwrap(), 1);
        assert_eq!(store.delete_event("b1", 1), Err(StoreError::NotFound));
    }
}
