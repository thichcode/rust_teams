# Implementation Plan: Icons, Notifications & URL Fix

## Overview

Add modern/flat icons for "R Teams" branding, implement message notifications with badge counter, and fix URL handling to prevent opening in Edge browser.

---

## 1. Add Icons (Modern/Flat Design)

### 1.1 Create Icon Assets

Create `src/assets/` directory with:
- `icon.ico` - Windows icon (taskbar + window)
- `icon_256.png` - High-res PNG for tray
- `icon_32.png` - Small tray icon

**Icon Design:**
- Modern flat style with "R Teams" branding
- Primary color: #6264A7 (Teams purple)
- Simple geometric shape with "R" lettermark
- Clean, minimal aesthetic

### 1.2 Update Cargo.toml

Add Windows resource dependencies:
```toml
[build-dependencies]
winres = "0.1"

[dependencies]
winapi = { version = "0.3", features = ["winuser", "shellapi", "winbase"] }
```

### 1.3 Create build.rs

```rust
fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("src/assets/icon.ico");
        res.compile().unwrap();
    }
}
```

### 1.4 Update Window Creation (main.rs)

Set window icon after creation:
```rust
// After window creation
window.set_window_icon(Some(icon));
```

### 1.5 Update Tray Icon (tray.rs)

Implement actual tray icon using tao's `SystemTray`:
- Load icon from embedded resource
- Set tooltip "R Teams"
- Add context menu (Show, Quit)

---

## 2. Message Notifications

### 2.1 Title Change Detection

**File: src/main.rs**

Add WebView2 title change handler:
```rust
use wry::webview::WebViewBuilder;

let webview = WebViewBuilder::new()
    .with_url(&teams_url)
    .with_on_title_changed(move |title| {
        // Parse unread count from title
        if let Some(count) = parse_unread_count(title) {
            update_taskbar_badge(window.hwnd(), count);
            if count > 0 {
                play_notification_sound();
            }
        }
    })
    .build(&window)?;
```

### 2.2 Unread Count Parser

```rust
fn parse_unread_count(title: &str) -> Option<u32> {
    // Teams title format: "R Teams (3)" or "R Teams - Chat (5)"
    let re = regex::Regex::new(r"\((\d+)\)").ok()?;
    re.captures(title)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse().ok())
}
```

### 2.3 Taskbar Badge (Windows API)

**File: src/ui/badge.rs** (new)

```rust
use winapi::um::winuser::*;
use winapi::um::shellapi::*;
use winapi::shared::windef::HWND;

pub fn update_taskbar_badge(hwnd: isize, count: u32) {
    unsafe {
        let hwnd = hwnd as HWND;
        if count > 0 {
            // Show badge with count
            let badge = THUMBBUTTON {
                iId: 0,
                dwFlags: THBF_ENABLED,
                hIcon: load_badge_icon(count),
                szTip: [0; 260],
            };
            // Use SetWindowLongPtrW to set badge
        } else {
            // Clear badge
        }
    }
}
```

### 2.4 System Notification Sound

```rust
use winapi::um::winuser::MessageBeep;

pub fn play_notification_sound() {
    unsafe {
        MessageBeep(0x00000040); // MB_ICONASTERISK
    }
}
```

---

## 3. Fix URL Handling (Prevent Edge Opening)

### 3.1 WebView2 Navigation Handler

**File: src/main.rs**

Intercept navigation events:
```rust
let webview = WebViewBuilder::new()
    .with_url(&teams_url)
    .with_new_window_req_handler(|url| {
        // Prevent opening in external browser
        // Open in same WebView instead
        log::info!("Intercepted navigation: {}", url);
        // Option: Open in same window
        // Option: Open in new internal window
    })
    .build(&window)?;
```

### 3.2 Alternative: JavaScript Injection

If WebView2 API doesn't support direct interception:
```rust
// Inject JavaScript to override window.open
webview.evaluate_script(r#"
    window.open = function(url) {
        window.location.href = url;
        return null;
    };
"#)?;
```

---

## Implementation Order

1. **Phase 1: Icons**
   - Create icon assets
   - Add build.rs for Windows resources
   - Update window creation with icon
   - Update tray icon implementation

2. **Phase 2: Notifications**
   - Add regex dependency
   - Implement title change handler
   - Add unread count parser
   - Implement taskbar badge
   - Add notification sound

3. **Phase 3: URL Fix**
   - Add navigation handler
   - Test link clicking in Teams
   - Verify URLs open in-app

---

## Dependencies to Add

```toml
[dependencies]
winapi = { version = "0.3", features = ["winuser", "shellapi", "winbase"] }
regex = "1.0"

[build-dependencies]
winres = "0.1"
```

---

## Testing

1. **Icons:** Verify icon appears in taskbar, window title bar, and tray
2. **Notifications:** Send test message, verify badge updates and sound plays
3. **URL Fix:** Click links in Teams, verify they open in-app not Edge

---

## Risk Assessment

- **Low risk:** Icons are cosmetic, no functional impact
- **Medium risk:** Windows API calls require unsafe blocks
- **Low risk:** WebView2 API is stable and well-documented

---

## Estimated Effort

- Phase 1 (Icons): 2-3 hours
- Phase 2 (Notifications): 3-4 hours  
- Phase 3 (URL Fix): 1-2 hours
- **Total:** 6-9 hours
