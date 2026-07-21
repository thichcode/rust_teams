# Multi-Window: Mở Meeting + Chat Cùng Session

**Date:** 2026-07-17
**Status:** Draft
**Author:** AI brainstorming with user

## Problem

Rust Teams currently routes meeting/call URLs to the system browser, losing the Teams session (login state, cookies). Users cannot simultaneously browse Chat and attend a meeting within the app — a key feature of the official Teams client.

## Goal

Allow users to **open meeting in a separate window** while keeping Chat open in the main window, **sharing the same login session** (no re-login required).

## Architecture

### High-Level Design

```
┌─────────────────────────────────────────────┐
│              WindowManager                   │
│  ┌──────────────────┐  ┌──────────────────┐  │
│  │   MainWindow     │  │  MeetingWindow   │  │
│  │  ┌────────────┐  │  │  ┌────────────┐  │  │
│  │  │  WebView   │  │  │  │  WebView   │  │  │
│  │  │ (Teams     │  │  │  │ (Teams     │  │  │
│  │  │  Chat/UI)  │  │  │  │  Meeting)  │  │  │
│  │  └────────────┘  │  │  └────────────┘  │  │
│  └──────────────────┘  └──────────────────┘  │
│         ▲                       ▲            │
│         │   Shared WebView2     │            │
│         └──── Environment ──────┘            │
│           (cùng user-data-dir)               │
└─────────────────────────────────────────────┘
```

- Both windows share the same `CoreWebView2Environment` (same process = same user data folder)
- WebView2 shares cookies/localStorage/session across controllers natively
- `WindowManager` tracks all open windows and their WebViews

### Flow

1. User clicks meeting link (`/meet/`, `/call/`, `meetup-join`) in main WebView
2. `new_window_req_handler` intercepts → calls `WindowManager::open_meeting(url)`
3. `WindowManager` creates a new `tao::Window` + `wry::WebView` loading the URL
4. Meeting window has its own closed-caption, mic/camera access via WebView2
5. Both windows run on the same event loop, same thread
6. Closing main window closes all meeting windows (cleanup cascade)

### Components

#### New files

- **`src/ui/window_manager.rs`** — Core multi-window management
  - `WindowManager` struct: holds references to main window + list of meeting windows
  - Methods: `new`, `create_main_window`, `open_meeting`, `close_meeting`, `get_window_count`
  - Uses `Vec<MeetingWindow>` for meeting windows, indexed by `WindowId`
  - Thread-safe access patterns (all on event-loop thread, no new threading complexity)

- **`src/ui/meeting_window.rs`** — Meeting window specifics
  - `MeetingWindow` struct: wraps `tao::Window`, `wry::WebView`, `WindowId`
  - Slightly smaller default size (e.g., 800x600 vs main's 1280x800)
  - Title: "R Teams Meeting – <meeting topic>"
  - Minimal initialization scripts (no auto-read, no command-bar, no performance scripts)
  - Close handler: removes self from `WindowManager` tracking

#### Modified files

- **`src/main.rs`**
  - Replace `static WEBVIEW: OnceLock<WebViewHandle>` with `WindowManager` instance
  - Pass `WindowManager` reference to event loop closures
  - Change `new_window_req_handler`: meeting URLs → `WindowManager::open_meeting()`
  - Update IPC handler to reference the correct WebView for `evaluate_script`

- **`src/ui/mod.rs`**
  - Add `pub mod window_manager;`
  - Add `pub mod meeting_window;`

- **`src/ui/badge.rs`** / title handler
  - Main window remains the badge/notification source
  - Meeting window does not set taskbar badge (only main window does)

### Risk & Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| `--renderer-process-limit=2` conflicts | Meeting WebView may have no renderer | Increase to 3 in aggressive mode, or dynamically adjust when meeting opens |
| Extra RAM (~80-150MB per window) | Higher memory usage | Acceptable trade-off — still far below Teams Electron (400-800MB) |
| `WebViewHandle` unsafe Send/Sync | Memory safety if misused | Refactor: remove global static; `WindowManager` owns WebViews and provides safe accessors |
| Mic/camera contested | Both WebViews may request peripherals | Teams web handles this: only active meeting tab requests hardware |
| Multiple meeting links clicked | Orphan windows | Track active meeting URL; focus existing meeting window instead of creating duplicate |

### Testing strategy

- Unit: `WindowManager` lifecycle (create, close, prevent duplicates)
- Integration: Open app → login → click meeting link → verify second window exists
- Integration: Close main window → verify meeting window also closes
- Manual: Verify session is shared (open meeting without re-login)

### Future considerations

- Add "Focus meeting" / "Focus chat" toggle
- Snap window positions (main left, meeting right)
- Meeting window tray icon
- Picture-in-picture mode for meeting video
