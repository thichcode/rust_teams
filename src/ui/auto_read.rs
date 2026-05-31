//! Auto-read messages module - injects JavaScript to mark messages as read

/// JavaScript to auto-read messages containing specific keywords
/// Updated selectors for current Teams UI
pub fn get_auto_read_script() -> String {
    r#"
    (function() {
        'use strict';
        
        console.log('[AutoRead] Starting initialization...');
        
        // Configuration
        const CONFIG = {
            keywords: ['closed', 'cancel'],
            checkInterval: 300000,     // Check every 5 minutes
            debounceMs: 1000,         // Debounce clicks (1s)
            maxClicksPerBatch: 3,     // Limit clicks per batch
            selectorTimeout: 3000     // Wait for elements to load
        };
        
        // State tracking
        const state = {
            clickedElements: new WeakSet(),
            isRunning: false,
            lastRun: 0
        };
        
        // Check if text contains any keyword
        function shouldAutoRead(text) {
            if (!text) return false;
            const lowerText = text.toLowerCase().trim();
            const result = CONFIG.keywords.some(keyword => lowerText.includes(keyword));
            if (result) {
                console.log('[AutoRead] Found keyword in:', lowerText.substring(0, 50));
            }
            return result;
        }
        
        // Safe click with tracking
        function safeClick(element, description) {
            if (state.clickedElements.has(element)) {
                return false;
            }
            
            state.clickedElements.add(element);
            
            // Try multiple click methods
            try {
                // Method 1: Native click
                element.click();
            } catch (e) {
                // Method 2: Dispatch event
                const clickEvent = new MouseEvent('click', {
                    view: window,
                    bubbles: true,
                    cancelable: true
                });
                element.dispatchEvent(clickEvent);
            }
            
            console.log('[AutoRead] ✓ Clicked:', description);
            return true;
        }
        
        // Find and click unread items with keywords
        function autoReadMessages() {
            if (state.isRunning) return;
            
            const now = Date.now();
            if (now - state.lastRun < CONFIG.debounceMs) return;
            
            state.isRunning = true;
            state.lastRun = now;
            
            let clickCount = 0;
            
            try {
                // Strategy 1: Find all elements with text content
                const allElements = document.querySelectorAll('*');
                
                for (const el of allElements) {
                    if (clickCount >= CONFIG.maxClicksPerBatch) break;
                    
                    // Skip if already processed
                    if (el.getAttribute('data-auto-read') === 'true') continue;
                    
                    // Get text content (only from leaf nodes)
                    if (el.children.length > 0) continue;
                    
                    const text = el.textContent || '';
                    if (!text || text.length < 3) continue;
                    
                    if (shouldAutoRead(text)) {
                        // Find clickable parent
                        let clickable = el.closest('[role="button"], [role="listitem"], [role="tab"], button, a, [class*="item"], [class*="chat"]');
                        
                        if (clickable && !state.clickedElements.has(clickable)) {
                            clickable.setAttribute('data-auto-read', 'true');
                            if (safeClick(clickable, text.substring(0, 40))) {
                                clickCount++;
                            }
                        }
                    }
                }
                
                // Strategy 2: Find unread badges/indicators
                const badges = document.querySelectorAll('[class*="badge"], [class*="unread"], [class*="count"]');
                
                for (const badge of badges) {
                    if (clickCount >= CONFIG.maxClicksPerBatch) break;
                    
                    const count = parseInt(badge.textContent || '0');
                    if (count > 0) {
                        // Find parent list item
                        const parent = badge.closest('[role="listitem"], [role="tab"], [class*="item"], [class*="chat"]');
                        if (parent && !state.clickedElements.has(parent)) {
                            const text = parent.textContent || '';
                            if (shouldAutoRead(text)) {
                                parent.setAttribute('data-auto-read', 'true');
                                if (safeClick(parent, 'Badge: ' + text.substring(0, 30))) {
                                    clickCount++;
                                }
                            }
                        }
                    }
                }
                
                if (clickCount > 0) {
                    console.log('[AutoRead] Processed', clickCount, 'items');
                }
                
            } catch (error) {
                console.error('[AutoRead] Error:', error);
            } finally {
                state.isRunning = false;
            }
        }
        
        // Observer for DOM changes
        let mutationTimeout = null;
        const observer = new MutationObserver((mutations) => {
            if (mutationTimeout) clearTimeout(mutationTimeout);
            mutationTimeout = setTimeout(autoReadMessages, 500);
        });
        
        // Start observing
        function startObserver() {
            if (document.body) {
                observer.observe(document.body, {
                    childList: true,
                    subtree: true
                });
                console.log('[AutoRead] ✓ Observer started');
            } else {
                setTimeout(startObserver, 1000);
            }
        }
        
        // Initialize
        function init() {
            console.log('[AutoRead] Initializing with keywords:', CONFIG.keywords);
            
            // Run after page loads
            setTimeout(() => {
                console.log('[AutoRead] Running initial check...');
                autoReadMessages();
                startObserver();
            }, CONFIG.selectorTimeout);
            
            // Periodic check
            setInterval(autoReadMessages, CONFIG.checkInterval);
        }
        
        // Start
        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', init);
        } else {
            init();
        }
        
    })();
    "#
    .to_string()
}
