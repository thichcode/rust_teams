# Chat Pop-out Second Window Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a hover action to each Teams Chat row that opens the selected chat in one reusable secondary Rust Teams window sharing the main WebView2 session.

**Architecture:** Inject a DOM observer that calls `window.open` for validated Teams Chat links. Route those popup requests through `EventLoopProxy<AppEvent>`, then create or navigate one retained `ChatWindow`; on Windows, build it with the main WebView's exact `ICoreWebView2Environment` so cookies and login state are shared.

**Tech Stack:** Rust 2024, tao 0.35, wry 0.55/WebView2, injected JavaScript, built-in Rust tests

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `src/ui/chat_popout.rs` | Create | Classify Teams Chat URLs and generate the Chat-row injection script |
| `src/ui/chat_window.rs` | Create | Own, navigate, focus, and identify the single secondary window |
| `src/ui/meeting_window.rs` | Delete | Remove the unfinished meeting-specific draft superseded by `ChatWindow` |
| `src/ui/mod.rs` | Modify | Export the new modules and change the custom event to `OpenChat` |
| `src/main.rs` | Modify | Install the script, route popup requests, share the WebView2 environment, and manage one secondary window |

The worktree already contains uncommitted multi-window meeting changes. Preserve unrelated edits, but replace the directly conflicting `MeetingWindow`/`OpenMeeting` draft as approved in the design. Do not commit unless the user explicitly requests it.

### Task 1: Teams Chat URL Classifier

**Files:**
- Create: `src/ui/chat_popout.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Register the module and write failing URL tests**

Add this declaration to `src/ui/mod.rs`:

```rust
pub mod chat_popout;
```

Create `src/ui/chat_popout.rs` with tests first:

```rust
//! Chat pop-out helpers and the script injected into Teams WebViews.

#[cfg(test)]
mod tests {
    use super::is_teams_chat_url;

    #[test]
    fn accepts_supported_teams_chat_urls() {
        assert!(is_teams_chat_url(
            "https://teams.microsoft.com/l/chat/0/0?users=alice@example.com"
        ));
        assert!(is_teams_chat_url(
            "https://teams.microsoft.com/v2/?ctx=chat&chatId=19%3Aabc"
        ));
        assert!(is_teams_chat_url(
            "https://teams.live.com/v2/?users=alice%40example.com"
        ));
    }

    #[test]
    fn rejects_non_chat_and_untrusted_urls() {
        assert!(!is_teams_chat_url(
            "https://teams.microsoft.com/l/channel/19%3Aabc/general"
        ));
        assert!(!is_teams_chat_url(
            "https://teams.microsoft.com/meet/123456"
        ));
        assert!(!is_teams_chat_url(
            "https://teams.microsoft.com.evil.example/l/chat/0/0?users=a"
        ));
        assert!(!is_teams_chat_url("not a url"));
    }
}
```

- [ ] **Step 2: Run the focused tests and confirm RED**

Run: `cargo test ui::chat_popout::tests -- --nocapture`

Expected: compilation fails because `is_teams_chat_url` is not defined.

- [ ] **Step 3: Implement the minimal URL classifier**

Insert before the test module in `src/ui/chat_popout.rs`:

```rust
use reqwest::Url;

/// Return true only for a Teams URL that identifies a chat conversation.
pub fn is_teams_chat_url(raw_url: &str) -> bool {
    let Ok(url) = Url::parse(raw_url) else {
        return false;
    };
    let Some(host) = url.host_str().map(str::to_ascii_lowercase) else {
        return false;
    };
    let trusted_host = host == "teams.microsoft.com"
        || host.ends_with(".teams.microsoft.com")
        || host == "teams.live.com";
    if !trusted_host {
        return false;
    }

    let path = url.path().to_ascii_lowercase();
    if path.contains("/l/channel/")
        || path.contains("/meet/")
        || path.contains("/call/")
        || path.contains("meetup-join")
    {
        return false;
    }

    path.contains("/l/chat/")
        || path.contains("/chat/")
        || url.query_pairs().any(|(key, value)| {
            (key.eq_ignore_ascii_case("ctx") && value.eq_ignore_ascii_case("chat"))
                || key.eq_ignore_ascii_case("users")
        })
}
```

- [ ] **Step 4: Run the focused tests and confirm GREEN**

Run: `cargo test ui::chat_popout::tests -- --nocapture`

Expected: both URL-classifier tests pass.

### Task 2: Chat-row Pop-out Injection

**Files:**
- Modify: `src/ui/chat_popout.rs`

- [ ] **Step 1: Add a failing script-contract test**

Add this test inside `mod tests`:

```rust
    #[test]
    fn injection_script_contains_popup_and_deduplication_contracts() {
        let script = super::get_chat_popout_script();
        assert!(script.contains("data-rteams-chat-popout-ready"));
        assert!(script.contains("MutationObserver"));
        assert!(script.contains("window.open(chatUrl"));
        assert!(script.contains("stopPropagation"));
        assert!(script.contains("aria-label"));
    }
