//! Performance optimization module - speeds up Teams content loading
//! Minimal version - preconnect only, no DOM mutations that could break Teams UI

/// JavaScript to optimize Teams performance
/// Minimal safe version - preconnect hints only
pub fn get_performance_script() -> String {
    r#"
    (function() {
        'use strict';
        
        // ========== RESOURCE HINTS ONLY ==========
        function addResourceHints() {
            const origins = [
                'https://teams.microsoft.com',
                'https://login.microsoftonline.com',
                'https://graph.microsoft.com',
                'https://statics.teams.cdn.office.net'
            ];
            
            origins.forEach(origin => {
                if (!document.querySelector('link[rel="preconnect"][href="' + origin + '"]')) {
                    const link = document.createElement('link');
                    link.rel = 'preconnect';
                    link.href = origin;
                    document.head.appendChild(link);
                }
            });
            
            console.log('[Perf] Preconnect hints added');
        }
        
        function init() {
            addResourceHints();
        }
        
        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', init);
        } else {
            setTimeout(init, 1000);
        }
        
    })();
    "#
    .to_string()
}

/// Empty - hover prefetch removed to prevent click hijack in dialogs
pub fn get_chat_speedup_script() -> String {
    String::new()
}

/// Get combined performance scripts (preconnect only)
pub fn get_all_optimization_scripts() -> String {
    get_performance_script()
}
