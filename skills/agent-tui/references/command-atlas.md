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
- `agent-tui sessions list`
- `agent-tui sessions show <id>`
- `agent-tui sessions switch <id>`
- `agent-tui sessions attach` (use `-s <id>` to target)
  - `-T, --no-tty`: stream only.
  - `--detach-keys <keys>`: custom detach sequence (env: `AGENT_TUI_DETACH_KEYS`).
- `agent-tui sessions cleanup [--all]`
- `agent-tui sessions status`

### Live Preview
- `agent-tui live start [--open] [--browser <cmd>]`
- `agent-tui live status`
- `agent-tui live stop`
- Options:
  - `--open`: open UI in browser (uses `AGENT_TUI_UI_URL` if set).
  - `--browser <cmd>`: override `$BROWSER`.
- Deprecated:
  - `--listen`, `--allow-remote`, `--max-viewers` (use env vars and restart the daemon).

### Daemon
- `agent-tui daemon start [--foreground]`
- `agent-tui daemon stop [--force]`
- `agent-tui daemon status`
- `agent-tui daemon restart`

### Debugging
- `agent-tui health`
- `agent-tui env`
- `agent-tui version`
- `agent-tui help`

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
- `AGENT_TUI_SESSION_STORE`: session metadata log path (default: `~/.agent-tui/sessions.jsonl`).
- `AGENT_TUI_LOG`: log file path (optional).
- `AGENT_TUI_LOG_FORMAT`: log format (text or json; default: text).
- `AGENT_TUI_LOG_STREAM`: log output stream (stderr or stdout; default: stderr).
- `BROWSER`: browser command (overridden by `--browser`).
- `PORT`: fallback port for API listen when `AGENT_TUI_API_LISTEN` is unset.
