# agent-tui Commands Reference

Complete reference for all agent-tui CLI commands.

## Session Management

| Command | Description | Example |
|---------|-------------|---------|
| `spawn <cmd>` | Start a new TUI session | `agent-tui spawn "npm init"` |
| `sessions` | List all active sessions | `agent-tui sessions` |
| `kill` | Terminate the active session | `agent-tui kill` |
| `attach <id>` | Attach to existing session | `agent-tui attach abc123` |
| `health` | Check daemon status | `agent-tui health -v` |
| `cleanup` | Remove stale sessions | `agent-tui cleanup --all` |

## Screen & Element Detection

| Command | Description | Example |
|---------|-------------|---------|
| `snapshot` | Capture screen text | `agent-tui snapshot` |
| `snapshot -i` | Capture screen + detect elements | `agent-tui snapshot -i` |
| `snapshot -i -c` | Compact element view | `agent-tui snapshot -i -c` |
| `snapshot -i --format tree` | Accessibility-tree format | `agent-tui snapshot -i --format tree` |
| `screen` | Raw screen text only | `agent-tui screen` |
| `find` | Find elements semantically | `agent-tui find --role button --name "Submit"` |

## Interaction

| Command | Description | Example |
|---------|-------------|---------|
| `fill <ref> <value>` | Fill an input field | `agent-tui fill @inp1 "my-app"` |
| `click <ref>` | Click/activate an element | `agent-tui click @btn1` |
| `keystroke <key>` | Send a single key | `agent-tui keystroke Enter` |
| `type <text>` | Type literal text | `agent-tui type "hello world"` |
| `select <ref> <opt>` | Select dropdown option | `agent-tui select @sel1 "Option A"` |
| `toggle <ref>` | Toggle checkbox/radio | `agent-tui toggle @cb1` |
| `focus <ref>` | Focus an element | `agent-tui focus @inp2` |
| `clear <ref>` | Clear an input field | `agent-tui clear @inp1` |
| `scroll <dir>` | Scroll in a direction | `agent-tui scroll down` |

## Waiting & Synchronization

| Command | Description | Example |
|---------|-------------|---------|
| `wait <text>` | Wait for text to appear | `agent-tui wait "Continue"` |
| `wait --stable` | Wait for screen to stabilize | `agent-tui wait --stable` |
| `wait --element <ref>` | Wait for element to appear | `agent-tui wait --element @btn1` |
| `wait --focused <ref>` | Wait for element to be focused | `agent-tui wait --focused @inp1` |
| `wait --not-visible <ref>` | Wait for element to disappear | `agent-tui wait --not-visible @modal1` |
| `wait --text-gone <text>` | Wait for text to disappear | `agent-tui wait --text-gone "Loading"` |

## Element State Queries

| Command | Description | Example |
|---------|-------------|---------|
| `get-text <ref>` | Get element text content | `agent-tui get-text @btn1` |
| `get-value <ref>` | Get input field value | `agent-tui get-value @inp1` |
| `is-visible <ref>` | Check if element is visible | `agent-tui is-visible @btn1` |
| `is-focused <ref>` | Check if element is focused | `agent-tui is-focused @inp1` |

## Recording & Debugging

| Command | Description | Example |
|---------|-------------|---------|
| `record-start` | Start recording session | `agent-tui record-start` |
| `record-stop` | Stop and save recording | `agent-tui record-stop -o session.json` |
| `trace` | Show recent interactions | `agent-tui trace -n 20` |
| `console` | Show terminal output | `agent-tui console -n 50` |
| `resize` | Resize terminal | `agent-tui resize --cols 120 --rows 40` |

## Diagnostics

| Command | Description | Example |
|---------|-------------|---------|
| `version` | Show CLI and daemon version | `agent-tui version` |
| `env` | Show environment configuration | `agent-tui env` |
| `assert <cond>` | Assert a condition | `agent-tui assert text:Success` |

## Global Options

| Option | Description |
|--------|-------------|
| `-s, --session <id>` | Specify session ID |
| `-f, --format <fmt>` | Output format: text, json, tree |
| `--no-color` | Disable colored output |
| `-v, --verbose` | Show request timing |
| `--debug` | Show full request/response details |

## Keystroke Reference

| Key | Usage |
|-----|-------|
| `Enter` | Confirm/submit |
| `Tab` | Next field |
| `Escape` | Cancel/close |
| `ArrowUp/Down/Left/Right` | Navigation |
| `Ctrl+C` | Interrupt/cancel |
| `F1`-`F12` | Function keys |
| `Backspace`, `Delete` | Delete characters |
| `Home`, `End` | Line navigation |
| `PageUp`, `PageDown` | Page navigation |
