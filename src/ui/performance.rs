//! Performance optimization module - speeds up Teams content loading

/// JavaScript to optimize Teams performance
/// Includes lazy loading, prefetch, and DOM optimization
pub fn get_performance_script() -> String {
    r#"
    (function() {
        'use strict';
        
        const PERF_CONFIG = {
            enablePrefetch: true,
            enableLazyLoad: true,
            enableDomOptimization: true,
            prefetchDelay: 1000,
            scrollThreshold: 200
        };
        
        // State
        const state = {
            prefetched: new Set(),
            lazyImages: new WeakMap(),
            isOptimized: false
        };
        
        // ========== PREFETCH ==========
        function prefetchLinks() {
            if (!PERF_CONFIG.enablePrefetch) return;
            
            // Prefetch visible links
            const links = document.querySelectorAll('a[href]:not([data-prefetched])');
            
            links.forEach(link => {
                const href = link.href;
                if (!href || state.prefetched.has(href)) return;
                
                // Only prefetch Teams/Microsoft links
                if (!href.includes('teams.microsoft.com') && 
                    !href.includes('microsoft.com')) return;
                
                // Check if link is in viewport
                const rect = link.getBoundingClientRect();
                if (rect.top > window.innerHeight + PERF_CONFIG.scrollThreshold) return;
                
                // Create prefetch link
                const prefetchLink = document.createElement('link');
                prefetchLink.rel = 'prefetch';
                prefetchLink.href = href;
                prefetchLink.as = 'document';
                document.head.appendChild(prefetchLink);
                
                state.prefetched.add(href);
                link.setAttribute('data-prefetched', 'true');
            });
        }
        
        // ========== LAZY LOADING ==========
        function setupLazyLoading() {
            if (!PERF_CONFIG.enableLazyLoad) return;
            
            // Observe images for lazy loading
            const imageObserver = new IntersectionObserver((entries) => {
                entries.forEach(entry => {
                    if (entry.isIntersecting) {
                        const img = entry.target;
                        
                        // Load data-src if available
                        if (img.dataset.src) {
                            img.src = img.dataset.src;
                            img.removeAttribute('data-src');
                        }
                        
                        // Load data-srcset if available
                        if (img.dataset.srcset) {
                            img.srcset = img.dataset.srcset;
                            img.removeAttribute('data-srcset');
                        }
                        
                        img.classList.add('loaded');
                        imageObserver.unobserve(img);
                    }
                });
            }, {
                rootMargin: '100px' // Start loading 100px before visible
            });
            
            // Observe all images
            document.querySelectorAll('img[data-src], img[data-srcset]').forEach(img => {
                imageObserver.observe(img);
            });
            
            // Observe avatars specifically
            document.querySelectorAll('[class*="avatar"], [class*="profile"]').forEach(el => {
                if (el.tagName === 'IMG' || el.querySelector('img')) {
                    const img = el.tagName === 'IMG' ? el : el.querySelector('img');
                    if (img) imageObserver.observe(img);
                }
            });
        }
        
        // ========== DOM OPTIMIZATION ==========
        function optimizeDOM() {
            if (!PERF_CONFIG.enableDomOptimization) return;
            
            // Remove empty elements
            document.querySelectorAll('div:empty, span:empty').forEach(el => {
                if (!el.classList.length && !el.id) {
                    el.remove();
                }
            });
            
            // Optimize scrollable containers
            document.querySelectorAll('[class*="scroll"], [class*="list"]').forEach(el => {
                // Add will-change for smoother scrolling
                el.style.willChange = 'transform';
                
                // Enable content-visibility for off-screen content
                if (el.scrollHeight > window.innerHeight * 2) {
                    el.style.contentVisibility = 'auto';
                    el.style.containIntrinsicSize = '0 500px';
                }
            });
            
            // Optimize animations
            document.querySelectorAll('[class*="animation"], [class*="transition"]').forEach(el => {
                el.style.willChange = 'transform, opacity';
            });
        }
        
        // ========== SCROLL OPTIMIZATION ==========
        function optimizeScrolling() {
            // Use passive event listeners for scroll
            let scrollTimeout;
            
            window.addEventListener('scroll', () => {
                if (scrollTimeout) clearTimeout(scrollTimeout);
                
                scrollTimeout = setTimeout(() => {
                    // Prefetch more content when scrolling
                    prefetchLinks();
                    
                    // Load more lazy images
                    setupLazyLoading();
                }, 100);
            }, { passive: true });
            
            // Virtual scrolling for long lists
            const chatLists = document.querySelectorAll('[class*="chat-list"], [class*="message-list"]');
            
            chatLists.forEach(list => {
                if (list.scrollHeight > window.innerHeight * 3) {
                    list.style.contentVisibility = 'auto';
                    list.style.containIntrinsicSize = '0 1000px';
                }
            });
        }
        
        // ========== RESOURCE HINTS ==========
        function addResourceHints() {
            // Preconnect to Microsoft servers
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
        
        // ========== PERFORMANCE MONITORING ==========
        function logPerformance() {
            if (window.performance) {
                const timing = performance.getEntriesByType('navigation')[0];
                if (timing) {
                    console.log('[Perf] DOMContentLoaded:', Math.round(timing.domContentLoadedEventEnd - timing.startTime), 'ms');
                    console.log('[Perf] Load complete:', Math.round(timing.loadEventEnd - timing.startTime), 'ms');
                }
            }
        }
        
        // ========== MAIN ==========
        function optimize() {
            if (state.isOptimized) return;
            state.isOptimized = true;
            
            console.log('[Perf] Starting optimization...');
            
            // Run optimizations
            addResourceHints();
            prefetchLinks();
            setupLazyLoading();
            optimizeDOM();
            optimizeScrolling();
            
            // Re-run on DOM changes
            const observer = new MutationObserver(() => {
                setupLazyLoading();
                optimizeDOM();
            });
            
            observer.observe(document.body, {
                childList: true,
                subtree: true
            });
            
            // Log performance
            setTimeout(logPerformance, 2000);
            
            console.log('[Perf] ✓ Optimization complete');
        }
        
        // Initialize
        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', optimize);
        } else {
            optimize();
        }
        
        // Re-optimize periodically for dynamic content
        setInterval(() => {
            state.isOptimized = false;
            optimize();
        }, 10000);
        
    })();
    "#
    .to_string()
}

