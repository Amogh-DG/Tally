use std::collections::HashMap;
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};
use serde_json::Value;
use ty_merge::merge_heartbeat;
use ty_types::Event;

#[derive(Debug, Clone)]
pub struct BucketBuffer {
    pub current_event: Option<Event>,
    pub pulsetime: f64,
    pub last_commit_time: Instant,
    pub last_committed_duration: Option<f64>,
}

impl BucketBuffer {
    pub fn new() -> Self {
        Self {
            current_event: None,
            pulsetime: 0.0,
            last_commit_time: Instant::now(),
            last_committed_duration: None,
        }
    }
}

impl Default for BucketBuffer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct InFlightManager {
    buffers: HashMap<String, BucketBuffer>,
}

impl InFlightManager {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    /// Process an incoming heartbeat.
    /// Returns Some((bucket_id, event_to_commit, pulsetime)) if an event should be committed
    /// due to data change (committing the previous event) or interval expiration.
    pub fn heartbeat(
        &mut self,
        bucket_id: &str,
        data: Value,
        pulsetime: f64,
        timestamp: DateTime<Utc>,
        commit_interval: Duration,
    ) -> Option<(String, Event, f64)> {
        let buffer = self
            .buffers
            .entry(bucket_id.to_string())
            .or_insert_with(BucketBuffer::new);
        let new_hb = Event::heartbeat(timestamp, data);

        match &buffer.current_event {
            None => {
                buffer.current_event = Some(new_hb);
                buffer.pulsetime = pulsetime;
                buffer.last_commit_time = Instant::now();
                buffer.last_committed_duration = None;
                None
            }
            Some(last) => match merge_heartbeat(last, &new_hb, buffer.pulsetime) {
                Some(merged) => {
                    buffer.current_event = Some(merged.clone());
                    buffer.pulsetime = pulsetime;
                    if buffer.last_commit_time.elapsed() >= commit_interval {
                        buffer.last_commit_time = Instant::now();
                        buffer.last_committed_duration = Some(merged.duration);
                        Some((bucket_id.to_string(), merged, pulsetime))
                    } else {
                        None
                    }
                }
                None => {
                    // Cannot merge: data changed or gap exceeded pulsetime
                    let prev_event = last.clone();
                    let prev_pulsetime = buffer.pulsetime;
                    let already_committed =
                        buffer.last_committed_duration == Some(prev_event.duration);

                    // Reset buffer for the new event
                    buffer.current_event = Some(new_hb);
                    buffer.pulsetime = pulsetime;
                    buffer.last_commit_time = Instant::now();
                    buffer.last_committed_duration = None;

                    if !already_committed {
                        Some((bucket_id.to_string(), prev_event, prev_pulsetime))
                    } else {
                        None
                    }
                }
            },
        }
    }

    /// Check all buffers to see if any in-flight event has reached commit_interval
    /// without a new heartbeat having triggered it.
    pub fn check_intervals(&mut self, commit_interval: Duration) -> Vec<(String, Event, f64)> {
        let mut to_commit = Vec::new();
        for (bucket_id, buffer) in &mut self.buffers {
            if let Some(ref ev) = buffer.current_event {
                if buffer.last_commit_time.elapsed() >= commit_interval {
                    if buffer.last_committed_duration != Some(ev.duration) {
                        buffer.last_commit_time = Instant::now();
                        buffer.last_committed_duration = Some(ev.duration);
                        to_commit.push((bucket_id.clone(), ev.clone(), buffer.pulsetime));
                    }
                }
            }
        }
        to_commit
    }

    /// Flush all current in-flight events (used during shutdown).
    pub fn flush_all(&mut self) -> Vec<(String, Event, f64)> {
        let mut to_commit = Vec::new();
        for (bucket_id, buffer) in &mut self.buffers {
            if let Some(ev) = buffer.current_event.take() {
                if buffer.last_committed_duration != Some(ev.duration) {
                    buffer.last_committed_duration = Some(ev.duration);
                    to_commit.push((bucket_id.clone(), ev, buffer.pulsetime));
                }
            }
        }
        to_commit
    }
}
