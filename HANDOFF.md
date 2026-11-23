# Tally handoff (updated 2026-11-23)

For any Claude session reading this: read all of it, then docs/PLAN.md, docs/CONTRACTS.md and docs/PROMPTS.md. The user has made the decisions below; do not re-ask them. Do not write code until the user says to start. The user dictates by voice (informal phrasing), is on free Claude plans with tight limits, and splits work across several chats, so keep replies short and work in small self-contained modules. Sessions share no memory, so the files in docs/ are the source of truth.

Status: planning is done. PLAN.md (parallel build plan), CONTRACTS.md (frozen interfaces) and PROMPTS.md (one prompt per chat) exist. Phase 0 (scaffold + ty-merge) is the next step unless the user says it has already landed.

## 1. Goal

A Windows time tracker in Rust, behaviorally modeled on ActivityWatch but fully our own code. The Python watchers use too much RAM, so the background part is a tiny Rust watcher. The UI lets the user browse and search where their time goes: sites with per-site page lists, fuzzy search, daily timeline and charts.

Replicate behavior, never copy code. Do not read or copy ActivityWatch source. Section 6 is the behavior spec.

## 2. Decisions (confirmed by the user)

V1 features
- Site list, expandable into pages, with time per page.
- Fuzzy search, global or scoped to one site, with a Sites / Titles / Both switch. With a site selected, title search is scoped to it.
- Daily view: day timeline (app and site blocks), top sites, top apps, hours per day over week or month.
- JSON export and a settings screen (AFK timeout, start of day).
- Categories are not in v1. Leave room in event data (`$category`).

Runtime and packaging
- Tiny always-on watcher with a tray icon. Menu: Open the UI, today's total active time (disabled line), Quit. No pause.
- UI is a separate Tauri + Svelte app, launched from the tray, fully exited when closed.
- Portable folder, no installer: watcher, ty-server, UI exe.
- The watcher starts with Windows, and starts and stops our own ty-server (own server in Rust, SQLite, port 5600, 127.0.0.1 only). We no longer depend on aw-server-rust, so no clash with a standard ActivityWatch install is expected beyond the shared default port.
- UI theme follows the Windows setting.

Behavior
- Record everything: no blocklist or incognito filtering in v1.
- AFK timeout is a setting (default 180 s). Start of day is a setting (default 00:00).
- Keep history forever.
- Page time counts only while the browser is the focused window and the user is not AFK.
- Browsers: Firefox (main) and Chrome.

## 3. Site URLs

A Windows program cannot read a browser tab's URL, only the window title. Options: browser extension (exact URLs, one-time install, recommended), UI Automation address bar (fragile), browser history matching (approximate, no incognito), titles only.

Decision: not now. V1 runs in title-only mode (browser time grouped by app, pages are window titles). ty-analytics already handles web events, so the extension or history matching can be added later without a rewrite. A custom extension must use bucket names starting `aw-watcher-web-` to pass the Firefox CORS restriction.

## 4. Milestone: the title catcher

ty-winwatch ships `examples/print_window.rs`, which prints app and title once per second. It is the first thing the user runs on Windows to prove window detection. Windows-only code cannot be compiled or tested in the Claude sandbox, so it is marked untested until the user runs it.

## 5. Architecture

```
ty-watcher (tray exe)
  ty-winwatch (1 s) + ty-afk (5 s) -> ty-client (pre-merge, disk queue)
        |  heartbeats over HTTP
        v
ty-server (127.0.0.1:5600) -> ty-store (SQLite)
        ^
        |  HTTP
Tauri backend (ui/src-tauri) -> ty-analytics -> Svelte UI (separate exe)
```

Crates: ty-types, ty-merge, ty-store, ty-server, ty-client, ty-winwatch, ty-afk, ty-tray, ty-settings, ty-analytics, ty-watcher (Phase 2), plus ui/src-tauri. The watcher and UI stay separate because a WebView costs far more RAM than a tiny watcher. Exact interfaces are in CONTRACTS.md.

Build order is in PLAN.md: Phase 0 scaffold, Phase 1 nine parallel chats on disjoint paths, Phase 2 watcher then integration and packaging.

## 6. Behavioral specification of ActivityWatch (from reading the source)

Produced by the user's own AI agent reading the cloned repos. Behavior only; implement independently.

### Window watcher (Windows path)
- **Foreground window:** `GetForegroundWindow`. If the handle is 0 (UAC prompt, lock screen, secure desktop, screen saver), the poll is **skipped entirely**: no event is sent, so the previous event ends cleanly.
- **App name:** `GetWindowThreadProcessId`, then open the process, get the full exe path, and take the **file name** (e.g. `firefox.exe`). Use `PROCESS_QUERY_LIMITED_INFORMATION` with `QueryFullProcessImageNameW`, then `CloseHandle`. The original used a fuller access right and a WMI fallback; we skip WMI and fall back to `"unknown"`.
- **Title:** `GetWindowTextW`. If the call fails, use `"unknown"`. An **empty title stays empty** (`""`); it is not replaced by `"unknown"`.
- **UWP apps:** if the app is `ApplicationFrameHost.exe`, enumerate child windows and find a child whose PID differs from the host's; use that process's app name. The child can take roughly 100-500 ms to attach after launch, so keep `ApplicationFrameHost.exe` when none is found yet.
- **Elevated windows:** opening an elevated process can be denied; fall back to `"unknown"`.
- **Polling:** about once per second (default `poll_time` 1.0). Win32 event hooks may be added later as an optimization.
- **Original config options:** `poll_time`, `exclude_title` (replaces every title with `"excluded"`), `exclude_titles` (regex list, matching titles become `"excluded"`), `exclude_apps` (regex list, matching apps are dropped entirely), plus a privacy-filter rule list. (Not needed in v1.)

