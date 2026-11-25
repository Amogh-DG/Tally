pub mod active_time;
pub mod autostart;
pub mod hostname;
pub mod tray;
pub mod watcher;

use std::sync::{Arc, Mutex};
use std::time::Duration;

pub use active_time::{compute_today_active_seconds, compute_today_range, format_active_time};
pub use autostart::{enable_autostart_on_first_run, enable_autostart_with_marker};
pub use hostname::{afk_bucket_id, get_sanitized_hostname, sanitize_hostname, window_bucket_id};
pub use tray::{TrayCallbacks, TrayManager, TrayUpdater};
pub use watcher::{spawn_watcher_loop, WatcherConfig, WatcherHandle};

use ty_server::{run as run_server, ServerConfig, ServerHandle};

/// Main application entry point for the watcher.
///
/// Lifecycle:
/// 1. Enables autostart on first run via Windows registry.
/// 2. Loads settings (`settings.json`).
/// 3. Starts `ty-server` on launch.
/// 4. Connects `ty-client`, registers window and AFK buckets, and starts sensing loop (1s window, 5s AFK).
/// 5. Shows system tray with "Open the UI", disabled active time line updated ~1 min, and "Quit".
/// 6. On Quit: flushes pending heartbeats to server and stops server.
///
/// NOTE: Windows sensing and tray logic are UNTESTED until executed on Windows.
pub fn run_main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Starting Tidemark watcher...");

    // 1. Enable autostart on first run
    if let Err(e) = enable_autostart_on_first_run() {
        eprintln!("Warning: failed to initialize autostart: {e}");
    }

    // 2. Load settings
    let settings = ty_settings::load().unwrap_or_default();
    let port = settings.port;
    let data_dir = settings.resolved_data_dir();
    let extension_origins = settings.extension_origins.clone();
    let base_url = format!("http://127.0.0.1:{}", port);
    let queue_dir = data_dir.join("client_queue");
    let hostname = get_sanitized_hostname();

    // 3. Start server on launch (unless an external server is already running on the port)
    let server_already_running = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_millis(500))
        .timeout_read(Duration::from_millis(500))
        .build()
        .get(&format!("{}/api/0/info", base_url))
        .call()
        .map(|r| r.status() == 200)
        .unwrap_or(false);

    let server_handle: Arc<Mutex<Option<ServerHandle>>> = if server_already_running {
        eprintln!(
            "Existing tm-server detected on 127.0.0.1:{}; connecting without spawning embedded server.",
            port
        );
        Arc::new(Mutex::new(None))
    } else {
        eprintln!(
            "Starting embedded tm-server on 127.0.0.1:{} (data_dir: {})...",
            port,
            data_dir.display()
        );
        Arc::new(Mutex::new(Some(run_server(ServerConfig {
            port,
            data_dir: data_dir.clone(),
            extension_origins,
        }))))
    };

    // 4. Setup watcher and tray
    let watcher_handle: Arc<Mutex<Option<WatcherHandle>>> = Arc::new(Mutex::new(None));

    let server_for_quit = Arc::clone(&server_handle);
    let watcher_for_quit = Arc::clone(&watcher_handle);

    // Quit callback cleans up watcher (flushing client heartbeats) then stops the server
    let quit_action = move || {
        eprintln!("Quit requested. Flushing watcher heartbeats...");
        if let Some(mut watcher) = watcher_for_quit.lock().unwrap().take() {
            watcher.stop();
        }
        eprintln!("Stopping server...");
        if let Some(server) = server_for_quit.lock().unwrap().take() {
            server.shutdown();
        }
        eprintln!("Cleanup complete. Exiting.");
    };

    let quit_action = Arc::new(quit_action);
    let quit_for_tray = Arc::clone(&quit_action);

    // Setup Tray
    let tray_callbacks = TrayCallbacks::new(
        move || {
            eprintln!("Opening UI via ty_tray::open_ui()...");
            if let Err(e) = ty_tray::open_ui() {
                eprintln!("Failed to launch UI: {e}");
            }
        },
        move || {
            quit_for_tray();
        },
    );

    let tray = TrayManager::new(tray_callbacks);
    let updater = tray.updater();

    // 5. Spawn background watcher sensing loop
    let watcher_config = WatcherConfig {
        base_url,
        queue_dir,
        hostname,
        afk_timeout_secs: settings.afk_timeout_secs,
        day_start: settings.day_start,
        poll_window_interval: Duration::from_secs(1),
        poll_afk_interval: Duration::from_secs(5),
        settings_interval: Duration::from_secs(10),
        active_time_interval: Duration::from_secs(60),
    };

    let handle = spawn_watcher_loop(watcher_config, move |time_text| {
        updater.set_total_text(time_text);
    });

    *watcher_handle.lock().unwrap() = Some(handle);

    // 6. Run tray message loop (blocks main thread until Quit is selected)
    eprintln!("Watcher running. Entering tray event loop.");
    tray.run();

    // 7. Ensure clean shutdown if tray loop exits
    quit_action();

    Ok(())
}
