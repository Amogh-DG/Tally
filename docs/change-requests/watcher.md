# Change Request: watcher

## CR-01: `ty-tray` `Tray::run(self)` takes `self` by value, preventing dynamic active time updates while running

### Status
Submitted / Workaround Applied in `crates/ty-watcher/`

### Problem
In `crates/ty-tray/src/lib.rs`, `Tray::run(self)` consumes ownership of `self`:
```rust
impl Tray {
    pub fn set_total_text(&self, text: &str) { ... }
    pub fn run(self) { ... }
}
```
`Tray::run(self)` enters a blocking Win32 message loop (`GetMessageW`). However, updating today's total active time line dynamically (e.g. once a minute) requires calling `tray.set_total_text(&self, text)`. Because `run(self)` takes `self` by value and `Tray` is neither `Clone` nor provides a shared handle or background update mechanism, callers cannot call `set_total_text` once the message loop has started.

### Proposed Change
Modify `Tray::run` in `ty-tray` to take `&self`:
```rust
impl Tray {
    pub fn run(&self) { ... }
}
```
Or expose a thread-safe, cloneable handle:
```rust
impl Tray {
    pub fn handle(&self) -> TrayHandle { ... }
}
```
where `TrayHandle` implements `Clone + Send` and has `set_total_text(&self, text: &str)`.

### Workaround in `crates/ty-watcher/`
Since other crates are frozen for Phase 1 / Phase 2 per plan rules, `ty-watcher` constructs the tray context menu using `tray-icon` directly while holding an `Arc<Mutex<Option<MenuItem>>>` for the disabled active time line. It delegates all standard actions to `ty-tray`:
- "Open the UI" invokes `ty_tray::open_ui()`.
- Autostart is managed via `ty_tray::enable_autostart()` and `ty_tray::is_autostart_enabled()`.
- Menu IDs match `ty_tray::MENU_ID_*`.
- Action handling logic follows `ty_tray::handle_menu_action()`.

This enables background active time calculation workers to update the menu item every minute while keeping 100% contract compatibility with `ty-tray`.