```

- [ ] **Step 2: Run the focused test and confirm RED**

Run: `cargo test ui::chat_popout::tests::injection_script_contains_popup_and_deduplication_contracts -- --nocapture`

Expected: compilation fails because `get_chat_popout_script` is not defined.

- [ ] **Step 3: Implement the complete injected script**

Add this function before the test module:

```rust
/// JavaScript that adds an Open-in-new-window action to each Teams Chat row.
pub fn get_chat_popout_script() -> String {
    r#"
    (function() {
        'use strict';

        const READY_ATTR = 'data-rteams-chat-popout-ready';
        const ROW_CLASS = 'rteams-chat-popout-row';
        const BUTTON_CLASS = 'rteams-chat-popout-button';
        const ROW_SELECTORS = [
            '[data-tid="chat-item"]',
            '[data-tid^="chat-item-"]',
            '[role="listitem"][data-tid*="chat-item"]',
            '[role="option"][data-tid*="chat-item"]'
        ];

        function isTeamsChatUrl(value) {
            try {
                const url = new URL(value, location.origin);
                const host = url.hostname.toLowerCase();
                const trusted = host === 'teams.microsoft.com'
                    || host.endsWith('.teams.microsoft.com')
                    || host === 'teams.live.com';
                if (!trusted) return false;

                const path = url.pathname.toLowerCase();
                if (path.includes('/l/channel/')
                    || path.includes('/meet/')
                    || path.includes('/call/')
                    || path.includes('meetup-join')) {
                    return false;
                }
                return path.includes('/l/chat/')
                    || path.includes('/chat/')
                    || url.searchParams.get('ctx') === 'chat'
                    || url.searchParams.has('users');
            } catch (_) {
                return false;
            }
        }

        function findChatUrl(row) {
            const anchors = [];
            if (row.matches('a[href]')) anchors.push(row);
            anchors.push(...row.querySelectorAll('a[href]'));
            for (const anchor of anchors) {
                const value = anchor.href || anchor.getAttribute('href');
                if (value && isTeamsChatUrl(value)) {
                    return new URL(value, location.origin).href;
                }
            }
            return null;
        }

        function ensureStyle() {
            if (document.getElementById('rteams-chat-popout-style')) return;
            const style = document.createElement('style');
            style.id = 'rteams-chat-popout-style';
            style.textContent = `
                .${ROW_CLASS} { position: relative !important; }
                .${BUTTON_CLASS} {
                    position: absolute;
                    right: 8px;
                    top: 50%;
                    transform: translateY(-50%);
                    width: 28px;
                    height: 28px;
                    display: inline-flex;
                    align-items: center;
                    justify-content: center;
                    padding: 0;
                    border: 0;
                    border-radius: 4px;
                    color: currentColor;
                    background: var(--colorNeutralBackground1, #fff);
                    opacity: 0;
                    pointer-events: none;
                    cursor: pointer;
                    z-index: 2;
                    transition: opacity 120ms ease, background-color 120ms ease;
                }
                .${ROW_CLASS}:hover .${BUTTON_CLASS},
                .${ROW_CLASS}:focus-within .${BUTTON_CLASS} {
                    opacity: 1;
                    pointer-events: auto;
                }
                .${BUTTON_CLASS}:hover,
                .${BUTTON_CLASS}:focus-visible {
                    background: var(--colorNeutralBackground1Hover, #f0f0f0);
                    outline: 2px solid var(--colorBrandStroke1, #6264a7);
                    outline-offset: 1px;
                }
                .${BUTTON_CLASS} svg { width: 16px; height: 16px; }
            `;
            document.head.appendChild(style);
        }

        function decorateRow(row) {
            if (!(row instanceof Element) || row.hasAttribute(READY_ATTR)) return;
            if (!findChatUrl(row)) {
                if (!row.hasAttribute('data-rteams-chat-popout-warning')) {
                    row.setAttribute('data-rteams-chat-popout-warning', 'true');
                    console.warn('[R Teams] Chat row has no supported URL', row);
                }
                return;
            }

            row.setAttribute(READY_ATTR, 'true');
            row.classList.add(ROW_CLASS);

            const button = document.createElement('button');
            button.type = 'button';
            button.className = BUTTON_CLASS;
            button.title = 'Open in new window';
            button.setAttribute('aria-label', 'Open chat in new window');
            button.innerHTML = '<svg viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 5h5v5"/><path d="M10 14 19 5"/><path d="M19 13v6H5V5h6"/></svg>';

            const stopRowAction = (event) => event.stopPropagation();
            button.addEventListener('pointerdown', stopRowAction);
            button.addEventListener('mousedown', stopRowAction);
            button.addEventListener('click', (event) => {
                event.preventDefault();
                event.stopPropagation();
                const chatUrl = findChatUrl(row);
                if (!chatUrl) {
                    console.warn('[R Teams] Chat URL is no longer available');
                    return;
                }
                window.open(chatUrl, '_blank', 'popup=yes');
            });
            row.appendChild(button);
        }

        function decorateChats() {
            if (!document.body) return;
            ensureStyle();
            document.querySelectorAll(ROW_SELECTORS.join(',')).forEach(decorateRow);
        }

        let timer = null;
        const observer = new MutationObserver(() => {
            clearTimeout(timer);
            timer = setTimeout(decorateChats, 100);
        });

        function init() {
            if (!document.body || !document.head) {
                setTimeout(init, 100);
                return;
            }
            decorateChats();
            observer.observe(document.body, { childList: true, subtree: true });
        }

        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', init, { once: true });
        } else {
            init();
        }
    })();
    "#
    .to_string()
}
```

- [ ] **Step 4: Run all Chat pop-out tests**

Run: `cargo test ui::chat_popout::tests -- --nocapture`

Expected: all three tests pass.

### Task 3: Reusable Secondary Chat Window

**Files:**
- Create: `src/ui/chat_window.rs`
- Delete: `src/ui/meeting_window.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Replace the module/event declarations before adding the implementation**

