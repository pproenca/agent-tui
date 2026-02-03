# Command Atlas (Full)

Use this file when you need complete CLI coverage and exact options.

## Global Flags
- `--session <id>`: target a specific session (default: most recent).
- `--format <text|json>`: output format.
- `--json`: shorthand for `--format json`.
- `--no-color`: disable color (also respects `NO_COLOR`).
- `--verbose`: print request timing.

## Core Commands

### Run
- `agent-tui run <command> [-- args...]`
- Options:
  - `-d, --cwd <dir>`: working directory.
  - `--cols <n>`: terminal columns (default 120).
  - `--rows <n>`: terminal rows (default 40).

### Screenshot
- `agent-tui screenshot`
- Options:
  - `--region <name>`: limit capture to region (if supported).
  - `--strip-ansi`: remove ANSI color codes.
  - `--include-cursor`: include cursor position.

### Scroll
- `agent-tui scroll <up|down|left|right> [amount]`

### Resize / Restart
- `agent-tui resize --cols <n> --rows <n>`
- `agent-tui restart`

### Press / Type
- `agent-tui press <key...> [--hold|--release]`
- `agent-tui type "text"`
  - Keys: Enter, Tab, Escape, Backspace, Delete, Arrow keys, Home, End, PageUp, PageDown, F1-F12
  - Modifiers: Ctrl+<key>, Alt+<key>, Shift+<key>

### Wait
- `agent-tui wait <text>`
- `agent-tui wait --stable`
- Modifiers:
  - `-g, --gone`: wait for text to disappear.
  - `-t, --timeout <ms>`: timeout in milliseconds (default 30000).
  - `--assert`: exit code 1 on timeout (0 on success).

### Kill
- `agent-tui kill`

### Sessions
- `agent-tui sessions` (list)
- `agent-tui sessions show <id>`
- `agent-tui sessions attach [id]`
  - `-T, --no-tty`: stream only.
  - `--detach-keys <keys>`: custom detach sequence (env: `AGENT_TUI_DETACH_KEYS`).
- `agent-tui sessions cleanup [--all]`
- `agent-tui sessions status`

### Live Preview
- `agent-tui live start [--open]`
- `agent-tui live status`
- `agent-tui live stop`
- Options:
  - `--open`: open UI in browser (uses `AGENT_TUI_UI_URL` if set).
  - `--browser <cmd>`: override `$BROWSER`.

### Daemon
- `agent-tui daemon start [--foreground]`
- `agent-tui daemon stop [--force]`
- `agent-tui daemon status`
- `agent-tui daemon restart`

### Debugging
- `agent-tui env`
- `agent-tui version`

### Shell Completions
- `agent-tui completions <bash|zsh|fish|powershell|elvish>`

## Environment Variables
- `NO_COLOR`: disable colored output.
- `AGENT_TUI_DETACH_KEYS`: default detach keys for `sessions attach`.
- `AGENT_TUI_API_LISTEN`: live API bind address.
- `AGENT_TUI_API_ALLOW_REMOTE`: allow non-loopback bind (boolean).
- `AGENT_TUI_API_TOKEN`: override live API token (or "none" to disable).
- `AGENT_TUI_API_STATE`: state file path (default: `~/.agent-tui/api.json`).
- `AGENT_TUI_API_MAX_CONNECTIONS`: max live connections.
- `AGENT_TUI_UI_URL`: base URL to open with `live start --open`.
- `BROWSER`: browser command (overridden by `--browser`).
