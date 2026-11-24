use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use ty_types::{Bucket, Event, Store, StoreError};

/// SQLite-backed implementation of the `Store` trait.
///
/// Thread-safe via an internal mutex on the SQLite connection.
#[derive(Clone)]
pub struct SqliteStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStore {
    /// Opens or creates a SQLite database at the given path.
    ///
    /// Automatically creates parent directories if needed.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StoreError> {
        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| StoreError::Internal(format!("Failed to create parent directory: {e}")))?;
            }
        }
        let conn = Connection::open(p)
            .map_err(|e| StoreError::Internal(format!("Failed to open sqlite database: {e}")))?;
        Self::init(conn)
    }

    /// Opens an in-memory SQLite database.
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| StoreError::Internal(format!("Failed to open in-memory sqlite: {e}")))?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Self, StoreError> {
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;

            CREATE TABLE IF NOT EXISTS buckets (
                id TEXT PRIMARY KEY NOT NULL,
                type TEXT NOT NULL,
                client TEXT NOT NULL,
                hostname TEXT NOT NULL,
                created TEXT NOT NULL,
                data TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bucket_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                duration REAL NOT NULL,
                data TEXT NOT NULL,
                FOREIGN KEY (bucket_id) REFERENCES buckets(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_events_bucket_timestamp ON events (bucket_id, timestamp);
            "#,
        )
        .map_err(|e| StoreError::Internal(format!("Failed to initialize database schema: {e}")))?;

        // Attempt WAL mode for durability and concurrent readers (no-op on in-memory)
        let _ = conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;");

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn bucket_exists_internal(conn: &Connection, bucket_id: &str) -> Result<bool, StoreError> {
        let mut stmt = conn
            .prepare("SELECT 1 FROM buckets WHERE id = ?1")
            .map_err(|e| StoreError::Internal(e.to_string()))?;
        let exists = stmt
            .exists(params![bucket_id])
            .map_err(|e| StoreError::Internal(e.to_string()))?;
        Ok(exists)
    }
}

impl Store for SqliteStore {
    fn create_bucket(&self, bucket: &Bucket) -> Result<(), StoreError> {
        if !bucket.data.is_object() {
            return Err(StoreError::BadRequest(
                "Bucket data must be a JSON object".to_string(),
            ));
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        if Self::bucket_exists_internal(&conn, &bucket.id)? {
            return Err(StoreError::AlreadyExists);
        }

        let created_str = bucket.created.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
        let data_str = serde_json::to_string(&bucket.data)
            .map_err(|e| StoreError::BadRequest(format!("Invalid bucket data JSON: {e}")))?;

        let res = conn.execute(
            "INSERT INTO buckets (id, type, client, hostname, created, data) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                bucket.id,
                bucket.bucket_type,
                bucket.client,
                bucket.hostname,
                created_str,
                data_str
            ],
        );

        match res {
            Ok(_) => Ok(()),
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == rusqlite::ffi::ErrorCode::ConstraintViolation =>
            {
                Err(StoreError::AlreadyExists)
            }
            Err(e) => Err(StoreError::Internal(e.to_string())),
        }
    }

    fn get_bucket(&self, id: &str) -> Result<Bucket, StoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        let mut stmt = conn
            .prepare("SELECT id, type, client, hostname, created, data FROM buckets WHERE id = ?1")
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        let mut rows = stmt
            .query(params![id])
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        if let Some(row) = rows.next().map_err(|e| StoreError::Internal(e.to_string()))? {
            let id: String = row.get(0).map_err(|e| StoreError::Internal(e.to_string()))?;
            let bucket_type: String = row.get(1).map_err(|e| StoreError::Internal(e.to_string()))?;
            let client: String = row.get(2).map_err(|e| StoreError::Internal(e.to_string()))?;
            let hostname: String = row.get(3).map_err(|e| StoreError::Internal(e.to_string()))?;
            let created_str: String = row.get(4).map_err(|e| StoreError::Internal(e.to_string()))?;
            let data_str: String = row.get(5).map_err(|e| StoreError::Internal(e.to_string()))?;

            let created = chrono::DateTime::parse_from_rfc3339(&created_str)
                .map_err(|e| StoreError::Internal(format!("Invalid timestamp in DB: {e}")))?
                .with_timezone(&Utc);
            let data: serde_json::Value = serde_json::from_str(&data_str)
                .map_err(|e| StoreError::Internal(format!("Invalid JSON in DB: {e}")))?;

            Ok(Bucket {
                id,
                bucket_type,
                client,
                hostname,
                created,
                data,
            })
        } else {
            Err(StoreError::NotFound)
        }
    }

    fn list_buckets(&self) -> Result<HashMap<String, Bucket>, StoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        let mut stmt = conn
            .prepare("SELECT id, type, client, hostname, created, data FROM buckets")
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        let mut rows = stmt
            .query([])
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        let mut map = HashMap::new();
        while let Some(row) = rows.next().map_err(|e| StoreError::Internal(e.to_string()))? {
            let id: String = row.get(0).map_err(|e| StoreError::Internal(e.to_string()))?;
            let bucket_type: String = row.get(1).map_err(|e| StoreError::Internal(e.to_string()))?;
            let client: String = row.get(2).map_err(|e| StoreError::Internal(e.to_string()))?;
            let hostname: String = row.get(3).map_err(|e| StoreError::Internal(e.to_string()))?;
            let created_str: String = row.get(4).map_err(|e| StoreError::Internal(e.to_string()))?;
            let data_str: String = row.get(5).map_err(|e| StoreError::Internal(e.to_string()))?;

            let created = chrono::DateTime::parse_from_rfc3339(&created_str)
                .map_err(|e| StoreError::Internal(format!("Invalid timestamp in DB: {e}")))?
                .with_timezone(&Utc);
            let data: serde_json::Value = serde_json::from_str(&data_str)
                .map_err(|e| StoreError::Internal(format!("Invalid JSON in DB: {e}")))?;

            map.insert(
                id.clone(),
                Bucket {
                    id,
                    bucket_type,
                    client,
                    hostname,
                    created,
                    data,
                },
            );
        }

        Ok(map)
    }

    fn delete_bucket(&self, id: &str) -> Result<(), StoreError> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        let tx = conn
            .transaction()
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        // Explicitly delete events to ensure cascading even if foreign keys pragma were altered
        tx.execute("DELETE FROM events WHERE bucket_id = ?1", params![id])
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        let rows_affected = tx
            .execute("DELETE FROM buckets WHERE id = ?1", params![id])
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        tx.commit()
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        if rows_affected == 0 {
            Err(StoreError::NotFound)
        } else {
            Ok(())
        }
    }

    fn insert_event(&self, bucket_id: &str, event: &Event) -> Result<Event, StoreError> {
        if !event.data.is_object() {
            return Err(StoreError::BadRequest(
                "Event data must be a JSON object".to_string(),
            ));
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        if !Self::bucket_exists_internal(&conn, bucket_id)? {
            return Err(StoreError::NotFound);
        }

        let ts_str = event.timestamp.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
        let data_str = serde_json::to_string(&event.data)
            .map_err(|e| StoreError::BadRequest(format!("Invalid event data JSON: {e}")))?;

        conn.execute(
            "INSERT INTO events (bucket_id, timestamp, duration, data) VALUES (?1, ?2, ?3, ?4)",
            params![bucket_id, ts_str, event.duration, data_str],
        )
        .map_err(|e| StoreError::Internal(e.to_string()))?;

        let id = conn.last_insert_rowid();
        let mut inserted = event.clone();
        inserted.id = Some(id);
        Ok(inserted)
    }

    fn get_last_event(&self, bucket_id: &str) -> Result<Option<Event>, StoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        if !Self::bucket_exists_internal(&conn, bucket_id)? {
            return Err(StoreError::NotFound);
        }

        let mut stmt = conn
            .prepare(
                "SELECT id, timestamp, duration, data FROM events WHERE bucket_id = ?1 ORDER BY timestamp DESC, id DESC LIMIT 1",
            )
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        let mut rows = stmt
            .query(params![bucket_id])
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        if let Some(row) = rows.next().map_err(|e| StoreError::Internal(e.to_string()))? {
            let id: i64 = row.get(0).map_err(|e| StoreError::Internal(e.to_string()))?;
            let ts_str: String = row.get(1).map_err(|e| StoreError::Internal(e.to_string()))?;
            let duration: f64 = row.get(2).map_err(|e| StoreError::Internal(e.to_string()))?;
            let data_str: String = row.get(3).map_err(|e| StoreError::Internal(e.to_string()))?;

            let timestamp = chrono::DateTime::parse_from_rfc3339(&ts_str)
                .map_err(|e| StoreError::Internal(format!("Invalid timestamp in DB: {e}")))?
                .with_timezone(&Utc);
            let data: serde_json::Value = serde_json::from_str(&data_str)
                .map_err(|e| StoreError::Internal(format!("Invalid JSON in DB: {e}")))?;

            Ok(Some(Event {
                id: Some(id),
                timestamp,
                duration,
                data,
            }))
        } else {
            Ok(None)
        }
    }

    fn replace_last_event(&self, bucket_id: &str, event: &Event) -> Result<Event, StoreError> {
        if !event.data.is_object() {
            return Err(StoreError::BadRequest(
                "Event data must be a JSON object".to_string(),
            ));
        }

        let mut conn = self
            .conn
            .lock()
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        if !Self::bucket_exists_internal(&conn, bucket_id)? {
            return Err(StoreError::NotFound);
        }

        let tx = conn
            .transaction()
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        let latest_id: Option<i64> = {
            let mut stmt = tx
                .prepare(
                    "SELECT id FROM events WHERE bucket_id = ?1 ORDER BY timestamp DESC, id DESC LIMIT 1",
                )
                .map_err(|e| StoreError::Internal(e.to_string()))?;
            let mut rows = stmt
                .query(params![bucket_id])
                .map_err(|e| StoreError::Internal(e.to_string()))?;
            if let Some(row) = rows.next().map_err(|e| StoreError::Internal(e.to_string()))? {
                Some(row.get(0).map_err(|e| StoreError::Internal(e.to_string()))?)
            } else {
                None
            }
        };

        let latest_id = match latest_id {
            Some(id) => id,
            None => return Err(StoreError::NotFound),
        };

        let ts_str = event.timestamp.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
        let data_str = serde_json::to_string(&event.data)
            .map_err(|e| StoreError::BadRequest(format!("Invalid event data JSON: {e}")))?;

        tx.execute(
            "UPDATE events SET timestamp = ?1, duration = ?2, data = ?3 WHERE id = ?4",
            params![ts_str, event.duration, data_str, latest_id],
        )
        .map_err(|e| StoreError::Internal(e.to_string()))?;

        tx.commit()
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        let mut updated = event.clone();
        updated.id = Some(latest_id);
        Ok(updated)
    }

    fn get_events(
        &self,
        bucket_id: &str,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Result<Vec<Event>, StoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        if !Self::bucket_exists_internal(&conn, bucket_id)? {
            return Err(StoreError::NotFound);
        }

        let mut sql = String::from(
            "SELECT id, timestamp, duration, data FROM events WHERE bucket_id = ?1",
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(bucket_id.to_string())];

        if let Some(s) = start {
            let s_str = s.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
            params_vec.push(Box::new(s_str));
            sql.push_str(&format!(" AND timestamp >= ?{}", params_vec.len()));
        }
        if let Some(e) = end {
            let e_str = e.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
            params_vec.push(Box::new(e_str));
            sql.push_str(&format!(" AND timestamp <= ?{}", params_vec.len()));
        }

        sql.push_str(" ORDER BY timestamp DESC, id DESC");

        if let Some(lim) = limit {
            params_vec.push(Box::new(lim as i64));
            sql.push_str(&format!(" LIMIT ?{}", params_vec.len()));
        }

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| StoreError::Internal(e.to_string()))?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let mut rows = stmt
            .query(param_refs.as_slice())
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        let mut events = Vec::new();
        while let Some(row) = rows.next().map_err(|e| StoreError::Internal(e.to_string()))? {
            let id: i64 = row.get(0).map_err(|e| StoreError::Internal(e.to_string()))?;
            let ts_str: String = row.get(1).map_err(|e| StoreError::Internal(e.to_string()))?;
            let duration: f64 = row.get(2).map_err(|e| StoreError::Internal(e.to_string()))?;
            let data_str: String = row.get(3).map_err(|e| StoreError::Internal(e.to_string()))?;

            let timestamp = chrono::DateTime::parse_from_rfc3339(&ts_str)
                .map_err(|e| StoreError::Internal(format!("Invalid timestamp in DB: {e}")))?
                .with_timezone(&Utc);
            let data: serde_json::Value = serde_json::from_str(&data_str)
                .map_err(|e| StoreError::Internal(format!("Invalid JSON in DB: {e}")))?;

            events.push(Event {
                id: Some(id),
                timestamp,
                duration,
                data,
            });
        }

        Ok(events)
    }

    fn event_count(
        &self,
        bucket_id: &str,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Result<usize, StoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        if !Self::bucket_exists_internal(&conn, bucket_id)? {
            return Err(StoreError::NotFound);
        }

        let mut sql = String::from("SELECT COUNT(*) FROM events WHERE bucket_id = ?1");
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(bucket_id.to_string())];

        if let Some(s) = start {
            let s_str = s.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
            params_vec.push(Box::new(s_str));
            sql.push_str(&format!(" AND timestamp >= ?{}", params_vec.len()));
        }
        if let Some(e) = end {
            let e_str = e.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
            params_vec.push(Box::new(e_str));
            sql.push_str(&format!(" AND timestamp <= ?{}", params_vec.len()));
        }

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| StoreError::Internal(e.to_string()))?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let count: i64 = stmt
            .query_row(param_refs.as_slice(), |row| row.get(0))
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        Ok(count as usize)
    }

    fn delete_event(&self, bucket_id: &str, event_id: i64) -> Result<(), StoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        let rows_affected = conn
            .execute(
                "DELETE FROM events WHERE id = ?1 AND bucket_id = ?2",
                params![event_id, bucket_id],
            )
            .map_err(|e| StoreError::Internal(e.to_string()))?;

        if rows_affected == 0 {
            Err(StoreError::NotFound)
        } else {
            Ok(())
        }
    }
}
