//! Performance optimization module - speeds up Teams content loading

/// JavaScript to optimize Teams performance
/// Safe version - does NOT remove or hide UI elements
pub fn get_performance_script() -> String {
    r#"
    (function() {
        'use strict';
        
        console.log('[Perf] Loading safe performance optimizations...');
        
        // ========== PREFETCH ==========
        function prefetchLinks() {
            const links = document.querySelectorAll('a[href]:not([data-prefetched])');
            
            links.forEach(link => {
                const href = link.href;
                if (!href) return;
                
                // Only prefetch Teams/Microsoft links
                if (!href.includes('teams.microsoft.com') && 
                    !href.includes('microsoft.com')) return;
                
                // Check if link is in viewport
                const rect = link.getBoundingClientRect();
                if (rect.top > window.innerHeight + 200) return;
                
                // Create prefetch link
                const prefetchLink = document.createElement('link');
                prefetchLink.rel = 'prefetch';
                prefetchLink.href = href;
                prefetchLink.as = 'document';
                document.head.appendChild(prefetchLink);
                
                link.setAttribute('data-prefetched', 'true');
            });
        }
        
        // ========== LAZY LOADING (images only) ==========
        function setupLazyLoading() {
            // Only handle images with data-src - DO NOT touch buttons/menus
            const imageObserver = new IntersectionObserver((entries) => {
                entries.forEach(entry => {
                    if (entry.isIntersecting) {
                        const img = entry.target;
                        
                        if (img.dataset.src) {
                            img.src = img.dataset.src;
                            img.removeAttribute('data-src');
                        }
                        
                        if (img.dataset.srcset) {
                            img.srcset = img.dataset.srcset;
                            img.removeAttribute('data-srcset');
                        }
                        
                        imageObserver.unobserve(img);
                    }
                });
            }, {
                rootMargin: '100px'
            });
            
            // Only observe images that have data-src
            document.querySelectorAll('img[data-src], img[data-srcset]').forEach(img => {
                imageObserver.observe(img);
            });
        }
        
        // ========== RESOURCE HINTS ==========
        function addResourceHints() {
            const origins = [
                'https://teams.microsoft.com',
                'https://login.microsoftonline.com',
                'https://graph.microsoft.com',
                'https://statics.teams.cdn.office.net'
            ];
            
            origins.forEach(origin => {
                if (!document.querySelector(`link[rel="preconnect"][href="${origin}"]`)) {
                    const link = document.createElement('link');
                    link.rel = 'preconnect';
                    link.href = origin;
                    document.head.appendChild(link);
                }
            });
        }
        
        // ========== SAFE SCROLL ==========
        function optimizeScrolling() {
            // Use passive event listeners for scroll - no DOM modifications
            window.addEventListener('scroll', () => {
                // Prefetch more content when scrolling
                requestAnimationFrame(prefetchLinks);
            }, { passive: true });
        }
        
        // ========== INITIALIZE ==========
        function init() {
            console.log('[Perf] Applying safe optimizations...');
            
            addResourceHints();
            prefetchLinks();
            setupLazyLoading();
            optimizeScrolling();
            
            console.log('[Perf] ✓ Done (no UI modifications)');
        }
        
        // Start when ready
        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', init);
        } else {
            // Wait a bit for Teams to load
            setTimeout(init, 2000);
        }
        
    })();
    "#
    .to_string()
}

/// JavaScript for faster chat loading - SAFE version
pub fn get_chat_speedup_script() -> String {
    r#"
    (function() {
        'use strict';
        
        console.log('[ChatSpeed] Loading safe chat optimizations...');
        
        // Only prefetch on hover - no DOM modifications
        document.addEventListener('mouseover', (e) => {
            const chatItem = e.target.closest('[class*="chat-item"], [class*="conversation"]');
            if (chatItem) {
                const link = chatItem.querySelector('a');
                if (link && link.href && !link.dataset.prefetched) {
                    const prefetch = document.createElement('link');
                    prefetch.rel = 'prefetch';
                    prefetch.href = link.href;
                    document.head.appendChild(prefetch);
                    link.dataset.prefetched = 'true';
                }
            }
        });
        
        console.log('[ChatSpeed] ✓ Hover prefetch enabled');
        
    })();
    "#
    .to_string()
}

/// Get combined performance and chat speed scripts
pub fn get_all_optimization_scripts() -> String {
    format!(
        "{}\n\n{}",
        get_performance_script(),
        get_chat_speedup_script()
    )
}
