use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use ty_types::{BucketSpec, Event};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: u64,
    pub bucket_id: String,
    pub event: Event,
    pub pulsetime: f64,
}

pub struct DiskQueue {
    queue_dir: PathBuf,
    pending_dir: PathBuf,
    tmp_dir: PathBuf,
    buckets_file: PathBuf,
    seq: AtomicU64,
}

impl DiskQueue {
    pub fn new(queue_dir: PathBuf) -> io::Result<Self> {
        let pending_dir = queue_dir.join("pending");
        let tmp_dir = queue_dir.join("tmp");
        let buckets_file = queue_dir.join("buckets.json");

        fs::create_dir_all(&queue_dir)?;
        fs::create_dir_all(&pending_dir)?;
        fs::create_dir_all(&tmp_dir)?;

        // Scan pending_dir to initialize sequence number
        let mut max_seq: u64 = 0;
        if let Ok(entries) = fs::read_dir(&pending_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(num) = stem.parse::<u64>() {
                            max_seq = max_seq.max(num);
                        }
                    }
                }
            }
        }

        Ok(Self {
            queue_dir,
            pending_dir,
            tmp_dir,
            buckets_file,
            seq: AtomicU64::new(max_seq + 1),
        })
    }

    /// Enqueue an event to disk.
    pub fn enqueue(
        &self,
        bucket_id: String,
        event: Event,
        pulsetime: f64,
    ) -> io::Result<(u64, PathBuf)> {
        let id = self.seq.fetch_add(1, Ordering::SeqCst);
        let item = QueueItem {
            id,
            bucket_id,
            event,
            pulsetime,
        };

        let json = serde_json::to_vec(&item)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let tmp_path = self.tmp_dir.join(format!("{:020}.tmp", id));
        let pending_path = self.pending_dir.join(format!("{:020}.json", id));

        fs::write(&tmp_path, json)?;
        fs::rename(&tmp_path, &pending_path)?;

        Ok((id, pending_path))
    }

    /// List all pending queue items in FIFO order.
    pub fn list_pending(&self) -> io::Result<Vec<(PathBuf, QueueItem)>> {
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.pending_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    files.push(path);
                }
            }
        }

        // Sort alphabetically, which matches {:020} numeric order
        files.sort();

        let mut items = Vec::new();
        for path in files {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<QueueItem>(&content) {
                    Ok(item) => items.push((path, item)),
                    Err(e) => {
                        eprintln!("Corrupt queue file {:?}: {}, dropping", path, e);
                        let _ = fs::remove_file(&path);
                    }
                },
                Err(e) => {
                    eprintln!("Error reading queue file {:?}: {}, dropping", path, e);
                    let _ = fs::remove_file(&path);
                }
            }
        }

        Ok(items)
    }

    /// Remove a processed item from disk.
    pub fn remove(&self, path: &Path) -> io::Result<()> {
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Check if pending queue is empty.
    pub fn is_empty(&self) -> bool {
        if let Ok(mut entries) = fs::read_dir(&self.pending_dir) {
            entries.next().is_none()
        } else {
            true
        }
    }

    /// Save a bucket spec to disk in buckets.json.
    pub fn save_bucket_spec(&self, spec: &BucketSpec) -> io::Result<()> {
        let mut map = self.load_bucket_specs().unwrap_or_default();
        map.insert(spec.id.clone(), spec.clone());

        let json = serde_json::to_vec_pretty(&map)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let tmp_file = self.queue_dir.join("buckets.json.tmp");
        fs::write(&tmp_file, json)?;
        fs::rename(&tmp_file, &self.buckets_file)?;
        Ok(())
    }

    /// Load all bucket specs from disk.
    pub fn load_bucket_specs(&self) -> io::Result<HashMap<String, BucketSpec>> {
        if !self.buckets_file.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(&self.buckets_file)?;
        let map = serde_json::from_str::<HashMap<String, BucketSpec>>(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(map)
    }
}
