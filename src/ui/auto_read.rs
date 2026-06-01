//! Auto-read messages module - injects JavaScript to mark messages as read
//! Strategy: read notification preview text via data-tid selectors

/// JavaScript to auto-read messages containing specific keywords
/// Scoped to preview-text elements (last message in each chat)
pub fn get_auto_read_script() -> String {
    r#"
    (function() {
        'use strict';
        
        const CONFIG = {
            keywords: ['closed', 'cancel'],
            checkInterval: 60000,     // 60s
            debounceMs: 1000,
            maxClicksPerBatch: 10
        };
        
        const state = {
            clickedChats: new WeakSet(),
            isRunning: false,
            lastRun: 0
        };
        
        function hasKeyword(text) {
            if (!text) return false;
            const lower = text.toLowerCase();
            return CONFIG.keywords.some(kw => lower.includes(kw));
        }
        
        function getMessageOnly(previewText) {
            const idx = previewText.indexOf(':');
            if (idx > 0 && idx < 50) {
                return previewText.substring(idx + 1).trim();
            }
            return previewText.trim();
        }
        
        function autoRead() {
            if (state.isRunning) return;
            const now = Date.now();
            if (now - state.lastRun < CONFIG.debounceMs) return;
            
            state.isRunning = true;
            state.lastRun = now;
            let clicked = 0;
            
            try {
                const previews = document.querySelectorAll(
                    '[data-tid="chat-item-preview-text"],' +
                    '[data-tid*="preview-text"],' +
                    '[data-tid*="last-message"]'
                );
                
                for (const preview of previews) {
                    if (clicked >= CONFIG.maxClicksPerBatch) break;
                    
                    const rawText = preview.textContent || '';
                    const message = getMessageOnly(rawText);
                    if (!hasKeyword(message)) continue;
                    
                    const option = preview.closest(
                        '[role="option"],' +
                        '[data-tid*="chat-item"]'
                    );
                    if (!option) continue;
                    if (state.clickedChats.has(option)) continue;
                    
                    const unread = option.querySelector(
                        '[data-tid*="unread"],' +
                        '[aria-label*="unread" i],' +
                        '[class*="unread"]'
                    );
                    if (!unread) continue;
                    
                    option.click();
                    state.clickedChats.add(option);
                    option.setAttribute('data-auto-read', 'true');
                    
                    console.log('[AutoRead] ✓', message.substring(0, 50));
                    clicked++;
                }
                
                if (clicked > 0) {
                    console.log('[AutoRead] Processed ' + clicked + ' chats');
                }
            } catch (err) {
                console.error('[AutoRead]', err);
            } finally {
                state.isRunning = false;
            }
        }
        
        const obs = new MutationObserver(() => {
            clearTimeout(window._arT);
            window._arT = setTimeout(autoRead, 500);
        });
        
        function init() {
            if (!document.body) {
                setTimeout(init, 500);
                return;
            }
            obs.observe(document.body, {childList: true, subtree: true});
            setTimeout(autoRead, 3000);
            setInterval(autoRead, CONFIG.checkInterval);
            console.log('[AutoRead] Active. Keywords:', CONFIG.keywords);
        }
        
        init();
    })();
    "#
    .to_string()
}

/// Get the list of keywords being monitored
#[allow(dead_code)]
pub fn get_keywords() -> Vec<String> {
    vec![
        "closed".to_string(),
        "cancel".to_string(),
    ]
}
