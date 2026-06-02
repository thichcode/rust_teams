//! Realtime translate side panel
//! Injects a floating overlay that shows the latest transcript, translation,
//! and 3 quick-reply suggestions. Push state via `window` events from Rust.
//!
//! ## Events from Rust
//! - `rteams-realtime`  -> { source_text, translated_text, suggestions, ... }
//! - `rteams-panel-state` -> { state, message }  where state in:
//!     "idle" | "listening" | "error" | "no_api_key" | "no_mic" | "stopped"

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
                    max-height: 70vh;
                    z-index: 2147483600;
                    background: rgba(32, 31, 31, 0.95);
                    color: #f5f5f5;
                    border: 1px solid #6264A7;
                    border-radius: 8px;
                    box-shadow: 0 8px 24px rgba(0,0,0,0.5);
                    font-family: 'Segoe UI', system-ui, sans-serif;
                    font-size: 13px;
                    line-height: 1.45;
                    display: none;
                    flex-direction: column;
                    overflow: hidden;
                    backdrop-filter: blur(8px);
                }
                #${PANEL_ID}.visible { display: flex; }
                #${PANEL_ID}.state-error { border-color: #d83b3b; }
                #${PANEL_ID}.state-listening { border-color: #4ec44e; }
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
                #${PANEL_ID}.state-listening .rt-header { background: #2d8a2d; }
                #${PANEL_ID}.state-error .rt-header { background: #a32d2d; }
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
                    padding: 8px 14px;
                    font-size: 11px;
                    color: #b6b6b6;
                    border-top: 1px solid rgba(255,255,255,0.1);
                }
                #${PANEL_ID} .rt-status.live::before {
                    content: '●';
                    color: #4ec44e;
                    margin-right: 6px;
                    animation: rt-blink 1.4s ease-in-out infinite;
                }
                #${PANEL_ID} .rt-status.error::before {
                    content: '⚠';
                    color: #ff7070;
                    margin-right: 6px;
                }
                @keyframes rt-blink {
                    0%, 100% { opacity: 1; }
                    50% { opacity: 0.3; }
                }
                #${PANEL_ID} .rt-error-detail {
                    font-size: 10px;
                    color: #ff9999;
                    margin-top: 4px;
                    line-height: 1.3;
                }
                #${PANEL_ID} .rt-actions {
                    padding: 6px 14px;
                    border-top: 1px solid rgba(255,255,255,0.1);
                    display: flex;
                    gap: 6px;
                }
                #${PANEL_ID} .rt-action-btn {
                    flex: 1;
                    background: rgba(98,100,167,0.25);
                    border: 1px solid rgba(98,100,167,0.5);
                    color: #f0f0ff;
                    border-radius: 4px;
                    padding: 6px 8px;
                    cursor: pointer;
                    font: inherit;
                    font-size: 11px;
                }
                #${PANEL_ID} .rt-action-btn:hover {
                    background: rgba(98,100,167,0.45);
                }
            `;
            document.head.appendChild(style);
        }

        function ensurePanel() {
            let panel = document.getElementById(PANEL_ID);
            if (panel) return panel;
            if (!document.body) {
                throw new Error('document.body not ready');
            }
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
                        <div class="rt-source rt-source-text">—</div>
                    </div>
                    <div class="rt-section">
                        <div class="rt-label">Translation (<span class="rt-target-lang">vi</span>)</div>
                        <div class="rt-translation rt-translation-text">—</div>
                    </div>
                    <div class="rt-section">
                        <div class="rt-label">Suggested replies</div>
                        <div class="rt-suggestions"></div>
                    </div>
                </div>
                <div class="rt-status">Initializing…</div>
                <div class="rt-error-detail" style="display:none"></div>
                <div class="rt-actions">
                    <button class="rt-action-btn" id="rt-toggle-listen">Start listening</button>
                </div>
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

            // Toggle body
            panel.querySelector('#rt-toggle').addEventListener('click', () => {
                const body = panel.querySelector('.rt-body');
                const status = panel.querySelector('.rt-status');
                const actions = panel.querySelector('.rt-actions');
                const toggleBtn = panel.querySelector('#rt-toggle');
                if (body.style.display === 'none') {
                    body.style.display = '';
                    status.style.display = '';
                    actions.style.display = '';
                    toggleBtn.textContent = '▾';
                } else {
                    body.style.display = 'none';
                    status.style.display = 'none';
                    actions.style.display = 'none';
                    toggleBtn.textContent = '▸';
                }
            });

            // Close
            panel.querySelector('#rt-close').addEventListener('click', () => {
                panel.classList.remove('visible');
            });

            // Manual start/stop
            panel.querySelector('#rt-toggle-listen').addEventListener('click', () => {
                const btn = panel.querySelector('#rt-toggle-listen');
                const isListening = btn.dataset.listening === '1';
                if (isListening) {
                    if (window.ipc && window.ipc.postMessage) {
                        window.ipc.postMessage(JSON.stringify({
                            type: 'realtime_toggle',
                            data: { enabled: false }
                        }));
                    }
                    btn.dataset.listening = '0';
                    btn.textContent = 'Start listening';
                } else {
                    if (window.ipc && window.ipc.postMessage) {
                        window.ipc.postMessage(JSON.stringify({
                            type: 'realtime_toggle',
                            data: { enabled: true }
                        }));
                    }
                    btn.dataset.listening = '1';
                    btn.textContent = 'Stop listening';
                }
            });

            return panel;
        }

        function renderPayload(panel, payload) {
            panel.classList.add('visible');
            if (payload.source_lang) panel.querySelector('.rt-source-lang').textContent = payload.source_lang;
            if (payload.target_lang) panel.querySelector('.rt-target-lang').textContent = payload.target_lang;
            if (payload.source_text) panel.querySelector('.rt-source-text').textContent = payload.source_text;
            if (payload.translated_text) panel.querySelector('.rt-translation-text').textContent = payload.translated_text;
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
        }

        function renderState(panel, state, message, detail) {
            panel.classList.add('visible');
            panel.classList.remove('state-listening', 'state-error');
            const status = panel.querySelector('.rt-status');
            const errBox = panel.querySelector('.rt-error-detail');
            const toggleBtn = panel.querySelector('#rt-toggle-listen');
            errBox.style.display = 'none';
            errBox.textContent = '';
            status.classList.remove('live', 'error');

            switch (state) {
                case 'listening':
                    panel.classList.add('state-listening');
                    status.classList.add('live');
                    status.textContent = message || 'Listening for call audio…';
                    if (toggleBtn) {
                        toggleBtn.dataset.listening = '1';
                        toggleBtn.textContent = 'Stop listening';
                    }
                    break;
                case 'error':
                    panel.classList.add('state-error');
                    status.classList.add('error');
                    status.textContent = message || 'Error';
                    if (detail) {
                        errBox.style.display = '';
                        errBox.textContent = detail;
                    }
                    if (toggleBtn) {
                        toggleBtn.dataset.listening = '0';
                        toggleBtn.textContent = 'Retry';
                    }
                    break;
                case 'no_api_key':
                    panel.classList.add('state-error');
                    status.classList.add('error');
                    status.textContent = '⚠ OpenAI API key not configured';
                    errBox.style.display = '';
                    errBox.textContent = detail || 'Edit config.toml at %APPDATA%\\rust_teams\\config.toml → [realtime_translate.stt] api_key = "sk-..." and set api_key for [realtime_translate.translator] + [realtime_translate.suggester] as well.';
                    if (toggleBtn) {
                        toggleBtn.dataset.listening = '0';
                        toggleBtn.textContent = 'Start listening';
                    }
                    break;
                case 'no_mic':
                    panel.classList.add('state-error');
                    status.classList.add('error');
                    status.textContent = '⚠ No microphone or loopback device';
                    errBox.style.display = '';
                    errBox.textContent = detail || 'No audio input device available. Connect a microphone or enable Stereo Mix in Windows sound settings.';
                    if (toggleBtn) {
                        toggleBtn.dataset.listening = '0';
                        toggleBtn.textContent = 'Start listening';
                    }
                    break;
                case 'stopped':
                    status.textContent = message || 'Stopped';
                    if (toggleBtn) {
                        toggleBtn.dataset.listening = '0';
                        toggleBtn.textContent = 'Start listening';
                    }
                    break;
                case 'idle':
                default:
                    status.textContent = message || 'Waiting for a Teams call to start…';
                    if (toggleBtn) {
                        toggleBtn.dataset.listening = '0';
                        toggleBtn.textContent = 'Start listening';
                    }
                    break;
            }
        }

        window.addEventListener('rteams-realtime', (e) => {
            try {
                const panel = ensurePanel();
                renderPayload(panel, e.detail);
            } catch (err) {
                console.error('[RealtimePanel] payload error:', err);
            }
        });

        window.addEventListener('rteams-panel-state', (e) => {
            try {
                const panel = ensurePanel();
                const d = e.detail || {};
                renderState(panel, d.state, d.message, d.detail);
            } catch (err) {
                console.error('[RealtimePanel] state error:', err);
            }
        });

        // ---- Auto-show on app start ----
        // Initialization scripts run BEFORE document.body exists in many cases.
        // We must wait until the body is ready, otherwise appendChild throws
        // and the whole IIFE dies silently, leaving the user with no panel.
        function initPanelWhenReady() {
            try {
                if (!document.body) {
                    // Body not ready yet - retry shortly
                    setTimeout(initPanelWhenReady, 50);
                    return;
                }
                const panel = ensurePanel();
                panel.classList.add('visible');
                renderState(panel, 'idle', 'Realtime translate ready. Join a Teams call to start.', null);
                console.log('[RealtimePanel] Mounted and visible');
            } catch (err) {
                console.error('[RealtimePanel] init error:', err);
                // Retry once more in case of transient DOM issues
                setTimeout(initPanelWhenReady, 200);
            }
        }

        // Wait for DOM ready before mounting
        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', initPanelWhenReady);
        } else {
            initPanelWhenReady();
        }

        // Safety net: try again after window.load (some SPAs replace body)
        window.addEventListener('load', () => {
            if (!document.getElementById(PANEL_ID)) {
                console.log('[RealtimePanel] Re-mounting on window.load');
                initPanelWhenReady();
            }
        });
    })();
    "#
    .to_string()
}
