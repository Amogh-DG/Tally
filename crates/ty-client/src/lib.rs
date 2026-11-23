use std::path::PathBuf;
pub use ty_types::BucketSpec;

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

pub struct Client;

impl Client {
    pub fn start(_config: ClientConfig) -> Self {
        todo!("Client::start not yet implemented")
    }

    pub fn register_bucket(&self, _spec: BucketSpec) {
        todo!("Client::register_bucket not yet implemented")
    }

    pub fn heartbeat(&self, _bucket_id: &str, _data: serde_json::Value, _pulsetime: f64) {
        todo!("Client::heartbeat not yet implemented")
    }

    pub fn shutdown(&self) {
        todo!("Client::shutdown not yet implemented")
    }
}
