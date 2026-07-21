# Multi-Window Meeting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow meeting URLs to open in a separate Rust Teams window instead of the system browser, sharing the same Teams session.

**Architecture:** Use `EventLoopProxy` to bridge the new-window-request handler (fires during event loop) with window creation. Keep main WebView unchanged. Meeting windows are tracked in a `Vec` inside the event loop closure.

**Tech Stack:** Rust, tao 0.35, wry 0.55, WebView2

---

## File Changes

| File | Action |
|------|--------|
| `src/main.rs` | Modify: add `AppEvent` enum, `EventLoopProxy`, meeting window creation in event loop |
| `src/ui/meeting_window.rs` | Create: `MeetingWindow` struct wrapping tao::Window + wry::WebView |
| `src/ui/mod.rs` | Modify: add `pub mod meeting_window;` |

No changes to `src/ui/window_manager.rs` (deferred — not needed for initial implementation, meeting windows handled inline in event loop closure).

---

### Task 1: Define AppEvent and wire EventLoopProxy

**Files:**
- Modify: `src/main.rs`

**Key concept:** `tao::EventLoop` has `create_proxy()` which returns `EventLoopProxy<Event>`. This proxy implements `Clone + Send + 'static`, so it can be moved into the `new_window_req_handler` closure. When a meeting URL is detected, the handler sends an `OpenMeeting` event back to the event loop, where we have access to `EventLoopWindowTarget` for creating windows.

- [ ] **Step 1: Add `AppEvent` enum before `main()`**

```rust
/// Custom events sent via EventLoopProxy from callbacks back to the event loop.
#[derive(Debug, Clone)]
enum AppEvent {
    OpenMeeting(String),
}
```

- [ ] **Step 2: Create proxy before building the main window**

Find the `event_loop.run(...)` call at line ~339. Before it, after `let event_loop = EventLoop::new();` (line ~135), add:

```rust
let proxy = event_loop.create_proxy();
```

---

### Task 2: Route meeting URLs through proxy instead of system browser

**Files:**
- Modify: `src/main.rs`

Currently the `new_window_req_handler` at line ~203 opens meeting URLs in the system browser. We change it to send an `AppEvent::OpenMeeting(url)` via proxy instead.

- [ ] **Step 1: Clone proxy into the handler and replace meeting routing**

Find the `with_new_window_req_handler` block (line ~203). Add `proxy` to the captured variables:

```rust
let proxy_for_meetings = proxy.clone();
// ... existing code ...

.with_new_window_req_handler(move |url: String, _features: NewWindowFeatures| {
    let lower = url.to_lowercase();
    let browser = BROWSER_PATH.get()
        .and_then(|m| m.lock().ok())
        .and_then(|g| g.clone());

    // — Meeting/Call URLs now go to Rust Teams meeting window —
    if lower.contains("/meet/")
        || lower.contains("/call/")
        || lower.contains("meetup-join")
        || lower.contains("teams.live.com/meet")
    {
        log::info!("Opening meeting in Rust Teams window: {}", url);
        let _ = proxy_for_meetings.send_event(AppEvent::OpenMeeting(url));
        return NewWindowResponse::Deny;
    }

    // — Existing popout logic unchanged —
    let is_teams_internal = lower.contains("teams.microsoft.com")
        || lower.contains("teams.live.com");
    let is_popout = lower.contains("/l/chat/")
        || lower.contains("/l/person/")
        || lower.contains("/l/channel/")
        || lower.contains("users=");

    if is_teams_internal && is_popout {
        log::info!("Routing Teams pop-out to new Edge window: {}", url);
        if let Err(e) = open_in_new_window(&url) {
            log::warn!("Failed to open in new window, fallback: {}", e);
            let _ = open_url_smart(&url, browser.as_deref());
        }
        return NewWindowResponse::Deny;
    }

    if is_teams_internal {
        NewWindowResponse::Allow
    } else {
        if let Err(e) = open_url_smart(&url, browser.as_deref()) {
            log::warn!("Failed to open URL: {}", e);
        }
        NewWindowResponse::Deny
    }
})
```