In `src/ui/mod.rs`, replace:

```rust
pub mod meeting_window;
```

with:

```rust
pub mod chat_window;
```

Replace the custom event with:

```rust
/// Custom events sent from WebView callbacks to the main event loop.
#[derive(Debug, Clone)]
pub enum AppEvent {
    OpenChat(String),
}
```

Delete `src/ui/meeting_window.rs`.

- [ ] **Step 2: Run `cargo check` and confirm RED**

Run: `cargo check`

Expected: compilation fails because `src/ui/chat_window.rs` does not exist and `main.rs` still references `MeetingWindow`/`OpenMeeting`.

- [ ] **Step 3: Add the native window wrapper**

Create `src/ui/chat_window.rs`:

```rust
//! One reusable secondary window for a Teams chat.

use std::error::Error;

use tao::dpi::LogicalSize;
use tao::event_loop::EventLoopWindowTarget;
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

/// A secondary Teams window that can be navigated between chats.
pub struct ChatWindow {
    window: tao::window::Window,
    webview: wry::WebView,
}

impl ChatWindow {
    /// Build a secondary window with an already configured WebView builder.
    pub fn create(
        event_loop: &EventLoopWindowTarget<super::AppEvent>,
        builder: WebViewBuilder<'static>,
    ) -> Result<Self, Box<dyn Error>> {
        let window = WindowBuilder::new()
            .with_title("R Teams Chat")
            .with_inner_size(LogicalSize::new(900.0, 700.0))
            .build(event_loop)
            .map_err(|error| -> Box<dyn Error> {
                format!("Failed to create chat window: {error}").into()
            })?;
        let webview = builder
            .build(&window)
            .map_err(|error| -> Box<dyn Error> {
                format!("Failed to create chat WebView: {error}").into()
            })?;
        Ok(Self { window, webview })
    }

    /// Navigate the retained window to another chat and bring it to the front.
    pub fn navigate_and_focus(&self, url: &str) -> wry::Result<()> {
        self.webview.load_url(url)?;
        self.window.set_visible(true);
        self.window.set_focus();
        Ok(())
    }

    pub fn window_id(&self) -> tao::window::WindowId {
        self.window.id()
    }
}
```

