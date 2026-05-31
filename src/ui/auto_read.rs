//! Auto-read messages module - injects JavaScript to mark messages as read

/// JavaScript to auto-read messages containing specific keywords
/// Optimized for smooth performance with debouncing and deduplication
pub fn get_auto_read_script() -> String {
    r#"
    (function() {
        'use strict';
        
        // Configuration
        const CONFIG = {
            keywords: ['closed', 'cancel'],
            checkInterval: 300000,     // Check every 5 minutes (300000ms)
            debounceMs: 500,          // Debounce clicks
            maxClicksPerBatch: 5,     // Limit clicks per batch
            selectorTimeout: 2000     // Wait for elements to load
        };
        
        // State tracking
        const state = {
            clickedElements: new WeakSet(),
            isRunning: false,
            lastRun: 0,
            debounceTimers: new Map()
        };
        
        // Debounce function to prevent rapid clicking
        function debounce(key, fn, delay) {
            if (state.debounceTimers.has(key)) {
                clearTimeout(state.debounceTimers.get(key));
            }
            state.debounceTimers.set(key, setTimeout(() => {
                fn();
                state.debounceTimers.delete(key);
            }, delay));
        }
        
        // Check if message text contains any keyword
        function shouldAutoRead(text) {
            const lowerText = text.toLowerCase().trim();
            return CONFIG.keywords.some(keyword => lowerText.includes(keyword));
        }
        
        // Safe click with tracking
        function safeClick(element, description) {
            if (state.clickedElements.has(element)) {
                return false; // Already clicked
            }
            
            state.clickedElements.add(element);
            
            // Use dispatchEvent for smoother interaction
            const clickEvent = new MouseEvent('click', {
                view: window,
                bubbles: true,
                cancelable: true
            });
            element.dispatchEvent(clickEvent);
            
            console.log('[AutoRead] ✓', description);
            return true;
        }
        
        // Main auto-read function with batch processing
        function autoReadMessages() {
            // Prevent concurrent runs
            if (state.isRunning) return;
            
            // Throttle execution
            const now = Date.now();
            if (now - state.lastRun < CONFIG.debounceMs) return;
            
            state.isRunning = true;
            state.lastRun = now;
            
            let clickCount = 0;
            
            try {
                // Priority 1: Find unread chat items with keywords
                const chatItems = document.querySelectorAll(
                    '[class*="chat-list"] [class*="item"],' +
                    '[class*="conversation"] [class*="item"],' +
                    '[data-tid*="chat-item"],' +
                    '[class*="thread-list"] [class*="item"]'
                );
                
                for (const item of chatItems) {
                    if (clickCount >= CONFIG.maxClicksPerBatch) break;
                    
                    // Check if already marked as read
                    if (item.getAttribute('data-auto-read') === 'true') continue;
                    
                    const text = item.textContent || '';
                    if (shouldAutoRead(text)) {
                        item.setAttribute('data-auto-read', 'true');
                        if (safeClick(item, 'Chat: ' + text.substring(0, 30))) {
                            clickCount++;
                        }
                    }
                }
                
                // Priority 2: Find unread indicators/badges
                const unreadIndicators = document.querySelectorAll(
                    '[class*="unread-count"],' +
                    '[class*="badge"][class*="count"],' +
                    '[data-unread="true"]'
                );
                
                for (const indicator of unreadIndicators) {
                    if (clickCount >= CONFIG.maxClicksPerBatch) break;
                    
                    // Find the parent clickable element
                    const parentItem = indicator.closest(
                        '[role="listitem"],' +
                        '[class*="item"],' +
                        '[class*="chat"],' +
                        'button'
                    );
                    
                    if (parentItem && !state.clickedElements.has(parentItem)) {
                        const text = parentItem.textContent || '';
                        if (shouldAutoRead(text)) {
                            if (safeClick(parentItem, 'Badge: ' + text.substring(0, 30))) {
                                clickCount++;
                            }
                        }
                    }
                }
                
                // Priority 3: Find notification badges in sidebar
                const sidebarBadges = document.querySelectorAll(
                    '.app-bar [class*="badge"],' +
                    '[class*="nav-item"] [class*="badge"],' +
                    '[role="tab"] [class*="count"]'
                );
                
                for (const badge of sidebarBadges) {
                    if (clickCount >= CONFIG.maxClicksPerBatch) break;
                    
                    const count = parseInt(badge.textContent || '0');
                    if (count > 0) {
                        const tab = badge.closest('[role="tab"], [class*="nav-item"], button');
                        if (tab) {
                            debounce('sidebar-' + tab.id, () => {
                                safeClick(tab, 'Sidebar badge: ' + count);
                            }, CONFIG.debounceMs);
                        }
                    }
                }
                
            } catch (error) {
                console.error('[AutoRead] Error:', error);
            } finally {
                state.isRunning = false;
            }
        }
        
        // Optimized MutationObserver with batching
        let mutationTimeout = null;
        const observer = new MutationObserver((mutations) => {
            // Batch mutations for better performance
            if (mutationTimeout) clearTimeout(mutationTimeout);
            mutationTimeout = setTimeout(() => {
                autoReadMessages();
            }, 100); // Wait 100ms for DOM to settle
        });
        
        // Start observing when body is ready
        function startObserver() {
            if (document.body) {
                observer.observe(document.body, {
                    childList: true,
                    subtree: true,
                    attributes: false,  // Don't watch attributes for performance
                    characterData: false // Don't watch text changes
                });
                console.log('[AutoRead] ✓ Observer started');
            } else {
                setTimeout(startObserver, 500);
            }
        }
        
        // Initialize
        function init() {
            console.log('[AutoRead] Initializing with keywords:', CONFIG.keywords);
            
            // Initial run after page loads
            setTimeout(() => {
                autoReadMessages();
                startObserver();
            }, CONFIG.selectorTimeout);
            
            // Periodic check (backup)
            setInterval(autoReadMessages, CONFIG.checkInterval);
        }
        
        // Start when DOM is ready
        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', init);
        } else {
            init();
        }
        
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

/// Add a new keyword to auto-read list (for future use with config)
#[allow(dead_code)]
pub fn get_auto_read_script_with_keywords(keywords: &[String]) -> String {
    let keywords_json = serde_json::to_string(keywords).unwrap_or_default();
    
    format!(
        r#"
        (function() {{
            'use strict';
            
            const CONFIG = {{
                keywords: {},
                checkInterval: 3000,
                debounceMs: 500,
                maxClicksPerBatch: 5,
                selectorTimeout: 2000
            }};
            
            const state = {{
                clickedElements: new WeakSet(),
                isRunning: false,
                lastRun: 0,
                debounceTimers: new Map()
            }};
            
            function debounce(key, fn, delay) {{
                if (state.debounceTimers.has(key)) {{
                    clearTimeout(state.debounceTimers.get(key));
                }}
                state.debounceTimers.set(key, setTimeout(() => {{
                    fn();
                    state.debounceTimers.delete(key);
                }}, delay));
            }}
            
            function shouldAutoRead(text) {{
                const lowerText = text.toLowerCase().trim();
                return CONFIG.keywords.some(keyword => lowerText.includes(keyword));
            }}
            
            function safeClick(element, description) {{
                if (state.clickedElements.has(element)) return false;
                state.clickedElements.add(element);
                
                const clickEvent = new MouseEvent('click', {{
                    view: window,
                    bubbles: true,
                    cancelable: true
                }});
                element.dispatchEvent(clickEvent);
                console.log('[AutoRead] ✓', description);
                return true;
            }}
            
            function autoReadMessages() {{
                if (state.isRunning) return;
                const now = Date.now();
                if (now - state.lastRun < CONFIG.debounceMs) return;
                
                state.isRunning = true;
                state.lastRun = now;
                let clickCount = 0;
                
                try {{
                    const chatItems = document.querySelectorAll(
                        '[class*="chat-list"] [class*="item"],' +
                        '[class*="conversation"] [class*="item"],' +
                        '[data-tid*="chat-item"]'
                    );
                    
                    for (const item of chatItems) {{
                        if (clickCount >= CONFIG.maxClicksPerBatch) break;
                        if (item.getAttribute('data-auto-read') === 'true') continue;
                        
                        const text = item.textContent || '';
                        if (shouldAutoRead(text)) {{
                            item.setAttribute('data-auto-read', 'true');
                            if (safeClick(item, 'Chat: ' + text.substring(0, 30))) {{
                                clickCount++;
                            }}
                        }}
                    }}
                    
                    const unreadIndicators = document.querySelectorAll(
                        '[class*="unread-count"],' +
                        '[class*="badge"][class*="count"]'
                    );
                    
                    for (const indicator of unreadIndicators) {{
                        if (clickCount >= CONFIG.maxClicksPerBatch) break;
                        
                        const parentItem = indicator.closest(
                            '[role="listitem"], [class*="item"], button'
                        );
                        
                        if (parentItem && !state.clickedElements.has(parentItem)) {{
                            const text = parentItem.textContent || '';
                            if (shouldAutoRead(text)) {{
                                if (safeClick(parentItem, 'Badge: ' + text.substring(0, 30))) {{
                                    clickCount++;
                                }}
                            }}
                        }}
                    }}
                    
                }} catch (error) {{
                    console.error('[AutoRead] Error:', error);
                }} finally {{
                    state.isRunning = false;
                }}
            }}
            
            let mutationTimeout = null;
            const observer = new MutationObserver((mutations) => {{
                if (mutationTimeout) clearTimeout(mutationTimeout);
                mutationTimeout = setTimeout(autoReadMessages, 100);
            }});
            
            function startObserver() {{
                if (document.body) {{
                    observer.observe(document.body, {{
                        childList: true,
                        subtree: true,
                        attributes: false,
                        characterData: false
                    }});
                }} else {{
                    setTimeout(startObserver, 500);
                }}
            }}
            
            function init() {{
                console.log('[AutoRead] Initialized with keywords:', CONFIG.keywords);
                setTimeout(() => {{
                    autoReadMessages();
                    startObserver();
                }}, CONFIG.selectorTimeout);
                setInterval(autoReadMessages, CONFIG.checkInterval);
            }}
            
            if (document.readyState === 'loading') {{
                document.addEventListener('DOMContentLoaded', init);
            }} else {{
                init();
            }}
            
        }})();
        "#,
        keywords_json
    )
}