---

### Task 3: Create meeting window handling in event loop

**Files:**
- Modify: `src/main.rs`

Add handling of `AppEvent::OpenMeeting` in the event loop. Also handle cleanup when a meeting window is closed.

- [ ] **Step 1: Import WindowBuilder at top of file**

Already imported at line ~15-17:
```rust
use tao::window::{Icon, WindowBuilder};
```

- [ ] **Step 2: Add meeting window tracking in event loop closure**

The `event_loop.run(move |event, _, control_flow|` closure currently captures `cm_for_save` and `config_for_save` by move. Change it to:

```rust
// Before event_loop.run, add:
let meeting_windows: Arc<Mutex<Vec<MeetingWindow>>> = Arc::new(Mutex::new(Vec::new()));
let mw_for_proxy = meeting_windows.clone();

event_loop.run(move |event, event_loop, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
        Event::UserEvent(AppEvent::OpenMeeting(url)) => {
            log::info!("Creating meeting window for: {}", url);
            match MeetingWindow::create(event_loop, &url) {
                Ok(mw) => {
                    if let Ok(mut list) = meeting_windows.lock() {
                        list.push(mw);
                    }
                }
                Err(e) => log::error!("Failed to create meeting window: {}", e),
            }
        }
        // — Existing close handling, enhanced for multi-window —
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            window_id,
            ..
        } => {
            let is_main = window_id == main_window_id; // we need to track this
            if is_main {
                log::info!("Main window close requested, shutting down...");
                // — existing save-on-close logic unchanged —
                #[cfg(target_os = "windows")]
                unsafe {
                    use winapi::um::winuser::GetWindowRect;
                    let mut rect = std::mem::zeroed();
                    if GetWindowRect(hwnd as _, &mut rect) != 0 {
                        config_for_save.window_settings.width = (rect.right - rect.left) as u32;
                        config_for_save.window_settings.height = (rect.bottom - rect.top) as u32;
                    }
                }
                if let Err(e) = cm_for_save.save(&config_for_save) {
                    log::warn!("Failed to save config on close: {e}");
                }
                *control_flow = ControlFlow::Exit;
            } else {
                log::info!("Meeting window closed");
                if let Ok(mut list) = meeting_windows.lock() {
                    list.retain(|mw| mw.window_id() != window_id);
                }
            }
        }
        Event::WindowEvent {
            event: WindowEvent::Destroyed,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        _ => {}
    }
});
```

- [ ] **Step 3: Save main window ID**

At the top of the event loop closure, capture the main window ID. Change:

```rust
let window = window_builder.build(&event_loop)?;
```
to:
```rust
let window = window_builder.build(&event_loop)?;
let main_window_id = window.id();
```

---

### Task 4: Create MeetingWindow module

**Files:**
- Create: `src/ui/meeting_window.rs`
- Modify: `src/ui/mod.rs`

`MeetingWindow` wraps a tao::Window + wry::WebView, providing a clean interface for creation and lifecycle.

- [ ] **Step 1: Create `src/ui/meeting_window.rs`**

