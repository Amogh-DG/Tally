use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub data_dir: PathBuf,
    pub extension_origins: Vec<String>,
}

pub struct ServerHandle;

impl ServerHandle {
    pub fn shutdown(&self) {
        todo!("ServerHandle::shutdown not yet implemented")
    }
}

pub fn run(_config: ServerConfig) -> ServerHandle {
    todo!("ty-server::run not yet implemented")
}
