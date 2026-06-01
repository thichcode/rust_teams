//! Realtime translate side panel
//! Injects a floating overlay that shows the latest transcript, translation,
//! and 3 quick-reply suggestions. Push state via `window` events from Rust.

/// Script that builds the overlay UI and listens for `rteams-realtime` events.
pub fn get_realtime_panel_script() -> String {
    r#"
    (function() {
        'use strict';

        if (window.__rteamsRealtimePanelLoaded) return;
        window.__rteamsRealtimePanelLoaded = true;

        const PANEL_ID = 'rteams-realtime-panel';
        const STYLE_ID = 'rteams-realtime-styles';

        function injectStyles() {
            if (document.getElementById(STYLE_ID)) return;
            const style = document.createElement('style');
            style.id = STYLE_ID;
            style.textContent = `
                #${PANEL_ID} {
                    position: fixed;
                    top: 80px;
                    right: 16px;
                    width: 360px;
                    max-height: 60vh;
                    z-index: 2147483600;
                    background: rgba(32, 31, 31, 0.92);
                    color: #f5f5f5;
                    border: 1px solid #6264A7;
                    border-radius: 8px;
                    box-shadow: 0 8px 24px rgba(0,0,0,0.4);
                    font-family: 'Segoe UI', system-ui, sans-serif;
                    font-size: 13px;
                    line-height: 1.45;
                    display: none;
                    flex-direction: column;
                    overflow: hidden;
                    backdrop-filter: blur(8px);
                }
                #${PANEL_ID}.visible { display: flex; }
                #${PANEL_ID} .rt-header {
                    padding: 10px 14px;
                    background: #6264A7;
                    color: #fff;
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    font-weight: 600;
                    cursor: move;
                    user-select: none;
                }
                #${PANEL_ID} .rt-header button {
                    background: transparent;
                    color: #fff;
                    border: 0;
                    font-size: 16px;
                    cursor: pointer;
                    padding: 0 4px;
                }
                #${PANEL_ID} .rt-body {
                    padding: 10px 14px;
                    overflow-y: auto;
                    flex: 1;
                }
                #${PANEL_ID} .rt-section {
                    margin-bottom: 10px;
                }
                #${PANEL_ID} .rt-label {
                    font-size: 11px;
                    text-transform: uppercase;
                    letter-spacing: 0.04em;
                    color: #b6b6b6;
                    margin-bottom: 3px;
                }
                #${PANEL_ID} .rt-source {
                    color: #e0e0e0;
                    font-size: 12px;
                }
                #${PANEL_ID} .rt-translation {
                    color: #c8c8ff;
                    font-size: 16px;
                    font-weight: 500;
                }
                #${PANEL_ID} .rt-suggestions {
                    display: flex;
                    flex-direction: column;
                    gap: 6px;
                }
                #${PANEL_ID} .rt-suggestion {
                    background: rgba(98,100,167,0.18);
                    border: 1px solid rgba(98,100,167,0.4);
                    border-radius: 6px;
                    padding: 8px 10px;
                    color: #f0f0ff;
                    cursor: pointer;
                    text-align: left;
                    font: inherit;
                    transition: background 0.15s;
                }
                #${PANEL_ID} .rt-suggestion:hover {
                    background: rgba(98,100,167,0.35);
                }
                #${PANEL_ID} .rt-status {
                    padding: 6px 14px;
                    font-size: 11px;
                    color: #b6b6b6;
                    border-top: 1px solid rgba(255,255,255,0.1);
                }
                #${PANEL_ID} .rt-status.live::before {
                    content: '●';
                    color: #4ec44e;
                    margin-right: 6px;
                }
            `;
            document.head.appendChild(style);
        }

        function ensurePanel() {
            let panel = document.getElementById(PANEL_ID);
            if (panel) return panel;
            injectStyles();
            panel = document.createElement('div');
            panel.id = PANEL_ID;
            panel.innerHTML = `
                <div class="rt-header">
                    <span>🌐 Realtime Translate</span>
                    <div>
                        <button id="rt-toggle" title="Toggle">▾</button>
                        <button id="rt-close" title="Close">✕</button>
                    </div>
                </div>
                <div class="rt-body">
                    <div class="rt-section">
                        <div class="rt-label">Original (<span class="rt-source-lang">en</span>)</div>
                        <div class="rt-source rt-source-text">Waiting for speech…</div>
                    </div>
                    <div class="rt-section">
                        <div class="rt-label">Translation (<span class="rt-target-lang">vi</span>)</div>
                        <div class="rt-translation rt-translation-text">Đang chờ giọng nói…</div>
                    </div>
                    <div class="rt-section">
                        <div class="rt-label">Suggested replies</div>
                        <div class="rt-suggestions"></div>
                    </div>
                </div>
                <div class="rt-status">Idle</div>
            `;
            document.body.appendChild(panel);

            // Drag
            const header = panel.querySelector('.rt-header');
            let dragging = false, dx = 0, dy = 0;
            header.addEventListener('mousedown', (e) => {
                if (e.target.tagName === 'BUTTON') return;
                dragging = true;
                const rect = panel.getBoundingClientRect();
                dx = e.clientX - rect.left;
                dy = e.clientY - rect.top;
                panel.style.left = rect.left + 'px';
                panel.style.top = rect.top + 'px';
                panel.style.right = 'auto';
                e.preventDefault();
            });
            document.addEventListener('mousemove', (e) => {
                if (!dragging) return;
                panel.style.left = (e.clientX - dx) + 'px';
                panel.style.top = (e.clientY - dy) + 'px';
            });
            document.addEventListener('mouseup', () => { dragging = false; });

            // Toggle
            const toggleBtn = panel.querySelector('#rt-toggle');
            toggleBtn.addEventListener('click', () => {
                const body = panel.querySelector('.rt-body');
                const status = panel.querySelector('.rt-status');
                if (body.style.display === 'none') {
                    body.style.display = '';
                    status.style.display = '';
                    toggleBtn.textContent = '▾';
                } else {
                    body.style.display = 'none';
                    status.style.display = 'none';
                    toggleBtn.textContent = '▸';
                }
            });

            // Close
            panel.querySelector('#rt-close').addEventListener('click', () => {
                panel.classList.remove('visible');
            });
            return panel;
        }

        function renderPayload(panel, payload) {
            panel.classList.add('visible');
            panel.querySelector('.rt-source-lang').textContent = payload.source_lang || 'en';
            panel.querySelector('.rt-target-lang').textContent = payload.target_lang || 'vi';
            panel.querySelector('.rt-source-text').textContent = payload.source_text || '';
            panel.querySelector('.rt-translation-text').textContent = payload.translated_text || '';
            const list = panel.querySelector('.rt-suggestions');
            list.innerHTML = '';
            (payload.suggestions || []).forEach((s) => {
                const btn = document.createElement('button');
                btn.className = 'rt-suggestion';
                btn.textContent = s;
                btn.addEventListener('click', () => {
                    navigator.clipboard && navigator.clipboard.writeText(s);
                    btn.textContent = '✓ Copied';
                    setTimeout(() => { btn.textContent = s; }, 1200);
                });
                list.appendChild(btn);
            });
            const status = panel.querySelector('.rt-status');
            status.classList.add('live');
            status.textContent = 'Live · last update ' + new Date(payload.timestamp || Date.now()).toLocaleTimeString();
        }

        window.addEventListener('rteams-realtime', (e) => {
            const panel = ensurePanel();
            renderPayload(panel, e.detail);
        });

        console.log('[RealtimePanel] Mounted');
    })();
    "#
    .to_string()
}
