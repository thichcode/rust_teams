//! Floating command bar — Telegram-style slash commands injected into Teams WebView.

pub fn get_command_bar_script() -> String {
    r#"
    (function() {
        'use strict';

        const BAR_ID = 'rteams-bot-bar';

        function ensureBar() {
            if (document.getElementById(BAR_ID)) return document.getElementById(BAR_ID);

            // Inject CSS
            if (!document.getElementById(BAR_ID + '-style')) {
                const style = document.createElement('style');
                style.id = BAR_ID + '-style';
                style.textContent = `
                    #${BAR_ID} {
                        position: fixed;
                        top: 12px;
                        left: 12px;
                        width: 300px;
                        z-index: 2147483647;
                        font-family: 'Segoe UI', system-ui, sans-serif;
                        opacity: 0.4;
                        transition: opacity .2s;
                    }
                    #${BAR_ID}:hover, #${BAR_ID}.focused { opacity: 1; }
                    #${BAR_ID}-input {
                        width: 100%;
                        height: 36px;
                        background: #1e1e1e;
                        border: 1px solid #333;
                        border-radius: 6px;
                        color: #f5f5f5;
                        padding: 0 12px;
                        font-size: 13px;
                        outline: none;
                        box-sizing: border-box;
                        caret-color: #6264A7;
                    }
                    #${BAR_ID}-input:focus {
                        border-color: #6264A7;
                    }
                    #${BAR_ID}-input::placeholder {
                        color: #666;
                    }
                    #${BAR_ID}-dropdown {
                        position: absolute;
                        top: 40px;
                        left: 0;
                        width: 100%;
                        max-height: 260px;
                        overflow-y: auto;
                        background: #1e1e1e;
                        border: 1px solid #333;
                        border-radius: 6px;
                        display: none;
                        box-shadow: 0 8px 24px rgba(0,0,0,0.5);
                    }
                    #${BAR_ID}-dropdown.visible { display: block; }
                    .rt-bot-item {
                        padding: 8px 12px;
                        cursor: pointer;
                        font-size: 12px;
                        border-bottom: 1px solid #2a2a2a;
                        display: flex;
                        justify-content: space-between;
                        align-items: center;
                    }
                    .rt-bot-item:hover { background: #2a2a2a; }
                    .rt-bot-item:last-child { border-bottom: none; }
                    .rt-bot-cmd { color: #6264A7; font-weight: 600; }
                    .rt-bot-desc { color: #888; font-size: 11px; }
                    .rt-bot-result {
                        padding: 10px 12px;
                        font-size: 12px;
                        color: #e0e0e0;
                        white-space: pre-wrap;
                        max-height: 200px;
                        overflow-y: auto;
                    }
                    .rt-bot-thinking {
                        color: #888;
                        font-style: italic;
                        padding: 8px 12px;
                        font-size: 12px;
                    }
                    .rt-bot-separator {
                        border-top: 1px solid #333;
                        margin: 4px 0;
                    }
                `;
                document.head.appendChild(style);
            }

            // Create bar
            const bar = document.createElement('div');
            bar.id = BAR_ID;
            bar.innerHTML = `
                <input id="${BAR_ID}-input" type="text" placeholder="Type / for commands..." autocomplete="off" spellcheck="false">
                <div id="${BAR_ID}-dropdown"></div>
            `;
            document.body.appendChild(bar);

            const input = bar.querySelector(`#${BAR_ID}-input`);
            const dropdown = bar.querySelector(`#${BAR_ID}-dropdown`);

            // Commands list (populated from Rust on startup)
            const commands = [
                { name: 'help', desc: 'List all commands' },
                { name: 'status', desc: 'Show pipeline status' },
                { name: 'translate', desc: 'Toggle translate on|off' },
                { name: 'meeting', desc: 'Toggle meeting start|stop' },
                { name: 'config', desc: 'Open config panel' },
                { name: 'clear', desc: 'Clear output' },
                { name: 'time', desc: 'Show current time' },
                { name: 'date', desc: 'Show current date' },
                { name: 'hello', desc: 'Welcome message' },
                { name: 'autoread', desc: 'Auto-read unread chats' },
                { name: 'browser', desc: 'Show/change link browser' },
            ];

            function showDropdown() {
                dropdown.classList.add('visible');
            }

            function hideDropdown() {
                dropdown.classList.remove('visible');
            }

            function renderCommandList(filter) {
                const filtered = filter
                    ? commands.filter(c => c.name.startsWith(filter.toLowerCase()))
                    : commands;
                if (filtered.length === 0) {
                    dropdown.innerHTML = '<div class="rt-bot-item rt-bot-desc">No matching commands</div>';
                    return;
                }
                dropdown.innerHTML = filtered.map(c =>
                    `<div class="rt-bot-item" data-cmd="${c.name}">
                        <span class="rt-bot-cmd">/${c.name}</span>
                        <span class="rt-bot-desc">${c.desc}</span>
                    </div>`
                ).join('');
                // Click handlers
                dropdown.querySelectorAll('.rt-bot-item[data-cmd]').forEach(el => {
                    el.addEventListener('click', () => {
                        input.value = '/' + el.dataset.cmd + ' ';
                        input.focus();
                        sendCommand(el.dataset.cmd);
                    });
                });
            }

            function renderResult(output) {
                if (!output) {
                    hideDropdown();
                    return;
                }
                dropdown.innerHTML = `<div class="rt-bot-result">${output.replace(/</g, '&lt;').replace(/>/g, '&gt;')}</div>`;
                showDropdown();
            }

            function renderThinking() {
                dropdown.innerHTML = '<div class="rt-bot-thinking">thinking...</div>';
                showDropdown();
            }

            function getIpc() {
                if (window.ipc && window.ipc.postMessage)
                    return window.ipc.postMessage.bind(window.ipc);
                if (window.chrome && window.chrome.webview && window.chrome.webview.postMessage)
                    return window.chrome.webview.postMessage.bind(window.chrome.webview);
                return null;
            }

            function sendCommand(fullInput) {
                const trimmed = fullInput.trim();
                if (!trimmed) return;
                const ipc = getIpc();
                if (ipc) {
                    renderThinking();
                    ipc(JSON.stringify({
                        type: 'bot_command',
                        data: { command: trimmed }
                    }));
                }
            }

            // Input events
            input.addEventListener('focus', () => {
                bar.classList.add('focused');
                if (input.value.startsWith('/')) {
                    const filter = input.value.slice(1).split(/\s/)[0];
                    renderCommandList(filter);
                    showDropdown();
                }
            });

            input.addEventListener('blur', () => {
                bar.classList.remove('focused');
                // Delay hide so click on dropdown item registers
                setTimeout(() => hideDropdown(), 200);
            });

            input.addEventListener('input', () => {
                if (input.value.startsWith('/')) {
                    const filter = input.value.slice(1).split(/\s/)[0];
                    renderCommandList(filter);
                    showDropdown();
                } else {
                    hideDropdown();
                }
            });

            input.addEventListener('keydown', (e) => {
                if (e.key === 'Enter') {
                    e.preventDefault();
                    const val = input.value.trim();
                    if (val) sendCommand(val);
                } else if (e.key === 'Escape') {
                    input.value = '';
                    hideDropdown();
                    input.blur();
                }
            });

            // Listen for bot responses from Rust
            window.addEventListener('rteams-bot-response', (e) => {
                const output = e.detail && e.detail.output ? e.detail.output : '';
                renderResult(output);
                input.value = '';
            });

            return bar;
        }

        // Initialize when DOM ready
        function initBarWhenReady() {
            if (!document.body) {
                setTimeout(initBarWhenReady, 50);
                return;
            }
            ensureBar();
        }

        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', initBarWhenReady);
        } else {
            initBarWhenReady();
        }

        window.addEventListener('load', () => {
            if (!document.getElementById(BAR_ID)) {
                initBarWhenReady();
            }
        });
    })();
    "#.to_string()
}
