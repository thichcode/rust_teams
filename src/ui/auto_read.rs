//! Auto-read messages module - injects JavaScript to mark messages as read.
//!
//! Strategy (v2):
//!   1. Find all chat items in the sidebar that have an unread badge.
//!   2. For each candidate (max N per cycle):
//!      - Click the chat to open it in the right pane (Teams marks it
//!        as read on open).
//!      - Wait for the message pane to render.
//!      - Read the bottom-most visible message bubble.
//!      - If it contains a keyword → confirmed; leave the chat open
//!        with a `data-auto-read="true"` marker.
//!      - Otherwise → click the back button to return to the chat
//!        list without changing the user's view.
//!   3. Skip if the user is actively typing in the message input.
//!   4. Per-chat 5-minute cooldown to avoid repeated clicks.

/// JavaScript injected into the WebView to auto-read messages.
pub fn get_auto_read_script() -> String {
    r#"
    (function() {
        'use strict';

        const CONFIG = {
            keywords: [
                'closed',
                'cancel',
                'đã đóng',
                'đã kết thúc',
                'hoàn tất',
                'cuộc hội thoại đã đóng',
                'đã được đóng',
                'đóng cuộc trò chuyện',
                'kết thúc cuộc trò chuyện',
                'conversation closed',
                'conversation ended',
                'this conversation has been closed',
                'chat closed',
                'chat ended',
            ],
            checkInterval: 30000,            // 30s between cycles
            maxChatsPerCycle: 3,             // limit disruption
            settleDelayMs: 800,              // wait for message pane
            perChatTimeoutMs: 4000,          // give up reading after this
            cooldownMs: 5 * 60 * 1000,       // don't re-check same chat
            pollIntervalMs: 200,             // how often to poll messages
            chatItemSelectors: [
                '[data-tid="chat-item"]',
                '[data-tid^="chat-item-"]',
                '[role="listitem"][data-tid*="chat-item"]',
                '[role="option"][data-tid*="chat-item"]'
            ],
            unreadBadgeSelectors: [
                '[data-tid*="unread-count"]',
                '[data-tid*="unread-badge"]',
                '[aria-label*="unread" i]',
                '[class*="unread-badge" i]',
                '[class*="UnreadBadge"]'
            ],
            lastMessageSelectors: [
                '[data-tid="messageBodyContent"]',
                '[data-tid*="message-body-content" i]',
                '[class*="message-body" i]',
                '[data-tid*="message-text" i]'
            ],
            backButtonSelectors: [
                '[data-tid="chat-header-back"]',
                '[data-tid="header-back"]',
                '[data-tid*="back-button"]',
                'button[aria-label="Back"]',
                'button[aria-label*="back" i]',
                'button[aria-label*="Quay lại" i]'
            ],
            messageInputSelectors: [
                '[data-tid="ckeditor-reply-input"]',
                '[data-tid="message-input"]',
                'div[contenteditable="true"][data-tid*="reply"]',
                'div[contenteditable="true"][aria-label*="Reply" i]',
                'div[contenteditable="true"][aria-label*="reply" i]',
                'textarea[aria-label*="reply" i]'
            ]
        };

        const state = {
            isRunning: false,
            lastRun: 0,
            confirmedChats: new WeakSet(),
            cooldown: new WeakMap()
        };

        const sleep = (ms) => new Promise(r => setTimeout(r, ms));

        function hasKeyword(text) {
            if (!text) return false;
            const lower = text.toLowerCase();
            return CONFIG.keywords.some(kw => lower.includes(kw.toLowerCase()));
        }

        function isInsideDialog(element) {
            if (!element || !element.closest) return false;
            return element.closest(
                '[role="dialog"], [role="alertdialog"], [aria-modal="true"],' +
                '[data-tid*="dialog" i], [data-tid*="popup" i],' +
                '[data-tid*="modal" i], [data-tid*="call-stage" i],' +
                '[data-tid*="meeting" i]'
            ) !== null;
        }

        function isUserTyping() {
            const input = document.querySelector(
                CONFIG.messageInputSelectors.join(',')
            );
            if (!input) return false;
            if (document.activeElement === input) return true;
            const text = (input.textContent || input.value || '').trim();
            return text.length > 0;
        }

        function findUnreadChats() {
            const items = document.querySelectorAll(
                CONFIG.chatItemSelectors.join(',')
            );
            const result = [];
            for (const item of items) {
                if (isInsideDialog(item)) continue;
                if (state.confirmedChats.has(item)) continue;
                if (!item.querySelector(
                    CONFIG.unreadBadgeSelectors.join(',')
                )) continue;

                const lastCheck = state.cooldown.get(item);
                if (lastCheck && (Date.now() - lastCheck) < CONFIG.cooldownMs) {
                    continue;
                }
                result.push(item);
            }
            return result;
        }

        function getLastMessageText() {
            const messages = document.querySelectorAll(
                CONFIG.lastMessageSelectors.join(',')
            );
            for (let i = messages.length - 1; i >= 0; i--) {
                const m = messages[i];
                if (isInsideDialog(m)) continue;
                const rect = m.getBoundingClientRect();
                if (rect.width > 0 && rect.height > 0) {
                    return (m.textContent || '').trim();
                }
            }
            return null;
        }

        function findBackButton() {
            for (const sel of CONFIG.backButtonSelectors) {
                const btn = document.querySelector(sel);
                if (btn && !isInsideDialog(btn)) return btn;
            }
            return null;
        }

        async function navigateBack() {
            const btn = findBackButton();
            if (btn) {
                btn.click();
                await sleep(300);
                return true;
            }
            // Fallback: synthesize Alt+Left
            try {
                const ev = new KeyboardEvent('keydown', {
                    key: 'ArrowLeft', altKey: true, bubbles: true
                });
                document.dispatchEvent(ev);
                await sleep(300);
                return true;
            } catch (_) {
                return false;
            }
        }

        async function openAndVerify(chatItem) {
            chatItem.setAttribute('data-auto-read-pending', 'true');
            chatItem.click();
            await sleep(CONFIG.settleDelayMs);

            // Poll for at least one visible message bubble
            const start = Date.now();
            let lastText = null;
            while (Date.now() - start < CONFIG.perChatTimeoutMs) {
                lastText = getLastMessageText();
                if (lastText) break;
                await sleep(CONFIG.pollIntervalMs);
            }
            return {
                chatItem,
                lastMessageText: lastText,
                matches: !!(lastText && hasKeyword(lastText))
            };
        }

        async function processChats() {
            if (state.isRunning) return;
            if (isUserTyping()) return;
            const now = Date.now();
            if (now - state.lastRun < 1000) return;

            state.isRunning = true;
            state.lastRun = now;
            try {
                const unread = findUnreadChats();
                if (unread.length === 0) return;

                const batch = unread.slice(0, CONFIG.maxChatsPerCycle);
                console.log('[AutoRead v2] Processing '
                    + batch.length + '/' + unread.length + ' chats');

                let confirmed = 0, dismissed = 0;
                for (const chat of batch) {
                    try {
                        const result = await openAndVerify(chat);
                        if (result.matches) {
                            state.confirmedChats.add(chat);
                            chat.setAttribute('data-auto-read', 'true');
                            const preview = (result.lastMessageText || '')
                                .substring(0, 60).replace(/\s+/g, ' ');
                            console.log('[AutoRead v2] ✓ marked: ' + preview);
                            confirmed++;
                            // Leave the chat open so the user sees
                            // exactly which chat was marked as read.
                        } else {
                            // No keyword in the last actual message —
                            // back out so we don't change the user's
                            // view. Chat is still marked as read by
                            // Teams (because we opened it), but we
                            // avoid spotlighting a false positive.
                            await navigateBack();
                            dismissed++;
                        }
                        state.cooldown.set(chat, Date.now());
                    } catch (err) {
                        console.error('[AutoRead v2]', err);
                    }
                }
                if (confirmed > 0 || dismissed > 0) {
                    console.log('[AutoRead v2] Cycle done — '
                        + confirmed + ' confirmed, '
                        + dismissed + ' dismissed');
                }
            } finally {
                state.isRunning = false;
            }
        }

        const obs = new MutationObserver(() => {
            clearTimeout(window._arT);
            window._arT = setTimeout(processChats, 500);
        });

        function init() {
            if (!document.body) {
                setTimeout(init, 500);
                return;
            }
            obs.observe(document.body, {childList: true, subtree: true});
            setTimeout(processChats, 3000);
            setInterval(processChats, CONFIG.checkInterval);
            console.log('[AutoRead v2] Active. Opens each unread chat, '
                + 'reads the last message, verifies keyword, '
                + 'and clicks back on false positives. '
                + 'Keywords: ' + JSON.stringify(CONFIG.keywords));
        }

        init();
    })();
    "#
    .to_string()
}

/// Get the list of keywords being monitored.
#[allow(dead_code)]
pub fn get_keywords() -> Vec<String> {
    vec![
        "closed".to_string(),
        "cancel".to_string(),
        "đã đóng".to_string(),
        "đã kết thúc".to_string(),
        "hoàn tất".to_string(),
        "cuộc hội thoại đã đóng".to_string(),
        "đã được đóng".to_string(),
        "đóng cuộc trò chuyện".to_string(),
        "kết thúc cuộc trò chuyện".to_string(),
        "conversation closed".to_string(),
        "conversation ended".to_string(),
        "this conversation has been closed".to_string(),
        "chat closed".to_string(),
        "chat ended".to_string(),
    ]
}