### Bucket and heartbeat format
- Bucket ID: `aw-watcher-window_<hostname>`, event type `currentwindow`.
- Create bucket: `POST /api/0/buckets/<bucket_id>` with body like `{"client":"aw-watcher-window","type":"currentwindow","hostname":"<host>","data":{}}`. Returns 200 if created, 304 if it already exists (treat as success), 400 if the hostname contains whitespace (**sanitize the hostname**; a special value `"!local"` makes the server fill in its own).
- Heartbeat: `POST /api/0/buckets/<bucket_id>/heartbeat?pulsetime=<seconds>` with body:
  `{"timestamp":"<RFC3339 UTC, fractional seconds>","duration":0.0,"data":{"app":"code.exe","title":"main.rs - Visual Studio Code"}}`
  Returns the inserted or merged event (200). 400 if pulsetime is negative; 404 if the bucket doesn't exist.
- Get events: `GET /api/0/buckets/<bucket_id>/events?start=<rfc3339>&end=<rfc3339>&limit=<n>`, returned newest first.
- Query: `POST /api/0/query` with `{"timeperiods":["<start>/<end>"],"query":["events = query_bucket(find_bucket(\"aw-watcher-window\"));","RETURN = events;"]}`. Returns one result array per time period. (Not in our v1: ty-server returns 501.)
- Server info: `GET /api/0/info`.
- Default port **5600** (5666 in testing mode).

### Pulsetime
- `pulsetime = max(poll_time * 1.5, poll_time + 1.0)`: 2.0 s at a 1 s poll, 7.5 s at 5 s.
- **Privacy-gap rule:** after an excluded window, the next heartbeat must be sent with `pulsetime = 0` so the server never merges across the excluded time. (Only relevant if exclusions are added later.)

### Server merge rules (heartbeat)
A new heartbeat merges into the bucket's latest event only if **all** are true: the `data` is exactly equal, the heartbeat timestamp is not earlier than the last event's start, and the heartbeat timestamp is within `last_end + pulsetime`. When merged: start stays the same, end = max(last end, heartbeat time + heartbeat duration), and duration = end - start. Otherwise a new event is inserted.

### Client behavior to replicate
- **Pre-merge in memory**, using the same merge rules, and send to the server only when the window data changes or about every 10 s (the original default commit interval). Sending an HTTP request every second would flood the server's single database worker.
- **Offline queue:** heartbeats go into a disk-backed queue. On connection errors, 408, 429, or 5xx, retry every ~10 s; never drop them. On 404 for the bucket, recreate the bucket then continue. On 400, log and drop that one item so the queue isn't blocked.

### Server categorization (for later, not v1)
- Query function `categorize(events, rules)`. Rules are `[category_path_array, {type:"regex", regex, ignore_case?, select_keys?, priority?}]`. Without `priority`, rank = path depth x 10. Highest rank wins; ties go to the later-defined rule. Unmatched events become `["Uncategorized"]`. The result is stored in `event.data["$category"]`.

### CORS and extension rules (matters when we add the extension)
- Allowed origins: `localhost`/`127.0.0.1` on the server port, the official Chrome extension ID, and `moz-extension://*` for Firefox. The Firefox origin is restricted to `GET /api/0/info` and `POST` on buckets named `aw-watcher-web-*` (including `/heartbeat`); anything else returns 403. In Tally, extra origins come from the `extension_origins` setting.

### Browser extension (`aw-watcher-web`) behavior
- Bucket: `aw-watcher-web-<browser>_<hostname>`, event type `web.tab.current`.
- Event data: `url` (punycode decoded), `title`, `audible`, `incognito`, `tabCount`, optional `profile`, `tab_group`, `tab_group_color`.
- Sends every 60 s, and immediately on tab activation or when the active tab's URL/title changes. On a tab switch it first sends the previous tab's heartbeat at `now - 1 ms`. Pulsetime is 80 s.
- It reports the **active tab even if the browser is not the focused window**, so analytics must intersect web events with window and AFK data to get accurate time.
- It records incognito tabs (if allowed) with `incognito: true`, and has no blocklist feature.

## 7. Resolved or dropped open questions

- AFK watcher: decided in CONTRACTS section 6. Bucket `aw-watcher-afk_<host>`, type `afkstatus`, data `{status: "afk" | "not-afk"}`, poll 5 s, `GetLastInputInfo` with tick wraparound handled.
- Query language: not needed in v1. Analytics is our own pure Rust library (CONTRACTS section 11), and ty-server's `/api/0/query` returns 501.
- Site-time computation: defined by our own rules in CONTRACTS section 11 (window and web events intersected with browser focus and not-AFK).
- Server embedding and startup: moot, since ty-server is our own crate and exe. It is also a library (`run(ServerConfig)`), so embedding it into the watcher later is possible.
- Settings: `%APPDATA%\Tally\settings.json`, fields in CONTRACTS section 7.
- Rate limits: handled by client pre-merge and the 10 s commit interval.

Still open
- URL source for real site URLs (extension vs history matching), after v1.
- Categories, privacy filters, Win32 event hooks, all later.

## 8. Working agreements

- Get the user's go-ahead before writing code. Prefer small modules that fit in one session.
- Be upfront that Windows-specific code is untested in the sandbox.
- Each chat edits only its own paths, writes contract change requests to docs/change-requests/, and keeps its README.md and PROGRESS.md updated (rules in PLAN.md).
- The user coordinates several free Claude accounts. Claude was not certain how Anthropic's terms treat using multiple free accounts to get around limits; the user should check.
