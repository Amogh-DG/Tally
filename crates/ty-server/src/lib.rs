pub mod cors;
pub mod fake_store;
pub mod routes;
pub mod worker;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use tokio::sync::oneshot;
use ty_types::Store;

pub use fake_store::InMemoryStore;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub data_dir: PathBuf,
    pub extension_origins: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 5600,
            data_dir: PathBuf::from("data"),
            extension_origins: Vec::new(),
        }
    }
}

pub struct ServerHandle {
    port: u16,
    local_addr: SocketAddr,
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
    server_thread: Mutex<Option<JoinHandle<()>>>,
    worker_thread: Mutex<Option<JoinHandle<()>>>,
}

impl ServerHandle {
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn shutdown(&self) {
        if let Some(tx) = self.shutdown_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.server_thread.lock().unwrap().take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.worker_thread.lock().unwrap().take() {
            let _ = handle.join();
        }
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn run(config: ServerConfig) -> ServerHandle {
    run_with_store(config, Box::new(InMemoryStore::new()))
}

pub fn run_with_store(config: ServerConfig, store: Box<dyn Store>) -> ServerHandle {
    let raw_hostname = gethostname::gethostname().to_string_lossy().to_string();
    let sanitized_hostname = raw_hostname.split_whitespace().collect::<Vec<_>>().join("-");
    let server_hostname = if sanitized_hostname.is_empty() {
        "localhost".to_string()
    } else {
        sanitized_hostname
    };

    let mut worker = worker::StoreWorker::spawn(store);
    let store_client = worker.client.clone();
    let worker_thread_handle = worker.thread_handle.take();

    // Bind strictly to 127.0.0.1
    let bind_addr = format!("127.0.0.1:{}", config.port);
    let std_listener = std::net::TcpListener::bind(&bind_addr)
        .unwrap_or_else(|e| panic!("failed to bind ty-server to {bind_addr}: {e}"));
    std_listener
        .set_nonblocking(true)
        .expect("failed to set nonblocking");

    let local_addr = std_listener
        .local_addr()
        .expect("failed to get local addr");
    let actual_port = local_addr.port();

    let state = Arc::new(cors::ServerState {
        port: actual_port,
        server_hostname,
        extension_origins: config.extension_origins,
        store_client,
    });

    let app = routes::create_router(state);

    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let server_thread = thread::Builder::new()
        .name("ty-server-http".to_string())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime for ty-server");

            rt.block_on(async move {
                let listener = tokio::net::TcpListener::from_std(std_listener)
                    .expect("failed to create tokio TcpListener");

                let server = axum::serve(listener, app);
                let graceful = server.with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                });

                if let Err(err) = graceful.await {
                    eprintln!("ty-server error: {err}");
                }
            });
        })
        .expect("failed to spawn ty-server http thread");

    ServerHandle {
        port: actual_port,
        local_addr,
        shutdown_tx: Mutex::new(Some(shutdown_tx)),
        server_thread: Mutex::new(Some(server_thread)),
        worker_thread: Mutex::new(worker_thread_handle),
    }
}
