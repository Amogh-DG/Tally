# Integration Progress

## Phase 0 & Phase 1
- Phase 0 scaffold (`ty-types`, `ty-merge`, fixtures, UI shell) and Phase 1 crates (`ty-store`, `ty-server`, `ty-client`, `ty-winwatch`, `ty-afk`, `ty-tray`, `ty-settings`, `ty-analytics`, `ui/src-tauri`, `ui/src`) and Phase 2 watcher (`crates/ty-watcher`) merged.

## Phase 2 Integration (chat `integrator`)

### Cross-Crate & Root Changes
1. **`crates/ty-settings/src/lib.rs`**:
   - Added `Settings::resolved_data_dir(&self)` and `expand_path(&Path)` to expand `%APPDATA%`, `%LOCALAPPDATA%`, and `%USERPROFILE%` placeholders at runtime while keeping `Settings::default().data_dir` as `%APPDATA%\Tally\data` for config/fixture compatibility.
2. **`crates/ty-server/` (`Cargo.toml`, `src/lib.rs`, `src/main.rs`, `src/bin/tm-server.rs`, `src/routes.rs`, `tests/server_tests.rs`)**:
   - Added `ty-store` dependency and wired `ty_server::run(ServerConfig)` to open `ty_store::SqliteStore` at `<resolved data_dir>/tally.db` instead of `InMemoryStore`.
   - Added `tm-server` binary target (`src/bin/tm-server.rs`) alongside `ty-server`, resolving `data_dir` via `settings.resolved_data_dir()`.
   - Fixed `EventsQueryParams::limit` from `Option<usize>` to `Option<isize>` (treating negative values such as `limit=-1` sent by `ui/src-tauri/src/client.rs` as unlimited instead of failing with HTTP 400).
   - Updated `server_tests.rs` to isolate `data_dir` per test in `std::env::temp_dir()`.
3. **`crates/ty-tray/` (`build.rs`, `src/lib.rs`)**:
   - Added `build.rs` embedding the Windows ComCtl32 v6 manifest dependency on MSVC so `ty-tray` tests do not fail with `0xc0000139` (`STATUS_ENTRYPOINT_NOT_FOUND`) when workspace feature unification enables `muda/common-controls-v6`.
   - Updated `ui_path()` to resolve `tidemark-ui.exe` (`tidemark-ui` on Unix) next to the running exe, with `open_ui()` falling back to `tally-ui.exe` if `tidemark-ui.exe` is absent.
4. **`crates/ty-watcher/` (`build.rs`, `src/lib.rs`)**:
   - Added `build.rs` embedding the Windows ComCtl32 v6 manifest dependency on MSVC so `tm-watcher.exe` / `ty-watcher.exe` launch cleanly when built in the workspace.
   - Updated `run_main()` to use `settings.resolved_data_dir()` and to probe `GET /api/0/info` first so if an external `tm-server.exe` is already running on the configured port, the watcher connects to it without failing to bind.
5. **`ui/src-tauri/` (`Cargo.toml`, `tauri.conf.json`, `tests/e2e_server_tests.rs`)**:
   - Renamed the Tauri binary target to `tidemark-ui` (`productName: "tidemark-ui"`, window title `"Tidemark"`).
   - Added `ty-server` dev-dependency and `tests/e2e_server_tests.rs` verifying real `ty-server` + `SqliteStore` + watcher-shaped events + `TallyHttpClient` (`?limit=-1`) + `ty-analytics` + `write_export_file` JSON export.
6. **`ui/src/` (`app/App.svelte`, `views/daily/*`, `views/sites/View.svelte`, `views/settings/View.svelte`)**:
   - Fixed `daily/utils.ts` (`parseSecondsInDay`, `formatTimeOfDay`) to render timestamps in local time when running in live Tauri mode while preserving UTC fixture behavior in mock mode.
   - Updated `daily/View.svelte`, `daily/DatePicker.svelte`, `daily/EmptyState.svelte`, and `sites/View.svelte` to default to the current local date in live mode instead of hardcoded `2026-11-23` / UTC `.toISOString()`.
   - Updated `App.svelte` brand title to `Tidemark`, defaulted initial tab to `daily` (where live window + title-only site data immediately appears), and added a live `server_status()` indicator in the sidebar.
   - Added a JSON export section in `settings/View.svelte` wired to `export_json`.
7. **Root (`.gitignore`, `Cargo.lock`, `scripts/package-portable.ps1`, `README.md`, `PROGRESS.md`)**:
   - Updated `.gitignore` to track `Cargo.lock` and top-level `README.md` / `PROGRESS.md` while keeping per-crate docs and `dist/` ignored.
   - Added `scripts/package-portable.ps1` to build and assemble `dist/tidemark-portable/` (`tm-watcher.exe`, `tm-server.exe`, `tidemark-ui.exe`, `README.md`).

### Verification Status
- **Verified**:
  - `cargo test --workspace`: all workspace crates pass (including `ty-tray`, `ty-watcher`, `ty-server` with `SqliteStore`, `ty-settings`, `ty-analytics`, `tally-ui` `plumbing_tests` and `e2e_server_tests`).
  - Frontend build (`npm run build` in `ui/`) and mock verification scripts (`ui/src/app/test_mock.mjs`, `ui/src/views/daily/verify.js`).
  - Portable packaging script (`scripts/package-portable.ps1`) produces `dist/tidemark-portable/` (`tm-watcher.exe`, `tm-server.exe`, `tidemark-ui.exe`, `README.md`).
  - Portable Windows smoke test: `tm-watcher.exe` starts cleanly, initializes `%APPDATA%\Tally\data\tally.db`, registers `aw-watcher-window_<host>` and `aw-watcher-afk_<host>`, records real AFK heartbeats (`not-afk`), `tidemark-ui.exe` launches cleanly, and standalone `tm-server.exe` reads persisted SQLite data.
- **Untested (requires interactive Windows desktop session)**:
  - Live `ty-winwatch` foreground window capture (`GetForegroundWindow()` returned null in the non-interactive session).
  - Interactive Windows notification tray click -> `tidemark-ui.exe` launch and visual WebView2 UI rendering with live data.
  - First-run `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` autostart registration during live desktop launch (test used pre-seeded `.autostart_initialized`).
