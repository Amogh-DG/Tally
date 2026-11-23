# Tally contracts (frozen)

Every chat builds against this file. Nobody edits it during Phase 1. If a chat needs a change, it writes `docs/change-requests/<chat-name>.md`, works around the gap, and the integrator decides later.

Wire formats follow ActivityWatch's API behavior (see `docs/HANDOFF.md`, section 6). Do not read or copy ActivityWatch source code.

## 1. Units
- Timestamps: UTC, RFC 3339 with fractional seconds on the wire (`chrono::DateTime<Utc>` in Rust).
- Durations: `f64` seconds.
- Event `data`: a JSON object.

## 2. Core types (crate `ty-types`)
- **Bucket:** `id`, `type` (Rust field `bucket_type`), `client`, `hostname`, `created`, `data` (object).
- **Event:** `id` (optional integer, assigned by the store), `timestamp`, `duration`, `data` (object). A heartbeat is an Event with duration 0.
- **StoreError:** `NotFound`, `AlreadyExists`, `BadRequest(msg)`, `Internal(msg)`.

## 3. Store trait (defined in `ty-types`, implemented by `ty-store`)
Blocking, `Send + Sync`. Operations:
- `create_bucket(bucket)`: created, or `AlreadyExists`.
- `get_bucket(id)`, `list_buckets()`, `delete_bucket(id)`.
- `insert_event(bucket_id, event)`: returns the event with its id.
- `get_last_event(bucket_id)`: the event with the latest timestamp, or none.
- `replace_last_event(bucket_id, event)`: overwrites that latest event in place (used when a heartbeat merges).
- `get_events(bucket_id, start?, end?, limit?)`: newest first.
- `event_count(bucket_id, start?, end?)`, `delete_event(bucket_id, event_id)`.
- Constructors on `SqliteStore`: `open(path)` and `open_in_memory()`.

## 4. Merge (crate `ty-merge`)
`merge_heartbeat(last, heartbeat, pulsetime) -> Option<Event>`

Returns a merged event only if all of these hold:
- the `data` is exactly equal;
- the heartbeat timestamp is not earlier than `last.timestamp`;
- the heartbeat timestamp is within `last_end + pulsetime`.

The merged event keeps `last.timestamp`. Its end is the max of the last end and (heartbeat timestamp + heartbeat duration), and its duration is end minus start. Otherwise it returns `None`.

Helper: `pulsetime_for_poll(poll_secs) = max(poll * 1.5, poll + 1.0)`.

## 5. HTTP API (crate `ty-server`; used by `ty-client` and the Tauri backend)
Default port 5600, bound to 127.0.0.1 only. Configurable.

