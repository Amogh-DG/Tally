# Tally

A lightweight Windows time tracker in Rust with a tray watcher, local SQLite storage server, and Tauri + Svelte UI.

## What It Is
Tally tracks active window focus and user idle status, aggregating time spent across desktop applications and browser pages into browsable timelines, top-site/top-app summaries, and fuzzy-searchable history.

## What Is Done (Phase 0 Scaffold + Merge)
- **Cargo Workspace**: Configured root `Cargo.toml` with `members = ["crates/*", "ui/src-tauri"]`.
- **`ty-types`**: Implemented core data models (`Bucket`, `BucketSpec`, `Event`, `StoreError`) and the blocking `Store` trait.
- **`ty-merge`**: Fully implemented heartbeat merging (`merge_heartbeat`, `pulsetime_for_poll`) per CONTRACTS section 4 with comprehensive unit tests:
  - Equal data merges cleanly.
  - Different data does not merge.
  - Timestamps earlier than last event start are rejected.
  - Heartbeats beyond `last_end + pulsetime` do not merge.
  - Duration calculation correctly implements `max(last_end, hb_end) - start`.
- **Public API Stubs**: Created compile-clean stubs whose public APIs and signatures match CONTRACTS.md:
  - `crates/ty-store` (SqliteStore with `open`, `open_in_memory`, and `Store` trait impl)
  - `crates/ty-server` (`run`, `ServerConfig`, `ServerHandle`, and `ty-server` binary)
  - `crates/ty-client` (`Client::start`, `register_bucket`, `heartbeat`, `shutdown`)
  - `crates/ty-winwatch` (`WindowWatcher`, `WindowSample`, `examples/print_window.rs`; *Windows-only code marked untested*)
  - `crates/ty-afk` (`AfkMonitor`, `AfkStatus`; *Windows-only code marked untested*)
  - `crates/ty-tray` (`Tray`, `TrayCallbacks`, autostart helpers, `open_ui`; *Windows-only code marked untested*)
  - `crates/ty-settings` (`Settings`, `load`, `save`, `settings_path`)
  - `crates/ty-analytics` (Data types, `build_day_report`, `build_range_hours`, `build_site_list`, `build_search_index`, `search`)
  - `crates/ty-watcher` (Watcher binary stub)
  - `ui/src-tauri` (Tauri commands: `get_day`, `get_range_hours`, `get_sites`, `search`, `export_json`, `get_settings`, `set_settings`, `server_status`)
- **Documentation & Fixtures**:
  - `docs/fixtures/`: `day_report.json`, `range_hours.json`, `site_list.json`, `search_hits.json`, `settings.json`.
- **UI Scaffold (`ui/`)**:
  - Svelte + Vite + TypeScript frontend.
  - Installed dependencies including `chart.js` and `@tauri-apps/api`.
  - Typed API client `ui/src/app/api.ts` with mock mode (`VITE_MOCK=1`) loading `docs/fixtures`.
  - View slots: `ui/src/views/{sites,daily,settings}/View.svelte`.
  - Application shell with light/dark theme matching Windows settings.

## How It Works
- The background watcher polls the active foreground window (1 s) and AFK status (5 s).
- Events are pre-merged locally using `ty-merge` rules and batched/queued via `ty-client` to `ty-server` (port 5600 on 127.0.0.1).
- `ty-server` writes records to SQLite via `ty-store`.
- The user interface launches via Tauri and invokes `ty-analytics` to query, slice, and fuzzy-search active time.

## What Is Left
- **Phase 1 Parallel Work**:
  1. `ty-store`: SQLite implementation and test suite.
  2. `ty-server`: HTTP endpoints, CORS rules, and heartbeat processing.
  3. `ty-client`: Queue management, offline disk spooling, and retry logic.
  4. `ty-winwatch` & `ty-afk`: Live Win32 polling implementations.
  5. `ty-tray` & `ty-settings`: Tray menu and persistent settings management.
  6. `ty-analytics`: Day reports, domain resolution via PSL, and fuzzy search indexing.
  7. `ui/src-tauri`: Real backend IPC command implementations.
  8. `ui-main`: Site list, search box, and settings views.
  9. `ui-daily`: Timeline SVG and Chart.js graphs.
- **Phase 2**:
  - Watcher service wiring, end-to-end integration, and packaging.
