use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use ty_types::BucketSpec;

use crate::in_flight::InFlightManager;
use crate::queue::{DiskQueue, QueueItem};

#[derive(Debug, PartialEq, Eq)]
pub enum SendOutcome {
    Success,
    DropItem(String),
    Retry(String),
}

pub struct WorkerContext {
    pub base_url: String,
    pub commit_interval: Duration,
    pub retry_interval: Duration,
    pub agent: ureq::Agent,
    pub disk_queue: Arc<DiskQueue>,
    pub in_flight: Arc<Mutex<InFlightManager>>,
    pub registered_buckets: Arc<Mutex<HashMap<String, BucketSpec>>>,
    pub shutdown_signal: Arc<AtomicBool>,
    pub notify: Arc<(Mutex<bool>, Condvar)>,
}

pub fn run_worker(ctx: WorkerContext) {
    let mut shutdown_retries = 0;
    loop {
        // 1. Check intervals for any in-flight events that should commit
        let interval_items = {
            let mut in_flight = ctx.in_flight.lock().unwrap();
            in_flight.check_intervals(ctx.commit_interval)
        };
        for (bucket_id, event, pulsetime) in interval_items {
            let _ = ctx.disk_queue.enqueue(bucket_id, event, pulsetime);
        }

        // 2. Read pending queue items from disk
        let pending = ctx.disk_queue.list_pending().unwrap_or_default();

        if pending.is_empty() {
            if ctx.shutdown_signal.load(Ordering::SeqCst) {
                break;
            }

            // Wait for new items, commit interval, or shutdown
            let (lock, cvar) = &*ctx.notify;
            let mut guard = lock.lock().unwrap();
            if !*guard && !ctx.shutdown_signal.load(Ordering::SeqCst) {
                let _ = cvar.wait_timeout(guard, Duration::from_millis(100));
            } else {
                *guard = false;
            }
            continue;
        }

        // 3. Process pending items in order
        let mut retry_needed = false;
        for (path, item) in pending {
            let outcome = send_heartbeat(&ctx.agent, &ctx.base_url, &item, &ctx.registered_buckets);

            match outcome {
                SendOutcome::Success => {
                    let _ = ctx.disk_queue.remove(&path);
                    shutdown_retries = 0;
                }
                SendOutcome::DropItem(reason) => {
                    eprintln!("Dropping queue item {}: {}", item.id, reason);
                    let _ = ctx.disk_queue.remove(&path);
                    shutdown_retries = 0;
                }
                SendOutcome::Retry(reason) => {
                    eprintln!(
                        "Retryable error sending queue item {}: {}",
                        item.id, reason
                    );
                    retry_needed = true;
                    break;
                }
            }
        }

        if retry_needed {
            if ctx.shutdown_signal.load(Ordering::SeqCst) {
                shutdown_retries += 1;
                if shutdown_retries >= 5 {
                    // Don't hang indefinitely if server is offline during shutdown;
                    // items remain persisted on disk for next start
                    break;
                }
            }

            // Wait for retry interval or wake up on signal
            let (lock, cvar) = &*ctx.notify;
            let guard = lock.lock().unwrap();
            let _ = cvar.wait_timeout(guard, ctx.retry_interval);
        }
    }
}

pub fn send_heartbeat(
    agent: &ureq::Agent,
    base_url: &str,
    item: &QueueItem,
    registered_buckets: &Mutex<HashMap<String, BucketSpec>>,
) -> SendOutcome {
    let url = format!(
        "{}/api/0/buckets/{}/heartbeat?pulsetime={}",
        base_url, item.bucket_id, item.pulsetime
    );

    let res = agent.post(&url).send_json(&item.event);
    match res {
        Ok(resp) => {
            if resp.status() >= 200 && resp.status() < 300 {
                SendOutcome::Success
            } else {
                SendOutcome::Success
            }
        }
        Err(ureq::Error::Status(code, _resp)) => match code {
            404 => {
                // 404 for bucket: recreate bucket and retry
                let spec_opt = {
                    let lock = registered_buckets.lock().unwrap();
                    lock.get(&item.bucket_id).cloned()
                };

                match spec_opt {
                    Some(spec) => {
                        let create_url = format!("{}/api/0/buckets/{}", base_url, item.bucket_id);
                        let payload = serde_json::json!({
                            "client": spec.client,
                            "type": spec.bucket_type,
                            "hostname": spec.hostname,
                            "data": {}
                        });

                        match agent.post(&create_url).send_json(payload) {
                            Ok(create_resp) => {
                                let c_status = create_resp.status();
                                if c_status == 200 || c_status == 304 {
                                    // Successfully created or already exists: retry heartbeat
                                    retry_heartbeat(agent, &url, &item.event)
                                } else {
                                    classify_status(c_status)
                                }
                            }
                            Err(ureq::Error::Status(c_code, _)) => {
                                if c_code == 304 {
                                    // 304 Already Exists means success
                                    retry_heartbeat(agent, &url, &item.event)
                                } else {
                                    classify_status(c_code)
                                }
                            }
                            Err(ureq::Error::Transport(e)) => {
                                SendOutcome::Retry(format!("Transport error recreating bucket: {}", e))
                            }
                        }
                    }
                    None => SendOutcome::DropItem(format!(
                        "Bucket {} not found (404) and no spec registered to recreate it",
                        item.bucket_id
                    )),
                }
            }
            code => classify_status(code),
        },
        Err(ureq::Error::Transport(e)) => {
            SendOutcome::Retry(format!("Connection error: {}", e))
        }
    }
}

fn retry_heartbeat(agent: &ureq::Agent, url: &str, event: &ty_types::Event) -> SendOutcome {
    match agent.post(url).send_json(event) {
        Ok(_) => SendOutcome::Success,
        Err(ureq::Error::Status(retry_code, _)) => classify_status(retry_code),
        Err(ureq::Error::Transport(e)) => {
            SendOutcome::Retry(format!("Transport error on retry heartbeat: {}", e))
        }
    }
}

pub fn classify_status(code: u16) -> SendOutcome {
    match code {
        400 => SendOutcome::DropItem("400 Bad Request".to_string()),
        408 => SendOutcome::Retry("408 Request Timeout".to_string()),
        429 => SendOutcome::Retry("429 Too Many Requests".to_string()),
        500..=599 => SendOutcome::Retry(format!("{} Server Error", code)),
        _ => SendOutcome::DropItem(format!("Client error {}", code)),
    }
}
