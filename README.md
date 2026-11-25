# Tidemark

A local-first Windows time tracker written in Rust with a system-tray watcher, a SQLite-backed local HTTP server, and a Tauri + Svelte desktop UI.

## What Works

- **Background Watcher (`tm-watcher.exe` / `crates/ty-watcher`)**:
  - Polls the active Windows foreground application and window title every 1s (`ty-winwatch`), handling UWP (`ApplicationFrameHost.exe`) child windows, elevated/inaccessible windows (`"unknown"`), and missing foreground windows.
  - Polls user idle state every 5s via Win32 `GetLastInputInfo` (`ty-afk`) with configurable AFK timeout (`afk_timeout_secs`, default `180`).
  - Pre-merges identical heartbeats in memory and commits on change or every 10s through an offline-resilient disk queue (`ty-client`) with automatic retry and bucket recreation.
  - Re-reads `%APPDATA%\Tally\settings.json` every 10s so AFK timeout updates take effect without restart.
  - Embeds and starts `tm-server` on `127.0.0.1:<port>` if no server is already listening on that port, and flushes queued heartbeats before shutting down on Quit.
  - System tray (`ty-tray` / `ty-watcher`) with three menu items: **Open the UI** (launches `tidemark-ui.exe` next to the watcher), disabled **Today: Xh Ym** active-time summary updated every minute, and **Quit**. Left-clicking the tray icon also opens the UI.
  - Enables Windows autostart in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` on first run (tracked via `%APPDATA%\Tally\.autostart_initialized`).
- **Local Server & Storage (`tm-server.exe` / `crates/ty-server`, `crates/ty-store`)**:
  - Binds strictly to `127.0.0.1` (default port `5600`) with ActivityWatch-compatible REST endpoints (`/api/0/info`, `/api/0/buckets`, `/events`, `/events/count`, `/heartbeat`), CORS policy, and `!local` hostname resolution.
  - Persists buckets and events in SQLite (`%APPDATA%\Tally\data\tally.db`, WAL mode, indexed on `(bucket_id, timestamp)`).
  - Merges consecutive heartbeats with matching `data` within `pulsetime` (`ty-merge`).
- **Analytics & Search (`crates/ty-analytics`)**:
  - Pure analytics engine computing active time (window events intersected with `not-afk`), local-timezone day boundaries (`day_start`), daily timelines, top apps/sites, multi-day hours, and `nucleo`-powered fuzzy search (`sites`, `titles`, `both`, optionally scoped to a site).
  - Operates in **title-only mode** when no web events are present (grouping browser time for `firefox.exe` and `chrome.exe` by app with window titles as pages), and supports full domain-level merging via `psl` when `web.tab.current` events exist.
- **Desktop UI (`tidemark-ui.exe` / `ui/`)**:
  - Tauri v2 + Svelte + Chart.js desktop app following Windows light/dark theme (`prefers-color-scheme`).
  - **Daily** view: date picker, 24-hour SVG timeline, Top Applications & Top Websites charts, and Past Week / Past Month hours bar chart.
  - **Sites** view: domain/page breakdown with expandable pages and fuzzy search (`Sites` / `Titles` / `Both`, plus per-site scoping).
  - **Settings** view: edit `afk_timeout_secs` and `day_start`, view server/storage info, and export a date range of raw events + computed analytics to JSON (`export_json`).

## How to Run

### Portable Windows Folder

1. Build the portable package from the repository root:
   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts/package-portable.ps1
   ```
   This produces `dist/tidemark-portable/` containing:
   - `tm-watcher.exe`
   - `tm-server.exe`
   - `tidemark-ui.exe`
   - `README.md`

2. Run `tm-watcher.exe` from `dist/tidemark-portable/`.
   - It starts the embedded local server on `127.0.0.1:5600` (using `%APPDATA%\Tally\data\tally.db`), starts tracking window/AFK events, and adds the Tidemark icon to the Windows notification tray.
   - Right-click the tray icon and click **Open the UI** (or left-click the tray icon) to launch `tidemark-ui.exe`.
   - Alternatively, you can run `tm-server.exe` standalone (for example to browse historical data without running the watcher) and launch `tidemark-ui.exe` directly; if `tm-server.exe` is already running when `tm-watcher.exe` starts, the watcher connects to it without binding a second server.

### From Source (Development & Tests)

- Build and run all Rust workspace tests:
  ```powershell
  cargo test --workspace
  ```
- Build the Svelte frontend and launch UI in mock fixture mode (`VITE_MOCK=1`):
  ```powershell
  cd ui
  npm install
  npm run build
  $env:VITE_MOCK = "1"; npm run dev
  ```
- Run the foreground window title-catcher diagnostic:
  ```powershell
  cargo run -p ty-winwatch --example print_window
  ```

### Configuration

Settings are stored at `%APPDATA%\Tally\settings.json` (created with defaults on first run):

```json
{
  "afk_timeout_secs": 180,
  "day_start": "00:00",
  "port": 5600,
  "extension_origins": [],
  "data_dir": "%APPDATA%\\Tally\\data"
}
```

## What Is Left (Post-v1 Roadmap)

- **URL source**: Browser extension (Firefox/Chrome `aw-watcher-web-*` sending `web.tab.current` events) or browser history matching. `ty-analytics` and `ty-server` CORS rules already support web events.
- **Categories**: Rule-based app/title/URL categorization (`$category` metadata in event data and UI category views).
- **Privacy filters & event hooks**: Title/URL redaction or ignore rules, plus optional Win32 `SetWinEventHook` event-driven window tracking instead of 1s polling.
- **Query language**: `POST /api/0/query` currently returns `501 Not Implemented` by design for v1.