```rust
//! Meeting window — separate window for Teams meetings, sharing session with main window.

use std::error::Error;
use tao::dpi::LogicalSize;
use tao::event_loop::EventLoopWindowTarget;
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

/// A meeting window containing a WebView pointed at a Teams meeting URL.
pub struct MeetingWindow {
    window: tao::window::Window,
    webview: wry::WebView,
}

impl MeetingWindow {
    /// Create a new meeting window with the given URL.
    ///
    /// Uses a smaller default size (900x700) and the same WebView2 environment
    /// as the main window (no custom user-data-dir), so cookies/session are shared.
    pub fn create(
        event_loop: &EventLoopWindowTarget<super::AppEvent>,
        url: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let window = WindowBuilder::new()
            .with_title("R Teams Meeting")
            .with_inner_size(LogicalSize::new(900.0, 700.0))
            .build(event_loop)
            .map_err(|e| format!("Failed to create meeting window: {e}"))?;

        let webview = WebViewBuilder::new()
            .with_url(url)
            .build(&window)
            .map_err(|e| format!("Failed to create meeting WebView: {e}"))?;

        log::info!("Meeting window created: {}", url);
        Ok(Self { window, webview })
    }

    /// Return the window ID for matching events.
    pub fn window_id(&self) -> tao::window::WindowId {
        self.window.id()
    }
}
```

- [ ] **Step 2: Add module to `src/ui/mod.rs`**

Find the module declarations in `src/ui/mod.rs` and add:

```rust
pub mod meeting_window;
```

Also add the re-export of `AppEvent` so `meeting_window.rs` can reference it in the type signature. In `src/main.rs`, make `AppEvent` public, or define it in `src/ui/mod.rs` instead.

Better approach: Define `AppEvent` in `src/ui/mod.rs` so both `main.rs` and `meeting_window.rs` can use it.

When creating `public enum AppEvent`, the handler in `main.rs` needs to match on it. Since `main.rs` already imports `ui::*`, this works naturally.

Edit `src/ui/mod.rs`:

```rust
// Add at the top of mod.rs (or near the re-exports):

/// Custom events sent via EventLoopProxy from WebView callbacks
/// back to the main event loop for processing.
#[derive(Debug, Clone)]
pub enum AppEvent {
    OpenMeeting(String),
}
```

---

### Task 5: Wire main window ID and event type into event loop

**Files:**
- Modify: `src/main.rs`

The event loop uses `Event::UserEvent(AppEvent::...)` — we need to make `tao::Event` parameterized on `AppEvent`.

- [ ] **Step 1: Change EventLoop to use AppEvent type**

Change:
```rust
let event_loop = EventLoop::new();
```
to:
```rust
let event_loop: EventLoop<AppEvent> = EventLoop::new();
```

- [ ] **Step 2: Adjust WindowBuilder::build signature**

`WindowBuilder::build(&event_loop)` should work with `EventLoop<AppEvent>` since `EventLoop<T>` implements `AsRef<EventLoopWindowTarget<T>>`.

- [ ] **Step 3: Update event_loop.run closure signature**

Change:
```rust
event_loop.run(move |event, _, control_flow| {
```
to:
```rust
event_loop.run(move |event, event_loop, control_flow| {
```

(We need `event_loop: &EventLoopWindowTarget<AppEvent>` passed to `MeetingWindow::create`.)

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: Clean compile with no errors.

---

### Task 6: Final integration and cleanup

**Files:**
- Modify: `src/main.rs`

Clean up the old meeting URL routing code (remove `open_url_smart` for meetings, since they're now handled by meeting windows).

- [ ] **Step 1: Remove unused meeting URL imports if they become unused**

Check if `open_url_smart` is still used by non-meeting code. It should still be used for non-Teams external URLs, so keep the import.

- [ ] **Step 2: Final check of meeting window lifecycle**

Verify:
1. Main window close → `ControlFlow::Exit` → all meeting windows close with process exit (no orphan windows)
2. Meeting window close → removed from `meeting_windows` vec
3. User clicks multiple meeting links → each creates a new window (no dedup yet — acceptable for v1)

- [ ] **Step 3: Full build and smoke test**

```bash
cargo build --release
# Manual smoke test:
# 1. Run the binary
# 2. Log into Teams
# 3. Click a meeting link
# 4. Verify meeting opens in new Rust Teams window without re-login
```

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/ui/mod.rs src/ui/meeting_window.rs
git commit -m "feat: open Teams meetings in separate Rust Teams window, sharing login session"
```
