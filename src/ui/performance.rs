//! Performance optimization module - speeds up Teams content loading
//! Minimal version - preconnect + light GC hints + visibility-pause + idle callback.
//! No DOM mutations that could break Teams UI.

/// JavaScript to force visible webview background (injected first).
/// Polls until documentElement available, then injects style tag immediately.
pub fn get_visibility_script() -> String {
    r#"(function(){'use strict';
function _rvis(){if(!document.documentElement){setTimeout(_rvis,0);return;}
var s=document.createElement('style');
s.textContent='html,body{background:#f5f5f5!important}';
document.documentElement.appendChild(s);}_rvis();})();
"#
    .to_string()
}

/// JavaScript to optimize Teams performance
/// Minimal safe version - preconnect hints + light GC hints
pub fn get_performance_script() -> String {
    r#"
    (function() {
        'use strict';

        // ========== RESOURCE HINTS ==========
        function addResourceHints() {
            const origins = [
                'https://teams.office.com',
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

        // ========== VISIBILITY PAUSE ==========
        // Khi user chuyển tab / minimize → pause timer/animation không cần thiết
        // Khi quay lại → resume
        let visibilityPaused = false;
        function setupVisibilityPause() {
            document.addEventListener('visibilitychange', function() {
                if (document.hidden) {
                    visibilityPaused = true;
                    console.log('[Perf] Tab hidden - pausing non-essential timers');
                    // Hook: dispatch event cho Teams extensions
                    window.dispatchEvent(new CustomEvent('rteams-visibility-pause'));
                } else if (visibilityPaused) {
                    visibilityPaused = false;
                    console.log('[Perf] Tab visible - resuming');
                    window.dispatchEvent(new CustomEvent('rteams-visibility-resume'));
                }
            });
        }

        // ========== IDLE GC HINT ==========
        // Sau 30s không tương tác → hint browser nên giải phóng bộ nhớ
        let idleTimer = null;
        let lastActivity = Date.now();
        const IDLE_THRESHOLD_MS = 30000; // 30s

        function resetIdleTimer() {
            lastActivity = Date.now();
            if (idleTimer) {
                clearTimeout(idleTimer);
                idleTimer = null;
            }
            scheduleIdleGC();
        }

        function scheduleIdleGC() {
            if (idleTimer) clearTimeout(idleTimer);
            idleTimer = setTimeout(function() {
                const idleTime = Date.now() - lastActivity;
                if (idleTime >= IDLE_THRESHOLD_MS) {
                    console.log('[Perf] Idle for ' + Math.round(idleTime/1000) + 's - hinting GC');
                    // Browser sẽ tự quyết định có GC hay không
                    if (window.requestIdleCallback) {
                        window.requestIdleCallback(function() {
                            // Giải phóng cached DOM nodes ngoài viewport
                            try {
                                const imgs = document.querySelectorAll('img[loading="lazy"]');
                                imgs.forEach(function(img) {
                                    if (img.getBoundingClientRect().bottom < 0 ||
                                        img.getBoundingClientRect().top > window.innerHeight) {
                                        img.src = '';
                                    }
                                });
                            } catch(e) {}
                        }, { timeout: 2000 });
                    }
                    // Hint cho V8 GC
                    if (window.gc) {
                        try { window.gc(); } catch(e) {}
                    }
                }
                scheduleIdleGC();
            }, IDLE_THRESHOLD_MS);
        }

        function setupIdleTracking() {
            ['mousedown', 'keydown', 'scroll', 'touchstart', 'wheel'].forEach(function(evt) {
                document.addEventListener(evt, resetIdleTimer, { passive: true });
            });
            scheduleIdleGC();
        }

        // ========== CONTENT VISIBILITY ==========
        // Hint trình duyệt render chậm cho element off-screen
        function applyContentVisibility() {
            try {
                // Tìm chat message lists, channel lists, v.v.
                const candidates = document.querySelectorAll(
                    '[data-tid*="message-list"], [data-tid*="channel-list"], .ts-message-list'
                );
                candidates.forEach(function(el) {
                    if (!el.style.contentVisibility) {
                        el.style.contentVisibility = 'auto';
                        el.style.containIntrinsicSize = '0 100px';
                    }
                });
            } catch(e) {}
        }

        function init() {
            fixVisibility();
            addResourceHints();
            setupVisibilityPause();
            setupIdleTracking();
            // Áp dụng content-visibility cho lists sau 3s (Teams render xong)
            setTimeout(applyContentVisibility, 3000);
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

/// Get combined performance scripts (preconnect + GC hint + visibility-pause + idle callback)
pub fn get_all_optimization_scripts() -> String {
    get_performance_script()
}
