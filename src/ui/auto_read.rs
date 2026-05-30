//! Auto-read messages module - injects JavaScript to mark messages as read

/// JavaScript to auto-read messages containing specific keywords
/// This runs automatically when the page loads
pub fn get_auto_read_script() -> String {
    r#"
    (function() {
        'use strict';
        
        // Keywords to auto-read (case-insensitive)
        const AUTO_READ_KEYWORDS = ['closed', 'cancel'];
        
        // Check if message text contains any keyword
        function shouldAutoRead(text) {
            const lowerText = text.toLowerCase();
            return AUTO_READ_KEYWORDS.some(keyword => lowerText.includes(keyword));
        }
        
        // Find and click unread message indicators
        function autoReadMessages() {
            // Look for unread message indicators in Teams
            // Teams uses various selectors for unread messages
            
            // Method 1: Look for elements with unread indicators
            const unreadElements = document.querySelectorAll(
                '[data-unread="true"], ' +
                '.ts-unread-count, ' +
                '[class*="unread"], ' +
                '[class*="badge"]'
            );
            
            unreadElements.forEach(el => {
                const text = el.textContent || el.innerText || '';
                if (shouldAutoRead(text)) {
                    // Click to mark as read
                    el.click();
                    console.log('[AutoRead] Marked as read:', text.substring(0, 50));
                }
            });
            
            // Method 2: Look for notification badges
            const badges = document.querySelectorAll(
                '.app-bar-badge, ' +
                '[class*="notification-badge"], ' +
                '[class*="count-badge"]'
            );
            
            badges.forEach(badge => {
                const count = parseInt(badge.textContent || '0');
                if (count > 0) {
                    // Find parent clickable element
                    const parent = badge.closest('[role="tab"], button, [class*="nav-item"]');
                    if (parent) {
                        parent.click();
                        console.log('[AutoRead] Clicked notification badge');
                    }
                }
            });
            
            // Method 3: Look for chat list items with unread indicators
            const chatItems = document.querySelectorAll(
                '[class*="chat-list-item"], ' +
                '[class*="conversation-item"], ' +
                '[data-tid*="chat"]'
            );
            
            chatItems.forEach(item => {
                const text = item.textContent || '';
                if (shouldAutoRead(text)) {
                    // Simulate click to mark as read
                    item.click();
                    console.log('[AutoRead] Auto-read chat:', text.substring(0, 50));
                }
            });
        }
        
        // Run immediately
        autoReadMessages();
        
        // Run periodically (every 5 seconds)
        setInterval(autoReadMessages, 5000);
        
        // Run when DOM changes
        const observer = new MutationObserver((mutations) => {
            mutations.forEach(() => {
                autoReadMessages();
            });
        });
        
        observer.observe(document.body, {
            childList: true,
            subtree: true
        });
        
        console.log('[AutoRead] Initialized - watching for messages with:', AUTO_READ_KEYWORDS);
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
            
            const AUTO_READ_KEYWORDS = {};
            
            function shouldAutoRead(text) {{
                const lowerText = text.toLowerCase();
                return AUTO_READ_KEYWORDS.some(keyword => lowerText.includes(keyword));
            }}
            
            function autoReadMessages() {{
                const unreadElements = document.querySelectorAll(
                    '[data-unread="true"], ' +
                    '.ts-unread-count, ' +
                    '[class*="unread"], ' +
                    '[class*="badge"]'
                );
                
                unreadElements.forEach(el => {{
                    const text = el.textContent || el.innerText || '';
                    if (shouldAutoRead(text)) {{
                        el.click();
                        console.log('[AutoRead] Marked as read:', text.substring(0, 50));
                    }}
                }});
                
                const badges = document.querySelectorAll(
                    '.app-bar-badge, ' +
                    '[class*="notification-badge"], ' +
                    '[class*="count-badge"]'
                );
                
                badges.forEach(badge => {{
                    const count = parseInt(badge.textContent || '0');
                    if (count > 0) {{
                        const parent = badge.closest('[role="tab"], button, [class*="nav-item"]');
                        if (parent) {{
                            parent.click();
                            console.log('[AutoRead] Clicked notification badge');
                        }}
                    }}
                }});
                
                const chatItems = document.querySelectorAll(
                    '[class*="chat-list-item"], ' +
                    '[class*="conversation-item"], ' +
                    '[data-tid*="chat"]'
                );
                
                chatItems.forEach(item => {{
                    const text = item.textContent || '';
                    if (shouldAutoRead(text)) {{
                        item.click();
                        console.log('[AutoRead] Auto-read chat:', text.substring(0, 50));
                    }}
                }});
            }}
            
            autoReadMessages();
            setInterval(autoReadMessages, 5000);
            
            const observer = new MutationObserver((mutations) => {{
                mutations.forEach(() => {{
                    autoReadMessages();
                }});
            }});
            
            observer.observe(document.body, {{
                childList: true,
                subtree: true
            }});
            
            console.log('[AutoRead] Initialized with keywords:', AUTO_READ_KEYWORDS);
        }})();
        "#,
        keywords_json
    )
}