- [ ] **Step 4: Defer the expected main.rs errors to Task 4**

Run: `cargo check`

Expected: `chat_window.rs` itself compiles; remaining errors only reference the old `MeetingWindow` import and `AppEvent::OpenMeeting` uses in `main.rs`.

### Task 4: Wire Popup Routing and Single-window Lifecycle

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Update imports and add the request-routing helper**

Change the relevant imports to:

```rust
use tao::event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy};
use wry::{
    NewWindowFeatures, NewWindowResponse, WebViewBuilder, WebViewBuilderExtWindows,
    WebViewExtWindows,
};

use ui::AppEvent;
use ui::chat_popout::{get_chat_popout_script, is_teams_chat_url};
use ui::chat_window::ChatWindow;
```

Remove the `MeetingWindow` import. Add this helper before `main()`:

```rust
fn handle_new_window_request(
    url: String,
    proxy: &EventLoopProxy<AppEvent>,
) -> NewWindowResponse {
    log::info!("Intercepted navigation: {url}");

    if is_teams_chat_url(&url) {
        log::info!("Opening Teams chat in the secondary window: {url}");
        if let Err(error) = proxy.send_event(AppEvent::OpenChat(url)) {
            log::error!("Failed to queue secondary chat window: {error}");
        }
        return NewWindowResponse::Deny;
    }

    let lower = url.to_lowercase();
    let browser = BROWSER_PATH
        .get()
        .and_then(|value| value.lock().ok())
        .and_then(|value| value.clone());
    let is_teams_internal = lower.contains("teams.microsoft.com")
        || lower.contains("teams.live.com");
    let is_popout = lower.contains("/l/person/")
        || lower.contains("/l/channel/");

    if is_teams_internal && is_popout {
        log::info!("Routing Teams pop-out to a new Edge window: {url}");
        if let Err(error) = open_in_new_window(&url) {
            log::warn!("Failed to open in a new window: {error}");
            let _ = open_url_smart(&url, browser.as_deref());
        }
        return NewWindowResponse::Deny;
    }

    if lower.contains("/meet/")
        || lower.contains("/call/")
        || lower.contains("meetup-join")
        || lower.contains("teams.live.com/meet")
    {
        log::info!("Routing meet/call URL with the existing browser behavior: {url}");
        if let Err(error) = open_url_smart(&url, browser.as_deref()) {
            log::warn!("Failed to open meet URL: {error}");
        }
        return NewWindowResponse::Deny;
    }

    if is_teams_internal {
        NewWindowResponse::Allow
    } else {
        if let Err(error) = open_url_smart(&url, browser.as_deref()) {
            log::warn!("Failed to open URL: {error}");
        }
        NewWindowResponse::Deny
    }
}
```

- [ ] **Step 2: Install the script and shared popup handler in the main WebView**

Near the other initialization scripts, add:

```rust
let chat_popout_js = get_chat_popout_script();
```

Add it to the main builder:

```rust
.with_initialization_script(&chat_popout_js)
```

Replace the existing inline `.with_new_window_req_handler(...)` block with:

```rust
let proxy_for_popouts = proxy.clone();

// In the WebViewBuilder chain:
.with_new_window_req_handler(move |url: String, _features: NewWindowFeatures| {
    handle_new_window_request(url, &proxy_for_popouts)
})
```

- [ ] **Step 3: Capture the main WebView2 environment before moving the WebView**

Immediately after the main WebView is built and before wrapping it in `Arc`, add:

```rust
#[cfg(target_os = "windows")]
let webview_environment = webview.environment();
```

