# Floating Command Bar (Bot Commands) — Design Spec

## Overview

Add a floating command bar to the Rust Teams window. Users type `/` to see available commands, select one, and get results displayed in an expanded dropdown. Built-in commands for controlling translate, meeting, status, and fun utilities. No AI dependency.

## Motivation

- Quick command input without navigating panels
- Telegram-style `/` command experience
- Independent of Teams' built-in bot system

## Components

### 1. Floating Command Bar

- **Position:** `fixed`, top-left corner of Teams window (`top: 12px`, `left: 12px`)
- **Size:** ~300px wide, 36px height
- **Styling:** Dark background (`#1e1e1e`), border on focus, `z-index: 2147483647`
- **Placeholder:** `Type / for commands...`
- **Behavior:**
  - Always visible (no auto-hide)
  - Focus → border highlight (`#6264A7`)
  - Blur → reduced opacity (0.4), hover restores to 1.0
  - Escape → close dropdown, blur input

### 2. Command Dropdown

- **Trigger:** User types `/` in input
- **Position:** Below input, same width
- **Content:** List of available commands with name + description
- **Filter:** Typing after `/` filters commands by name (fuzzy match)
- **Selection:** Click or Enter to execute
- **Result display:** Dropdown expands to show command output (max-height: 200px, scrollable)
- **Loading state:** Show "thinking..." text while command executes
- **Clear:** `/clear` or Escape clears dropdown content

### 3. Built-in Commands

| Command | Description | Args |
|---------|-------------|------|
| `/help` | List all commands | none |
| `/status` | Show pipeline status, meeting state, whisper availability | none |
| `/translate on` | Start translate pipeline (WASAPI loopback) | `on` or `off` |
| `/translate off` | Stop translate pipeline | |
| `/meeting start` | Start meeting notes recording | `start` or `stop` |
| `/meeting stop` | Stop meeting notes recording | |
| `/config` | Open configure panel (API keys) | none |
| `/clear` | Clear dropdown content | none |
| `/time` | Show current time | none |
| `/date` | Show current date | none |
| `/hello` | Show welcome message | none |

## Architecture

### New Module: `src/bot/`

```
src/bot/
  mod.rs          — Module root, exports CommandRegistry
  commands.rs     — Built-in command definitions + handlers
  parser.rs       — Parse "/command args" from input string
```

### `CommandRegistry`

```rust
pub struct CommandRegistry {
    commands: HashMap<String, Box<dyn Fn(&str) -> CommandResult + Send + Sync>>,
}

pub struct CommandResult {
    pub output: String,
    pub async_state: Option<String>,  // e.g. "thinking..."
}
```

- Registered on startup with all built-in commands
- `execute(command: &str, args: &str) -> CommandResult`
- Sync commands return immediately
- Async commands (meeting start/stop) return `async_state: Some("thinking...")` and send result later via panel_state channel

### Parser

```rust
pub fn parse_command(input: &str) -> Option<(&str, &str)>
```

- Input: `"/translate on"` → returns `("translate", "on")`
- Input: `"hello"` → returns `None` (not a command)
- Input: `"/"` → returns `("", "")` (show all commands)

### IPC Flow

```
JS Input → "bot_command" IPC → Rust CommandRegistry::execute()
                                    ↓
                              CommandResult { output }
                                    ↓
                              bot_response channel → event loop
                                    ↓
                              JS "rteams-bot-response" CustomEvent
                                    ↓
                              Render in dropdown
```

### JS Integration

- **Input field:** Added to floating command bar DOM
- **Event listener:** `keydown` on input — Enter executes, Escape closes
- **Dropdown rendering:** Filtered command list, click to execute
- **Result display:** `rteams-bot-response` event listener renders output

## CSS

```css
#rteams-bot-bar {
    position: fixed;
    top: 12px;
    left: 12px;
    width: 300px;
    z-index: 2147483647;
    font-family: 'Segoe UI', system-ui, sans-serif;
}

#rteams-bot-input {
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
}

#rteams-bot-input:focus {
    border-color: #6264A7;
}

#rteams-bot-dropdown {
    position: absolute;
    top: 40px;
    left: 0;
    width: 100%;
    max-height: 260px;
    overflow-y: auto;
    background: #2a2a2a;
    border: 1px solid #444;
    border-radius: 6px;
    display: none;
    box-shadow: 0 8px 24px rgba(0,0,0,0.5);
}

#rteams-bot-dropdown.visible { display: block; }

.rt-bot-item {
    padding: 8px 12px;
    cursor: pointer;
    font-size: 12px;
    border-bottom: 1px solid #333;
}

.rt-bot-item:hover { background: #3a3a3a; }

.rt-bot-cmd { color: #6264A7; font-weight: 600; }
.rt-bot-desc { color: #888; font-size: 11px; }
.rt-bot-result { color: #e0e0e0; padding: 10px 12px; font-size: 12px; }
.rt-bot-thinking { color: #888; font-style: italic; }
```

## Error Handling

- Unknown command → "Unknown command: /foo. Type /help for available commands."
- Missing args → "Usage: /translate on|off"
- Pipeline not running → "Pipeline is not running. Use /translate on to start."
- Whisper not found → "Whisper not found. Use 🖥 Local to setup."
- Meeting already started → "Meeting is already in progress."

## Testing

- Parser: valid commands, invalid commands, empty input
- Registry: execute each built-in command, verify output
- JS: dropdown shows on `/`, filters correctly, executes on Enter
- IPC: bot_command message → bot_response event round-trip
- Error cases: unknown command, missing args

## Non-Goals

- No AI Q&A (user chose built-in only)
- No custom user-defined commands
- No external bot integration
- No keyboard shortcuts for commands
- No persistent command history
