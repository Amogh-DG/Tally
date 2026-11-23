# Tally parallel build plan

A Windows time tracker in Rust (tray watcher, own server, Tauri + Svelte UI). This plan splits the work so each chat owns separate files and all of Phase 1 runs in parallel.

Read first: `docs/CONTRACTS.md` (frozen interfaces) and `docs/HANDOFF.md` (the original handoff, saved as-is; section 6 is the behavior spec).

## Repo layout
```
tally/
  Cargo.toml            workspace: members = ["crates/*", "ui/src-tauri"]
  docs/                 PLAN.md, CONTRACTS.md, HANDOFF.md, fixtures/, change-requests/
  crates/
    ty-types/  ty-merge/  ty-store/  ty-server/  ty-client/
    ty-winwatch/  ty-afk/  ty-tray/  ty-settings/  ty-analytics/
    ty-watcher/         (Phase 2)
  ui/
    package.json, vite config      (created in Phase 0, frozen)
    src/app/                       shell, router, theme, api.ts
    src/views/{sites,daily,settings}/View.svelte
    src-tauri/
```
The workspace uses a `crates/*` glob, so adding a crate never edits the root `Cargo.toml`.

## Rules for every chat
1. Work in its own git clone or worktree, on branch `chat/<name>`. This also avoids cargo build-lock clashes.
2. Edit only the owned paths below, including your own `Cargo.toml`. Never touch the root `Cargo.toml`, `ty-types`, `docs/CONTRACTS.md`, `ui/src/app/api.ts`, or another chat's files.
3. Need a contract change? Write `docs/change-requests/<name>.md`, work around it, and keep going.
4. Keep `README.md` (what it is, what is done, how it works, what is left) and `PROGRESS.md` inside your own directory, updated after each increment, so another chat can resume.
5. Do not commit `Cargo.lock` during Phase 1.
6. Test with your own fakes (in-memory store, mock HTTP server, JSON fixtures). Do not rely on another chat's unfinished crate.
7. Windows-only code goes behind `cfg(windows)` with a stub elsewhere. Mark it untested until the user has run it on Windows.
8. Work from the spec. Do not read or copy ActivityWatch source.

## Phase 0: one chat, must land on `main` first
**Scaffold + merge.** Owns everything at the root plus `ty-types` and `ty-merge`.
- Workspace, all crates as compile-clean stubs matching CONTRACTS, `.gitignore`, docs, fixtures (`docs/fixtures/*.json`: day_report, range_hours, site_list, search_hits, settings).
- UI scaffold: Tauri + Svelte, dependencies installed (including `chart.js`), `api.ts` with mock mode, empty `View.svelte` for sites, daily and settings.
- `ty-merge` fully implemented, with unit tests from the merge rules.
- Done when: `cargo build --workspace` passes, `cargo test -p ty-merge` passes, `npm install` and the dev server start.

## Phase 1: nine chats in parallel
| # | Chat | Owns (only these paths) | Done when |
|---|---|---|---|
| 1 | store | `crates/ty-store/` | Every Store op passes tests on in-memory and temp-file DBs: newest-first, range filters, replace-last, indexes on (bucket, timestamp), safe under concurrent calls |
| 2 | server | `crates/ty-server/` | With an in-memory fake store inside the crate, all endpoints and status codes work, heartbeat merge-or-insert works, CORS rules and hostname sanitizing pass, `run()` and the exe both work |
| 3 | client | `crates/ty-client/` | Against a mock HTTP server: pre-merge, commit on change or every 10 s, disk queue survives restart, retry on connection error / 408 / 429 / 5xx, recreate bucket on 404, drop item on 400 |
| 4 | sensing | `crates/ty-winwatch/`, `crates/ty-afk/` | Compiles on Windows. Includes `examples/print_window.rs`, which prints app and title every second (the "title catcher"). Handles no foreground window, UWP, elevated windows (`"unknown"`), empty title |
| 5 | shell | `crates/ty-tray/`, `crates/ty-settings/` | Tray with the three menu items, autostart on and off, `open_ui()`, settings load and save with defaults and tests |
| 6 | analytics | `crates/ty-analytics/` | Unit tests from fixtures: AFK and browser intersection, domain merging, title-only mode, day boundaries, all four builders, fuzzy search in all three modes |
| 7 | tauri | `ui/src-tauri/` | Every command in CONTRACTS section 12 works against a mock HTTP server and fixtures: date ranges, settings, JSON export, `server_status` |
| 8 | ui-main | `ui/src/app/` (except `api.ts`), `ui/src/views/sites/`, `ui/src/views/settings/` | Site list with per-site page dropdown, search box with Sites / Titles / Both switch, settings screen, light and dark theme following Windows; all working in mock mode |
| 9 | ui-daily | `ui/src/views/daily/` | Day timeline (plain SVG), top sites and top apps charts, hours per day with week and month toggle; all working in mock mode |

Chats 4 and 5 are small. If your Claude limits are tight, one chat can do both, since their files are still separate.

Known gap: chat 7's glue calls `ty-analytics`, which is still a stub until chat 6 lands, so real data flows only in Phase 2.

## Merge order
Phase 0 first. After that any order works, because the files are disjoint. At integration, delete any `Cargo.lock` conflict, then run `cargo build` and commit the regenerated lockfile.

## Phase 2: sequential, after Phase 1 is merged
1. **Watcher exe** (`crates/ty-watcher/`): wire winwatch, afk, client and tray; spawn and stop `ty-server` (as decided; the server is a library too, so embedding it later is possible); autostart; re-read settings every 10 s.
2. **UI integration:** Tauri backend, analytics and the real server end to end; fix gaps; verify export.
3. **Packaging:** portable folder script (watcher, `ty-server`, UI exe), top-level `README.md`.

## Later (not in v1)
- URL source: browser extension, or history matching. Analytics already handles web events.
- Categories (leave `$category` room in event data).
- Optional privacy filters, Win32 event hooks.

## Brief template for each chat
> You are chat `<name>` on Tally. Read `docs/PLAN.md` and `docs/CONTRACTS.md` (and `docs/HANDOFF.md` section 6 if relevant). You may edit only: `<owned paths>`. Build it so that: `<done when>`. Keep `README.md` and `PROGRESS.md` in your directory updated after each increment. Do not stop to ask whether to continue. Mark Windows-only code as untested.