This exact environment is required by WebView2 for reliable cookie/session sharing.

- [ ] **Step 4: Replace meeting-window tracking with one optional ChatWindow**

Before `event_loop.run`, replace `meeting_windows` with:

```rust
let mut chat_window: Option<ChatWindow> = None;
```

Replace the `Event::UserEvent(AppEvent::OpenMeeting(...))` arm with:

```rust
Event::UserEvent(AppEvent::OpenChat(url)) => {
    let needs_new_window = match chat_window.as_ref() {
        Some(window) => match window.navigate_and_focus(&url) {
            Ok(()) => false,
            Err(error) => {
                log::warn!("Failed to navigate the secondary chat window: {error}");
                true
            }
        },
        None => true,
    };

    if needs_new_window {
        chat_window = None;
        let proxy_for_secondary = proxy.clone();
        let builder = WebViewBuilder::new()
            .with_url(&url)
            .with_initialization_script(&chat_popout_js)
            .with_new_window_req_handler(
                move |url: String, _features: NewWindowFeatures| {
                    handle_new_window_request(url, &proxy_for_secondary)
                },
            );
        #[cfg(target_os = "windows")]
        let builder = builder.with_environment(webview_environment.clone());

        match ChatWindow::create(event_loop, builder) {
            Ok(window) => {
                window.navigate_and_focus(&url).ok();
                chat_window = Some(window);
            }
            Err(error) => log::error!("Failed to create secondary chat window: {error}"),
        }
    }
}
```

- [ ] **Step 5: Make close/destroy handling window-specific**

Replace the non-main branch of `CloseRequested` with:

```rust
} else if chat_window
    .as_ref()
    .is_some_and(|window| window.window_id() == window_id)
{
    log::info!("Secondary chat window closed");
    chat_window = None;
}
```

Replace the unconditional `Destroyed` arm with:

```rust
Event::WindowEvent {
    event: WindowEvent::Destroyed,
    window_id,
    ..
} if window_id == main_window_id => {
    *control_flow = ControlFlow::Exit;
}
```

This prevents destroying the secondary window from terminating the whole app.

- [ ] **Step 6: Format and run the focused tests**

Run: `cargo fmt --all -- --check`

Expected: exit code 0. If formatting differs, run `cargo fmt --all`, then rerun the check.

Run: `cargo test ui::chat_popout::tests -- --nocapture`

Expected: all Chat pop-out tests pass.

- [ ] **Step 7: Run compile verification**

Run: `cargo check --workspace`

Expected: exit code 0 with no references to `MeetingWindow` or `OpenMeeting`.

### Task 5: Full Verification and Manual Acceptance

**Files:**
- Verify only; no planned source edits

- [ ] **Step 1: Run all automated tests**

Run: `cargo test --workspace`

Expected: all workspace tests pass.

- [ ] **Step 2: Build the release binaries**

Run: `cargo build --release --workspace`

Expected: both `rust_teams` and `rteams-meeting-assistant` build successfully.

- [ ] **Step 3: Check the final diff for accidental changes**

Run: `git diff --check`

Expected: no whitespace errors.

Run: `git status --short`

Expected: only the pre-existing worktree changes plus the approved Chat pop-out implementation/spec/plan are listed; no generated binaries or secrets are added.

- [ ] **Step 4: Perform Windows smoke testing**

Run: `cargo run --release --bin rust_teams`

Verify manually:

1. Hovering any one-to-one or group Chat row shows exactly one pop-out action.
2. Clicking the action leaves the main conversation unchanged.
3. The secondary Rust Teams window opens directly to that chat without login.
4. Clicking the action for a second chat navigates and focuses the existing secondary window.
5. No third Rust Teams window appears.
6. Closing the secondary window does not close the main window, and another chat can reopen it.
7. Scrolling the virtualized Chat list does not create duplicate buttons.
8. Channel, meeting, and external-link behavior remains unchanged.

- [ ] **Step 5: Report residual manual-only risk**

Record whether Teams' current production DOM exposes usable anchors in all Chat-row variants. If Microsoft renders a row without an anchor, the script intentionally omits the action and logs `[R Teams] Chat URL is no longer available`; capture that row's DOM before changing selectors.
