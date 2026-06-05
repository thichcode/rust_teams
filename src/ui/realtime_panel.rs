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
                #${PANEL_ID} #rt-toggle-listen {
                    font-weight: 600;
                    font-size: 12px;
                    transition: background .2s, border-color .2s;
                }
                #${PANEL_ID} #rt-toggle-listen[data-listening="1"] {
                    background: rgba(29,111,29,0.4);
                    border-color: rgba(29,185,29,0.6);
                }
                #${PANEL_ID} #rt-toggle-listen[data-listening="0"] {
                    background: rgba(168,122,0,0.25);
                    border-color: rgba(168,122,0,0.4);
                }
                #rt-config-modal {
                    position: fixed;
                    top: 0; left: 0; right: 0; bottom: 0;
                    background: rgba(0,0,0,0.7);
                    z-index: 2147483647;
                    display: none;
                    align-items: center;
                    justify-content: center;
                    font-family: 'Segoe UI', system-ui, sans-serif;
                }
                #rt-config-modal.visible { display: flex; }
                #rt-config-modal .rt-modal-box {
                    background: #2a2a2a;
                    color: #f5f5f5;
                    border: 1px solid #6264A7;
                    border-radius: 8px;
                    padding: 20px;
                    width: 480px;
                    max-width: 90vw;
                    max-height: 90vh;
                    overflow-y: auto;
                    box-shadow: 0 12px 40px rgba(0,0,0,0.6);
                }
                #rt-config-modal h3 {
                    margin: 0 0 12px;
                    color: #c8c8ff;
                    font-size: 16px;
                }
                #rt-config-modal p {
                    margin: 0 0 14px;
                    font-size: 12px;
                    color: #b6b6b6;
                    line-height: 1.5;
                }
                #rt-config-modal label {
                    display: block;
                    font-size: 11px;
                    text-transform: uppercase;
                    color: #b6b6b6;
                    margin-top: 10px;
                    margin-bottom: 4px;
                    letter-spacing: 0.04em;
                }
                #rt-config-modal input {
                    width: 100%;
                    padding: 8px 10px;
                    background: #1a1a1a;
                    border: 1px solid #444;
                    border-radius: 4px;
                    color: #f5f5f5;
                    font: inherit;
                    font-size: 12px;
                    box-sizing: border-box;
                }
                #rt-config-modal input:focus {
                    outline: none;
                    border-color: #6264A7;
                }
                #rt-config-modal .rt-modal-actions {
                    margin-top: 18px;
                    display: flex;
                    gap: 8px;
                    justify-content: flex-end;
                }
                #rt-config-modal button {
                    background: #6264A7;
                    color: #fff;
                    border: 0;
                    padding: 8px 16px;
                    border-radius: 4px;
                    cursor: pointer;
                    font: inherit;
                    font-size: 12px;
                }
                #rt-config-modal button:hover { background: #7a7cc7; }
                #rt-config-modal button.rt-btn-secondary {
                    background: transparent;
                    color: #b6b6b6;
                    border: 1px solid #555;
                }
                #rt-config-modal .rt-config-hint {
                    font-size: 10px;
                    color: #888;
                    margin-top: 4px;
                }
                #rt-local-wizard {
                    position: fixed;
                    top: 0; left: 0; right: 0; bottom: 0;
                    background: rgba(0,0,0,0.7);
                    z-index: 2147483647;
                    display: none;
                    align-items: center;
                    justify-content: center;
                    font-family: 'Segoe UI', system-ui, sans-serif;
                }
                #rt-local-wizard.visible { display: flex; }
                #rt-local-wizard .rt-modal-box {
                    background: #2a2a2a;
                    color: #f5f5f5;
                    border: 1px solid #6264A7;
                    border-radius: 8px;
                    padding: 20px;
                    width: 520px;
                    max-width: 90vw;
                    max-height: 90vh;
                    overflow-y: auto;
                    box-shadow: 0 12px 40px rgba(0,0,0,0.6);
                }
                #rt-local-wizard h3 {
                    margin: 0 0 8px;
                    color: #c8c8ff;
                    font-size: 16px;
                }
                #rt-local-wizard h4 {
                    margin: 8px 0;
                    font-size: 13px;
                    color: #d0d0d0;
                }
                #rt-local-wizard h4 small {
                    font-weight: normal;
                    color: #888;
                }
                #rt-local-wizard p {
                    margin: 0 0 12px;
                    font-size: 12px;
                    color: #b6b6b6;
                }
                #rt-local-wizard .rt-radio-group {
                    display: flex;
                    flex-direction: column;
                    gap: 6px;
                }
                #rt-local-wizard .rt-radio {
                    display: flex;
                    flex-direction: column;
                    background: #1a1a1a;
                    border: 1px solid #444;
                    border-radius: 6px;
                    padding: 10px 12px;
                    cursor: pointer;
                    font-size: 12px;
                }
                #rt-local-wizard .rt-radio:hover {
                    border-color: #6264A7;
                }
                #rt-local-wizard .rt-radio input {
                    margin-right: 8px;
                }
                #rt-local-wizard .rt-radio span {
                    font-weight: 600;
                    color: #e0e0e0;
                }
                #rt-local-wizard .rt-radio .rt-config-hint {
                    font-size: 10px;
                    color: #888;
                    margin-top: 2px;
                }
                #rt-local-wizard .rt-modal-actions {
                    margin-top: 16px;
                    display: flex;
                    gap: 8px;
                    justify-content: flex-end;
                }
                #rt-local-wizard button {
                    background: #6264A7;
                    color: #fff;
                    border: 0;
                    padding: 8px 16px;
                    border-radius: 4px;
                    cursor: pointer;
                    font: inherit;
                    font-size: 12px;
                }
                #rt-local-wizard button:hover { background: #7a7cc7; }
                #rt-local-wizard button.rt-btn-secondary {
                    background: transparent;
                    color: #b6b6b6;
                    border: 1px solid #555;
                }
                #rt-local-wizard code {
                    font-size: 10px;
                    background: #1a1a1a;
                    padding: 1px 4px;
                    border-radius: 3px;
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
                    <button class="rt-action-btn" id="rt-toggle-listen" data-listening="0">🔴 Off</button>
                    <button class="rt-action-btn" id="rt-configure">⚙ Configure</button>
                    <button class="rt-action-btn" id="rt-local-mode">🖥 Local</button>
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

            // Close - stop pipeline if running
            panel.querySelector('#rt-close').addEventListener('click', () => {
                const btn = panel.querySelector('#rt-toggle-listen');
                if (btn && btn.dataset.listening === '1') {
                    if (window.ipc && window.ipc.postMessage) {
                        window.ipc.postMessage(JSON.stringify({
                            type: 'manual_toggle',
                            data: { enabled: false }
                        }));
                    }
                    btn.dataset.listening = '0';
                    btn.textContent = '🔴 Off';
                }
                panel.classList.remove('visible');
            });

            // Manual start/stop
            panel.querySelector('#rt-toggle-listen').addEventListener('click', () => {
                const btn = panel.querySelector('#rt-toggle-listen');
                const isListening = btn.dataset.listening === '1';
                if (isListening) {
                    if (window.ipc && window.ipc.postMessage) {
                        window.ipc.postMessage(JSON.stringify({
                            type: 'manual_toggle',
                            data: { enabled: false }
                        }));
                    }
                    btn.dataset.listening = '0';
                    btn.textContent = '🔴 Off';
                } else {
                    if (window.ipc && window.ipc.postMessage) {
                        window.ipc.postMessage(JSON.stringify({
                            type: 'manual_toggle',
                            data: { enabled: true }
                        }));
                    }
                    btn.dataset.listening = '1';
                    btn.textContent = '🟢 On';
                }
            });

            // Configure API keys
            panel.querySelector('#rt-configure').addEventListener('click', () => {
                showConfigModal();
            });

            // Local mode wizard
            panel.querySelector('#rt-local-mode').addEventListener('click', () => {
                showLocalWizard();
            });

            return panel;
        }

        function showConfigModal() {
            let modal = document.getElementById('rt-config-modal');
            if (modal) {
                modal.classList.add('visible');
                return;
            }
            modal = document.createElement('div');
            modal.id = 'rt-config-modal';
            modal.innerHTML = `
                <div class="rt-modal-box">
                    <h3>⚙ Configure API keys</h3>
                    <p>Enter your OpenAI / Google / DeepL API keys. Leave blank to keep existing value. Keys are saved to <code>%APPDATA%\\rust_teams\\config.json</code>.</p>
                    <label>STT (Speech-to-Text) API key</label>
                    <input type="password" id="cfg-stt" placeholder="sk-... (OpenAI Whisper)">
                    <div class="rt-config-hint">Used for transcribing call audio. Set api_key if provider is openai / google / deepl.</div>
                    <label>Translator API key</label>
                    <input type="password" id="cfg-translator" placeholder="sk-... (OpenAI GPT-4o-mini / Google / DeepL)">
                    <div class="rt-config-hint">Used for translating transcripts.</div>
                    <label>Suggester API key</label>
                    <input type="password" id="cfg-suggester" placeholder="sk-... (OpenAI GPT-4o-mini / Ollama)">
                    <div class="rt-config-hint">Used for generating reply suggestions.</div>
                    <div class="rt-modal-actions">
                        <button class="rt-btn-secondary" id="cfg-cancel">Cancel</button>
                        <button id="cfg-save">Save</button>
                    </div>
                </div>
            `;
            document.body.appendChild(modal);
            modal.classList.add('visible');

            modal.querySelector('#cfg-cancel').addEventListener('click', () => {
                modal.classList.remove('visible');
            });
            modal.addEventListener('click', (e) => {
                if (e.target === modal) modal.classList.remove('visible');
            });
            modal.querySelector('#cfg-save').addEventListener('click', () => {
                const stt = modal.querySelector('#cfg-stt').value.trim();
                const translator = modal.querySelector('#cfg-translator').value.trim();
                const suggester = modal.querySelector('#cfg-suggester').value.trim();
                if (!stt && !translator && !suggester) {
                    alert('Please enter at least one key (or close to cancel).');
                    return;
                }
                if (window.ipc && window.ipc.postMessage) {
                    window.ipc.postMessage(JSON.stringify({
                        type: 'config_update',
                        data: {
                            stt_api_key: stt,
                            translator_api_key: translator,
                            suggester_api_key: suggester
                        }
                    }));
                }
                modal.classList.remove('visible');
            });
        }

        // ---- Local LLM mode wizard ----

        function showLocalWizard() {
            if (window.ipc && window.ipc.postMessage) {
                window.ipc.postMessage(JSON.stringify({
                    type: 'local_setup_open',
                    data: {}
                }));
            }
            // Open immediately with placeholder; populated when Rust replies
            showLocalWizardModal({ stt: [], translator: [], suggester: [], ollama_endpoint: 'http://localhost:11434', whisper_binary_path: '' });
        }

        const wizardState = { step: 1, choices: { stt: null, translator: null, suggester: null } };

        function showLocalWizardModal(opts) {
            let modal = document.getElementById('rt-local-wizard');
            if (modal) {
                modal.classList.add('visible');
                renderWizardStep(modal, opts, 1);
                return;
            }
            modal = document.createElement('div');
            modal.id = 'rt-local-wizard';
            modal.innerHTML = `
                <div class="rt-modal-box">
                    <h3>🖥 Local LLM mode</h3>
                    <p>Pick your STT, translator, and suggester models. Then R Teams verifies readiness.</p>
                    <div id="rt-wizard-step"></div>
                    <div class="rt-modal-actions">
                        <button class="rt-btn-secondary" id="rt-wiz-cancel">Cancel</button>
                        <button class="rt-btn-secondary" id="rt-wiz-back" style="display:none">← Back</button>
                        <button id="rt-wiz-next">Next →</button>
                    </div>
                </div>
            `;
            document.body.appendChild(modal);
            modal.classList.add('visible');
            modal.querySelector('#rt-wiz-cancel').addEventListener('click', () => {
                modal.classList.remove('visible');
            });
            renderWizardStep(modal, opts, 1);
        }

        function renderWizardStep(modal, opts, step) {
            wizardState.step = step;
            const container = modal.querySelector('#rt-wizard-step');
            const backBtn = modal.querySelector('#rt-wiz-back');
            const nextBtn = modal.querySelector('#rt-wiz-next');
            backBtn.style.display = step > 1 ? '' : 'none';
            nextBtn.textContent = step < 3 ? 'Next →' : '✓ Apply';
            const role = step === 1 ? 'stt' : step === 2 ? 'translator' : 'suggester';
            const title = step === 1 ? 'Pick your STT model'
                : step === 2 ? 'Pick your Translator model'
                : 'Pick your Suggester model';
            const models = opts[role] || [];
            const radios = models.map((m, i) => `
                <label class="rt-radio">
                    <input type="radio" name="rt-wiz-${role}" value="${m.id}" ${m.recommended || i === 0 ? 'checked' : ''}>
                    <span>${m.label}${m.recommended ? ' ⭐' : ''}</span>
                    <div class="rt-config-hint">${m.install_hint}</div>
                </label>
            `).join('');
            container.innerHTML = `
                <h4>${title} <small>(step ${step}/3)</small></h4>
                <div class="rt-radio-group">
                    ${radios || '<p>No models available.</p>'}
                </div>
                <div class="rt-config-hint" style="margin-top:8px">
                    Endpoint: <code>${opts.ollama_endpoint}</code><br>
                    Whisper path: <code>${opts.whisper_binary_path}</code>
                </div>
            `;
            backBtn.onclick = () => renderWizardStep(modal, opts, step - 1);
            nextBtn.onclick = () => {
                const selected = container.querySelector(`input[name="rt-wiz-${role}"]:checked`);
                if (!selected) {
                    alert('Please pick a model.');
                    return;
                }
                wizardState.choices[role] = { id: selected.value };
                if (step < 3) {
                    renderWizardStep(modal, opts, step + 1);
                } else {
                    submitWizard(modal);
                }
            };
        }

        function submitWizard(modal) {
            const choices = {
                stt: { id: wizardState.choices.stt ? wizardState.choices.stt.id : '', path: null, endpoint: null },
                translator: { id: wizardState.choices.translator ? wizardState.choices.translator.id : '', path: null, endpoint: null },
                suggester: { id: wizardState.choices.suggester ? wizardState.choices.suggester.id : '', path: null, endpoint: null }
            };
            if (window.ipc && window.ipc.postMessage) {
                window.ipc.postMessage(JSON.stringify({
                    type: 'local_setup_apply',
                    data: JSON.stringify(choices)
                }));
            }
            modal.classList.remove('visible');
        }

        function showLocalResultBanner(readiness, allReady) {
            let banner = document.getElementById('rt-local-result');
            if (!banner) {
                banner = document.createElement('div');
                banner.id = 'rt-local-result';
                banner.style.cssText = 'position:fixed;top:16px;right:16px;z-index:2147483647;padding:12px 16px;border-radius:6px;font:13px Segoe UI;max-width:340px;box-shadow:0 4px 12px rgba(0,0,0,.3);transition:opacity .3s';
                document.body.appendChild(banner);
            }
            const ok = readiness && readiness.ollama && readiness.whisper
                && readiness.ollama.status === 'ready' && readiness.whisper.status === 'ready';
            banner.style.background = (allReady && ok) ? '#1d6f1d' : '#a87a00';
            banner.style.color = '#fff';
            banner.textContent = (allReady && ok)
                ? '✅ Local mode ready — pipeline will use local providers'
                : '⚠ Local mode partially ready — check Configure for details';
            banner.style.display = 'block';
            banner.style.opacity = '1';
            setTimeout(() => {
                banner.style.opacity = '0';
                setTimeout(() => { banner.style.display = 'none'; }, 300);
            }, allReady ? 4000 : 8000);
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
                        toggleBtn.textContent = '🟢 On';
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
                        toggleBtn.textContent = '🔴 Off';
                    }
                    break;
                case 'no_api_key':
                    panel.classList.add('state-error');
                    status.classList.add('error');
                    status.textContent = '⚠ OpenAI API key not configured';
                    errBox.style.display = '';
                    errBox.textContent = detail || 'No API keys configured. Click ⚙ Configure to set keys, or use 🖥 Local for offline mode.';
                    if (toggleBtn) {
                        toggleBtn.dataset.listening = '0';
                        toggleBtn.textContent = '🔴 Off';
                    }
                    break;
                case 'no_mic':
                    panel.classList.add('state-error');
                    status.classList.add('error');
                    status.textContent = '⚠ No microphone or loopback device';
                    errBox.style.display = '';
                    errBox.textContent = detail || 'No audio loopback device available. Check Windows sound settings — Stereo Mix or loopback should be enabled.';
                    if (toggleBtn) {
                        toggleBtn.dataset.listening = '0';
                        toggleBtn.textContent = '🔴 Off';
                    }
                    break;
                case 'stopped':
                    status.textContent = message || 'Stopped';
                    if (toggleBtn) {
                        toggleBtn.dataset.listening = '0';
                        toggleBtn.textContent = '🔴 Off';
                    }
                    break;
                case 'idle':
                default:
                    status.textContent = message || 'Click 🟢 On to start translating';
                    if (toggleBtn) {
                        toggleBtn.dataset.listening = '0';
                        toggleBtn.textContent = '🔴 Off';
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
                if (d.state === 'local_wizard_options' && d.detail) {
                    try { showLocalWizardModal(JSON.parse(d.detail)); } catch (_) {}
                    return;
                }
                if (d.state === 'local_ready' || d.state === 'local_partial') {
                    let readiness = null;
                    try { readiness = d.detail ? JSON.parse(d.detail) : null; } catch (_) {}
                    showLocalResultBanner(readiness, d.state === 'local_ready');
                    return;
                }
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
                renderState(panel, 'idle', 'Realtime translate ready. Click 🟢 On to start.', null);
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