| Method and path | Behavior |
|---|---|
| `GET /api/0/info` | server info JSON |
| `GET /api/0/buckets/` | map of id to bucket |
| `GET /api/0/buckets/<id>` | bucket, or 404 |
| `POST /api/0/buckets/<id>` | 200 created, 304 already exists, 400 if hostname has whitespace (hostname `!local` is replaced by the server's own) |
| `DELETE /api/0/buckets/<id>` | 200, or 404 |
| `GET /api/0/buckets/<id>/events?start&end&limit` | newest first |
| `POST /api/0/buckets/<id>/events` | insert an array of events |
| `GET /api/0/buckets/<id>/events/count?start&end` | integer |
| `DELETE /api/0/buckets/<id>/events/<event_id>` | 200, or 404 |
| `POST /api/0/buckets/<id>/heartbeat?pulsetime=<s>` | get last event, `merge_heartbeat`, then `replace_last_event` or `insert_event`; returns the resulting event; 400 if pulsetime is negative; 404 if no bucket |
| `POST /api/0/query` | not in v1: return 501 |

CORS: allow `localhost` / `127.0.0.1` on the server port, plus any extension origins listed in settings. `moz-extension://*` may only call `GET /api/0/info` and `POST` on buckets named `aw-watcher-web-*` (including `/heartbeat`); anything else returns 403.

Library entry point: `run(ServerConfig { port, data_dir, extension_origins }) -> ServerHandle` with `shutdown()`. The same crate also builds the exe `ty-server`, which reads `settings.json`.

## 6. Event shapes
| Source | Bucket id | Event type | Data |
|---|---|---|---|
| Window | `aw-watcher-window_<host>` | `currentwindow` | `{app, title}` |
| AFK | `aw-watcher-afk_<host>` | `afkstatus` | `{status: "afk" or "not-afk"}` |
| Web (later) | `aw-watcher-web-<browser>_<host>` | `web.tab.current` | `{url, title, audible, incognito, tabCount, profile?}` |

Window poll is 1 s. AFK poll is 5 s. Pulsetime is `pulsetime_for_poll(poll)`.

## 7. Settings (crate `ty-settings`)
- File: `%APPDATA%\Tally\settings.json`, created with defaults if missing.
- Fields: `afk_timeout_secs` (180), `day_start` ("00:00"), `port` (5600), `extension_origins` ([]), `data_dir` (`%APPDATA%\Tally\data`).
- API: `load()`, `save(&Settings)`, `settings_path()`. Readers re-check the file at most every 10 s.

## 8. Window and AFK sensing
- `ty-winwatch`: `WindowWatcher::new()`, then `poll(&mut self) -> Option<WindowSample { app, title }>`. Returns `None` when there is no foreground window (skip the poll). Behavior per HANDOFF section 6, "Window watcher".
- `ty-afk`: `AfkMonitor::new(timeout_secs)`, `set_timeout(secs)`, `poll(&mut self) -> AfkStatus` (`Afk` or `NotAfk`), `idle_secs()`.
- Both gate Win32 code with `cfg(windows)` and compile a stub on other platforms.

## 9. Client (crate `ty-client`)
- `Client::start(ClientConfig { base_url, queue_dir, commit_interval_secs: 10 }) -> Client`
- `register_bucket(BucketSpec { id, type, client, hostname })`
- `heartbeat(bucket_id, data, pulsetime)`: pre-merges in memory, commits when the data changes or the interval elapses.
- `shutdown()`: flushes.
- Retry and queue rules per HANDOFF section 6, "Client behavior".

## 10. Tray and autostart (crate `ty-tray`)
- `Tray::new(callbacks { on_open_ui, on_quit })`, `set_total_text(text)`, `run()` (blocks, owns the message loop).
- Menu: Open the UI, a disabled line showing today's total active time, Quit. No pause item.
- `enable_autostart()`, `disable_autostart()`, `is_autostart_enabled()` (HKCU Run key).
- `open_ui()` launches `tally-ui.exe` located next to the running exe.

## 11. Analytics (crate `ty-analytics`, pure functions, no I/O)
Input: window events, AFK events, web events (may be empty), a `TimeRange`, and `day_start`. Days use the local timezone.

Rules:
- **Active time:** window events intersected with AFK `not-afk` time.
- **Page time:** web events intersected with window events whose app is a browser (`firefox.exe`, `chrome.exe`) and not-AFK.
- **Domain:** registrable domain via the public suffix list (`m.youtube.com` and `www.youtube.com` merge).
- **Title-only mode:** with no web events, browser time is grouped by app and pages are window titles. Output shapes stay the same.

Output types (JSON field names are snake_case):
- **DayReport:** `date`, `active_secs`, `timeline` (blocks of `start`, `end`, `kind` "app" or "site", `label`, `app`), `top_apps` (`app`, `secs`), `top_sites` (`domain`, `secs`).
- **RangeHours:** list of `date`, `active_secs`.
- **SiteList:** list of `domain`, `secs`, `pages` (`url`, `title`, `secs`), sorted by secs descending.
- **Hit:** `kind` ("site" or "page"), `domain`, `url?`, `title?`, `score`, `secs`.
- **Functions:** `build_day_report`, `build_range_hours`, `build_site_list`, `build_search_index`, and `search(index, query, mode, site?)` where mode is "sites", "titles" or "both". Fuzzy matching uses the `nucleo` crate. If a site is given, title search is scoped to that site.

## 12. Tauri commands (`ui/src-tauri` implements, `ui/src` calls)
All go through `ui/src/app/api.ts`, which is created in Phase 0 and then frozen. It has a mock mode (`VITE_MOCK=1`) that serves `docs/fixtures/*.json`.

| Command | Args | Returns |
|---|---|---|
| `get_day` | `date` (YYYY-MM-DD) | DayReport |
| `get_range_hours` | `from`, `to` | RangeHours |
| `get_sites` | `from`, `to` | SiteList |
| `search` | `query`, `mode`, `site?`, `from`, `to` | list of Hit |
| `export_json` | `from`, `to`, `path` | path written |
| `get_settings` / `set_settings` | Settings | Settings |
| `server_status` | none | `{reachable: bool}` |

Errors are plain strings.

## 13. UI view slots
Each view is a default-exported Svelte component at `ui/src/views/<name>/View.svelte` for `sites`, `daily` and `settings`. The shell imports exactly those paths. The UI theme follows the Windows setting.