/// JavaScript for faster chat loading
pub fn get_chat_speedup_script() -> String {
    r#"
    (function() {
        'use strict';
        
        // Intercept fetch requests to cache responses
        const originalFetch = window.fetch;
        const cache = new Map();
        
        window.fetch = function(...args) {
            const url = args[0];
            
            // Cache GET requests for 30 seconds
            if (args[1]?.method === 'GET' || !args[1]?.method) {
                const cacheKey = typeof url === 'string' ? url : url.url;
                
                if (cache.has(cacheKey)) {
                    const cached = cache.get(cacheKey);
                    if (Date.now() - cached.time < 30000) {
                        return Promise.resolve(cached.response.clone());
                    }
                }
                
                return originalFetch.apply(this, args).then(response => {
                    cache.set(cacheKey, {
                        response: response.clone(),
                        time: Date.now()
                    });
                    return response;
                });
            }
            
            return originalFetch.apply(this, args);
        };
        
        // Optimize XMLHttpRequest too
        const originalOpen = XMLHttpRequest.prototype.open;
        const originalSend = XMLHttpRequest.prototype.send;
        
        XMLHttpRequest.prototype.open = function(method, url, ...rest) {
            this._url = url;
            this._method = method;
            return originalOpen.call(this, method, url, ...rest);
        };
        
        // Prefetch when hovering over chat items
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
        
        console.log('[ChatSpeed] ✓ Fetch caching enabled');
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
