use ty_server::{run, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Attempt to load settings via ty_settings; fall back gracefully if failing
    let settings = std::panic::catch_unwind(ty_settings::load)
        .ok()
        .and_then(|res| res.ok())
        .unwrap_or_default();

    let data_dir = settings.resolved_data_dir();
    let config = ServerConfig {
        port: settings.port,
        data_dir,
        extension_origins: settings.extension_origins,
    };

    eprintln!(
        "Starting tm-server on 127.0.0.1:{} (data_dir: {})",
        config.port,
        config.data_dir.display()
    );
    let handle = run(config);

    // Wait for ctrl-c signal to shut down
    let _ = tokio::signal::ctrl_c().await;
    eprintln!("Shutting down tm-server...");
    handle.shutdown();
    eprintln!("tm-server stopped.");

    Ok(())
}
