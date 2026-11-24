use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub use ty_types::BucketSpec;
use chrono::{DateTime, Utc};
use serde_json::Value;

mod in_flight;
mod queue;
mod worker;

use in_flight::InFlightManager;
use queue::DiskQueue;
use worker::{run_worker, WorkerContext};

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub base_url: String,
    pub queue_dir: PathBuf,
    pub commit_interval_secs: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:5600".to_string(),
            queue_dir: PathBuf::from("queue"),
            commit_interval_secs: 10,
        }
    }
}

pub struct ClientInner {
    pub config: ClientConfig,
    pub commit_interval: Duration,
    pub retry_interval: Duration,
    pub disk_queue: Arc<DiskQueue>,
    pub in_flight: Arc<Mutex<InFlightManager>>,
    pub registered_buckets: Arc<Mutex<HashMap<String, BucketSpec>>>,
    pub agent: ureq::Agent,
    pub shutdown_signal: Arc<AtomicBool>,
    pub notify: Arc<(Mutex<bool>, Condvar)>,
    pub worker_handle: Mutex<Option<JoinHandle<()>>>,
}

#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

impl Client {
    /// Start the client with default retry interval (10 s, or overridden by TALLY_CLIENT_RETRY_MS).
    pub fn start(config: ClientConfig) -> Self {
        let retry_interval = env::var("TALLY_CLIENT_RETRY_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_secs(10));

        Self::start_with_retry_interval(config, retry_interval)
    }

    /// Start the client with a custom retry interval (useful for fast automated tests).
    pub fn start_with_retry_interval(config: ClientConfig, retry_interval: Duration) -> Self {
        let disk_queue = Arc::new(
            DiskQueue::new(config.queue_dir.clone())
                .expect("Failed to initialize client disk queue"),
        );

        let initial_specs = disk_queue.load_bucket_specs().unwrap_or_default();
        let registered_buckets = Arc::new(Mutex::new(initial_specs));

        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(5))
            .timeout_read(Duration::from_secs(5))
            .timeout_write(Duration::from_secs(5))
            .max_idle_connections(0)
            .max_idle_connections_per_host(0)
            .build();

        let in_flight = Arc::new(Mutex::new(InFlightManager::new()));
        let shutdown_signal = Arc::new(AtomicBool::new(false));
        let notify = Arc::new((Mutex::new(false), Condvar::new()));

        let commit_interval = Duration::from_secs(config.commit_interval_secs);

        let worker_ctx = WorkerContext {
            base_url: config.base_url.clone(),
            commit_interval,
            retry_interval,
            agent: agent.clone(),
            disk_queue: disk_queue.clone(),
            in_flight: in_flight.clone(),
            registered_buckets: registered_buckets.clone(),
            shutdown_signal: shutdown_signal.clone(),
            notify: notify.clone(),
        };

        let worker_handle = thread::spawn(move || {
            run_worker(worker_ctx);
        });

        Self {
            inner: Arc::new(ClientInner {
                config,
                commit_interval,
                retry_interval,
                disk_queue,
                in_flight,
                registered_buckets,
                agent,
                shutdown_signal,
                notify,
                worker_handle: Mutex::new(Some(worker_handle)),
            }),
        }
    }

    /// Register a bucket specification. Saves to disk and attempts creation on the server.
    pub fn register_bucket(&self, spec: BucketSpec) {
        self.inner
            .registered_buckets
            .lock()
            .unwrap()
            .insert(spec.id.clone(), spec.clone());
        let _ = self.inner.disk_queue.save_bucket_spec(&spec);

        let create_url = format!("{}/api/0/buckets/{}", self.inner.config.base_url, spec.id);
        let payload = serde_json::json!({
            "client": spec.client,
            "type": spec.bucket_type,
            "hostname": spec.hostname,
            "data": {}
        });
        let _ = self.inner.agent.post(&create_url).send_json(payload);
    }

    /// Record a heartbeat for a bucket at current time.
    pub fn heartbeat(&self, bucket_id: &str, data: Value, pulsetime: f64) {
        self.heartbeat_at(bucket_id, data, pulsetime, Utc::now());
    }

    /// Record a heartbeat for a bucket with an explicit timestamp.
    pub fn heartbeat_at(
        &self,
        bucket_id: &str,
        data: Value,
        pulsetime: f64,
        timestamp: DateTime<Utc>,
    ) {
        let commit_opt = {
            let mut in_flight = self.inner.in_flight.lock().unwrap();
            in_flight.heartbeat(
                bucket_id,
                data,
                pulsetime,
                timestamp,
                self.inner.commit_interval,
            )
        };

        if let Some((bucket_id, event, pulsetime)) = commit_opt {
            let _ = self.inner.disk_queue.enqueue(bucket_id, event, pulsetime);
            let (lock, cvar) = &*self.inner.notify;
            let mut guard = lock.lock().unwrap();
            *guard = true;
            cvar.notify_all();
        }
    }

    /// Flushes any in-flight buffer and drains the disk queue to the server.
    pub fn shutdown(&self) {
        // 1. Flush any in-flight buffered events into disk queue
        let to_flush = {
            let mut in_flight = self.inner.in_flight.lock().unwrap();
            in_flight.flush_all()
        };
        for (bucket_id, event, pulsetime) in to_flush {
            let _ = self.inner.disk_queue.enqueue(bucket_id, event, pulsetime);
        }

        // 2. Signal worker to shut down
        self.inner.shutdown_signal.store(true, Ordering::SeqCst);

        // 3. Notify worker
        {
            let (lock, cvar) = &*self.inner.notify;
            let mut guard = lock.lock().unwrap();
            *guard = true;
            cvar.notify_all();
        }

        // 4. Wait for worker to finish sending pending items and terminate
        let handle = self.inner.worker_handle.lock().unwrap().take();
        if let Some(h) = handle {
            let _ = h.join();
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) == 1 {
            self.shutdown();
        }
    }
}
