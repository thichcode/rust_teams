# Chat Pop-out in a Second Rust Teams Window

**Date:** 2026-07-20
**Status:** Approved design

## Problem

Rust Teams cannot keep two chats visible at the same time inside the app. Chat
rows do not expose a convenient app-controlled action for opening a conversation
in a second Rust Teams window, and the existing pop-out routing opens Teams links
in Edge.

## Goal

Add an `Open in new window` action to every Chat row. The action opens that chat
in one secondary Rust Teams window while leaving the main window unchanged. Both
windows share the same Teams login session, and the application never has more
than two windows in total.

## Scope

- Support one-to-one and group chats shown in the Teams Chat list.
- Add the action to Chat rows only, not channels or meeting links.
- Reuse the single secondary window when another chat is opened.
- Keep existing behavior for non-chat links and meeting links.
- Do not add configuration or support more than one secondary window.

## Architecture

### Chat-row injection

A new `src/ui/chat_popout.rs` module provides an initialization script for the
main and secondary WebViews. The script observes the Teams Chat list because
Teams renders and recycles rows dynamically.

For each supported Chat row, the script:

1. Finds the row's chat anchor and resolves it against `location.origin`.
2. Rejects URLs outside the Teams domains or outside supported chat routes.
3. Adds one pop-out button, guarded by a stable data attribute to prevent
   duplicates after DOM updates.
4. Shows the button only while the row is hovered or keyboard-focused.
5. Adds an accessible label and tooltip.

Clicking the button calls `preventDefault()` and `stopPropagation()` so the main
window does not switch conversations. It then calls
`window.open(chatUrl, "_blank", "popup")`, using the normal Teams pop-out path.

### Rust event bridge

The main and secondary WebViews install a `new_window_req_handler`. A pure URL
classifier identifies supported Teams Chat URLs. For a supported chat URL, the
handler sends `AppEvent::OpenChat(url)` through `EventLoopProxy` and returns
`NewWindowResponse::Deny` so WebView2 does not create an unmanaged window.

Other URLs continue through their existing routing behavior.

### Secondary window lifecycle

A new `src/ui/chat_window.rs` module owns the secondary `tao::Window` and
`wry::WebView`. It replaces the unfinished meeting-specific window draft rather
than adding a second competing multi-window implementation.

The event loop stores `Option<ChatWindow>`:

- `None`: create the secondary window at the requested chat URL.
- `Some`: navigate its WebView to the requested chat URL and focus its window.
- Secondary `CloseRequested`: clear the option and destroy that window.
- Main `CloseRequested`: save the main window state and exit the application,
  which closes both windows.

Both WebViews use the same WebView2 user-data environment, so Teams cookies and
login state are shared. The secondary WebView receives only the initialization
and popup-handling behavior required for normal Teams navigation; main-window
badge ownership remains unchanged.

## Data Flow

1. Teams renders or recycles a Chat row.
2. The observer validates the row's chat URL and injects one hover action.
3. The user clicks the action.
4. JavaScript requests `window.open` for that chat without selecting it in the
   main window.
5. The WebView handler classifies the URL, emits `AppEvent::OpenChat`, and denies
   the unmanaged popup.
6. The event loop creates the secondary window or navigates the existing one.
7. The secondary window is focused and displays the requested chat using the
   shared Teams session.

## Error Handling

- Rows without a valid supported chat URL receive no button; the script writes a
  diagnostic warning instead of opening an incorrect page.
- Failure to send an event or create a WebView is logged without terminating the
  main window.
- If navigation of the retained secondary WebView fails, the stale secondary
  window is discarded and creation is retried once.
- External, channel, and meeting URLs are not accepted by the Chat URL
  classifier and retain their current behavior.

## Testing

### Automated

- Unit-test accepted Teams Chat routes and rejected external, channel, and
  meeting URLs.
- Unit-test the single-secondary-window state transitions where practical
  without requiring a native WebView.
- Run `cargo test`, `cargo check`, and `cargo build --release`.

### Manual acceptance

1. Every supported Chat row shows one action only on hover or keyboard focus.
2. Opening Chat A creates one secondary Rust Teams window without changing the
   conversation displayed in the main window.
3. Opening Chat B navigates and focuses the same secondary window.
4. Repeated actions never produce a third window or duplicate row buttons.
5. Closing the secondary window permits a new secondary window to be opened.
6. The secondary window is already logged in to the same Teams account.
7. Virtualized Chat-list rerenders do not remove actions permanently or create
   duplicate actions.
8. Existing external-link, channel-link, and meeting-link behavior is unchanged.

## Out of Scope

- More than one secondary window.
- Separate accounts or isolated cookie stores.
- Channel-row pop-outs.
- Meeting-window management.
- Persisting the secondary window's size or position.
