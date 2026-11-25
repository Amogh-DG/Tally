use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde_json::json;
use ty_afk::{AfkMonitor, AfkStatus};
use ty_client::{Client, ClientConfig};
use ty_merge::pulsetime_for_poll;
use ty_types::BucketSpec;
use ty_winwatch::WindowWatcher;

use crate::active_time::{compute_today_active_seconds, format_active_time};
use crate::hostname::{afk_bucket_id, window_bucket_id};

#[derive(Debug, Clone)]
pub struct WatcherConfig {
    pub base_url: String,
    pub queue_dir: PathBuf,
    pub hostname: String,
    pub afk_timeout_secs: u64,
    pub day_start: String,
    pub poll_window_interval: Duration,
    pub poll_afk_interval: Duration,
    pub settings_interval: Duration,
    pub active_time_interval: Duration,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:5600".to_string(),
            queue_dir: PathBuf::from("data/client_queue"),
            hostname: crate::hostname::get_sanitized_hostname(),
            afk_timeout_secs: 180,
            day_start: "00:00".to_string(),
            poll_window_interval: Duration::from_secs(1),
            poll_afk_interval: Duration::from_secs(5),
            settings_interval: Duration::from_secs(10),
            active_time_interval: Duration::from_secs(60),
        }
    }
}

pub struct WatcherHandle {
    running: Arc<AtomicBool>,
    worker_thread: Option<JoinHandle<()>>,
    client: Client,
}

impl WatcherHandle {
    /// Stops the watcher loop, flushes in-flight heartbeats to the server, and joins worker threads.
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        // Explicitly flush client queue and join worker
        self.client.shutdown();
        if let Some(handle) = self.worker_thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for WatcherHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Spawns the sensing watcher loop on a background thread.
///
/// Polls:
/// - `WindowWatcher` every 1s (pulsetime 2.0s)
/// - `AfkMonitor` every 5s (pulsetime 7.5s)
/// - Re-reads settings every 10s (re-arms AFK timeout)
/// - Updates today's active time about once a minute
///
/// NOTE: Windows sensing logic is UNTESTED until executed on Windows.
pub fn spawn_watcher_loop<F>(
    config: WatcherConfig,
    on_active_time_update: F,
) -> WatcherHandle
where
    F: Fn(&str) + Send + Sync + 'static,
{
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);

    let client = Client::start(ClientConfig {
        base_url: config.base_url.clone(),
        queue_dir: config.queue_dir.clone(),
        commit_interval_secs: 10,
    });

    let window_bucket = window_bucket_id(&config.hostname);
    let afk_bucket = afk_bucket_id(&config.hostname);

    // Register buckets on start
    client.register_bucket(BucketSpec {
        id: window_bucket.clone(),
        bucket_type: "currentwindow".to_string(),
        client: "aw-watcher-window".to_string(),
        hostname: config.hostname.clone(),
    });

    client.register_bucket(BucketSpec {
        id: afk_bucket.clone(),
        bucket_type: "afkstatus".to_string(),
        client: "aw-watcher-afk".to_string(),
        hostname: config.hostname.clone(),
    });

    let client_for_worker = client.clone();
    let on_update = Arc::new(on_active_time_update);

    let worker_thread = thread::Builder::new()
        .name("ty-watcher-sensing".to_string())
        .spawn(move || {
            run_sensing_loop(config, client_for_worker, running_clone, on_update);
        })
        .expect("failed to spawn watcher thread");

    WatcherHandle {
        running,
        worker_thread: Some(worker_thread),
        client,
    }
}

fn run_sensing_loop(
    config: WatcherConfig,
    client: Client,
    running: Arc<AtomicBool>,
    on_active_time_update: Arc<dyn Fn(&str) + Send + Sync>,
) {
    let mut window_watcher = WindowWatcher::new();
    let mut afk_monitor = AfkMonitor::new(config.afk_timeout_secs);
    let mut current_afk_timeout = config.afk_timeout_secs;
    let mut current_day_start = config.day_start.clone();

    let window_bucket = window_bucket_id(&config.hostname);
    let afk_bucket = afk_bucket_id(&config.hostname);

    let window_pulsetime = pulsetime_for_poll(config.poll_window_interval.as_secs_f64());
    let afk_pulsetime = pulsetime_for_poll(config.poll_afk_interval.as_secs_f64());

    let mut last_window = Instant::now() - config.poll_window_interval;
    let mut last_afk = Instant::now() - config.poll_afk_interval;
    let mut last_settings = Instant::now();
    let mut last_active_time = Instant::now();

    // Initial check for active time after a brief startup delay
    let mut initial_active_time_done = false;
    let start_time = Instant::now();

    while running.load(Ordering::SeqCst) {
        let now = Instant::now();

        // 1. Poll WindowWatcher every 1s
        if now.duration_since(last_window) >= config.poll_window_interval {
            last_window = now;
            if let Some(sample) = window_watcher.poll() {
                let data = json!({
                    "app": sample.app,
                    "title": sample.title,
                });
                client.heartbeat(&window_bucket, data, window_pulsetime);
            }
        }

        // 2. Poll AfkMonitor every 5s
        if now.duration_since(last_afk) >= config.poll_afk_interval {
            last_afk = now;
            let status = afk_monitor.poll();
            let status_str = match status {
                AfkStatus::Afk => "afk",
                AfkStatus::NotAfk => "not-afk",
            };
            let data = json!({
                "status": status_str,
            });
            client.heartbeat(&afk_bucket, data, afk_pulsetime);
        }

        // 3. Re-read settings at most every 10s
        if now.duration_since(last_settings) >= config.settings_interval {
            last_settings = now;
            if let Ok(settings) = ty_settings::load() {
                if settings.afk_timeout_secs != current_afk_timeout {
                    current_afk_timeout = settings.afk_timeout_secs;
                    afk_monitor.set_timeout(current_afk_timeout);
                }
                current_day_start = settings.day_start;
            }
        }

        // 4. Update today's active time about once a minute (and after 2s on start)
        let should_update_active_time = (!initial_active_time_done
            && now.duration_since(start_time) >= Duration::from_secs(2))
            || now.duration_since(last_active_time) >= config.active_time_interval;

        if should_update_active_time {
            initial_active_time_done = true;
            last_active_time = now;
            if let Ok(secs) = compute_today_active_seconds(
                &config.base_url,
                &window_bucket,
                &afk_bucket,
                &current_day_start,
            ) {
                let text = format_active_time(secs);
                on_active_time_update(&text);
            }
        }

        // Sleep briefly to yield CPU and handle fast shutdown
        thread::sleep(Duration::from_millis(100));
    }
}
