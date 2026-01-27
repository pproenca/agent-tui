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
  - `-e, --elements`: include detected elements and refs.
  - `-a, --accessibility`: output accessibility-tree format.
  - `--interactive-only`: only interactive elements (requires `-a`).
  - `--region <name>`: limit capture to region (if supported).
  - `--strip-ansi`: remove ANSI color codes.
  - `--include-cursor`: include cursor position.

### Find / Count
- `agent-tui find [filters]`
- `agent-tui count [filters]`
- Filters:
  - `--role <ROLE>`
  - `--name <NAME>`
  - `--text <TEXT>`
  - `--placeholder <TEXT>` (find only)
  - `--focused` (find only)
  - `--nth <N>` (find only)
  - `--exact` (find only; exact text match)

### Resize / Restart
- `agent-tui resize --cols <n> --rows <n>`
- `agent-tui restart`

### Scroll Into View
- `agent-tui scroll-into-view @ref`

### Action (element operations)
- `agent-tui action @ref [operation]`
- Operations:
  - `click`
  - `dblclick`
  - `fill "value"`
  - `select "Option" ["Option2" ...]`
  - `toggle [on|off]`
  - `focus`
  - `clear`
  - `selectall`
  - `scroll <up|down|left|right> [amount]`

### Press / Type / Input
- `agent-tui press <key...>`
- `agent-tui type "text"`
- `agent-tui input <key|text> [--hold|--release]`
  - Keys: Enter, Tab, Escape, Backspace, Delete, Arrow keys, Home, End, PageUp, PageDown, F1-F12
  - Modifiers: Ctrl+<key>, Alt+<key>, Shift+<key>

### Wait
- `agent-tui wait <text>`
- `agent-tui wait -e @ref`
- `agent-tui wait --focused @ref`
- `agent-tui wait --stable`
- `agent-tui wait --value @ref=VALUE`
- Modifiers:
  - `-g, --gone`: wait for text/element to disappear.
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

## Selector Shortcuts (external subcommand)
- `agent-tui @e1` (click)
- `agent-tui :Submit` (click by partial text)
- `agent-tui '@"Exact Text"'` (click by exact text)
- `agent-tui @e1 fill "value"`
- `agent-tui @e1 toggle on`
- `agent-tui @e1 choose "Option"` (maps to select)
- `agent-tui @e1 clear`
- `agent-tui @e1 focus`

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
