---
name: agent-tui
description: Automates TUI/CLI interactions for project wizards, terminal UIs, and interactive shell tools. Use when the user needs to interact with interactive CLI applications like create-next-app, npm init, htop, vim, or any interactive terminal program.
allowed-tools: Bash(agent-tui:*)
---

# TUI Automation with agent-tui

## Quick start

```bash
agent-tui spawn "npx create-next-app"  # Start TUI
agent-tui wait --stable                 # Wait for ready
agent-tui snapshot -i                   # See screen + elements
agent-tui fill @inp1 "my-project"       # Fill input
agent-tui keystroke Enter               # Confirm
```

## Core workflow

1. **spawn** - Start the TUI application
2. **wait** - Wait for ready state (use `--stable` for dynamic UIs)
3. **snapshot -i** - Get screen content with interactive elements (@refs)
4. **interact** - Use `fill`, `click`, `keystroke`, `type` with @refs
5. **wait** - Wait for UI to update after interaction
6. **repeat** steps 3-5 as needed
7. **kill** - Close the session when done

---

## Commands

### Session management

| Command | Description | Example |
|---------|-------------|---------|
| `spawn <cmd>` | Start a new TUI session | `agent-tui spawn "npm init"` |
| `sessions` | List all active sessions | `agent-tui sessions` |
| `kill` | Terminate the active session | `agent-tui kill` |
| `health` | Check daemon status | `agent-tui health -v` |
| `cleanup` | Remove stale sessions | `agent-tui cleanup --all` |

### Snapshot (screen analysis)

| Command | Description | Example |
|---------|-------------|---------|
| `snapshot` | Capture screen text | `agent-tui snapshot` |
| `snapshot -i` | Capture screen + detect elements | `agent-tui snapshot -i` |
| `snapshot -i -c` | Compact view (less noise) | `agent-tui snapshot -i -c` |
| `snapshot --strip-ansi` | Plain text without colors | `agent-tui snapshot --strip-ansi` |
| `snapshot --include-cursor` | Include cursor position | `agent-tui snapshot --include-cursor` |

### Interactions (use @refs from snapshot)

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

### Get information

| Command | Description | Example |
|---------|-------------|---------|
| `get-text <ref>` | Get element text content | `agent-tui get-text @btn1` |
| `get-value <ref>` | Get input field value | `agent-tui get-value @inp1` |

### Check state

| Command | Description | Example |
|---------|-------------|---------|
| `is-visible <ref>` | Check if element is visible | `agent-tui is-visible @btn1` |
| `is-focused <ref>` | Check if element is focused | `agent-tui is-focused @inp1` |

### Wait

| Command | Description | Example |
|---------|-------------|---------|
| `wait <text>` | Wait for text to appear | `agent-tui wait "Continue"` |
| `wait --stable` | Wait for screen to stabilize | `agent-tui wait --stable` |
| `wait --element <ref>` | Wait for element to appear | `agent-tui wait --element @btn1` |
| `wait --text-gone <text>` | Wait for text to disappear | `agent-tui wait --text-gone "Loading"` |

### Debugging

| Command | Description | Example |
|---------|-------------|---------|
| `trace` | Show recent interactions | `agent-tui trace -n 20` |
| `console` | Show terminal output | `agent-tui console -n 50` |
| `record-start` | Start recording session | `agent-tui record-start` |
| `record-stop` | Stop and save recording | `agent-tui record-stop -o session.json` |
| `assert <cond>` | Assert a condition | `agent-tui assert text:Success` |

---

## Element refs

Element refs are identifiers assigned to detected UI elements:

| Ref Pattern | Element Type | Example |
|-------------|--------------|---------|
| `@btn1`, `@btn2` | Buttons | `agent-tui click @btn1` |
| `@inp1`, `@inp2` | Input fields | `agent-tui fill @inp1 "value"` |
| `@sel1`, `@sel2` | Select/dropdown | `agent-tui select @sel1 "Option"` |
| `@cb1`, `@cb2` | Checkboxes | `agent-tui toggle @cb1` |
| `@rb1`, `@rb2` | Radio buttons | `agent-tui click @rb1` |
| `@mi1`, `@mi2` | Menu items | `agent-tui click @mi1` |

**Important:** Element refs can change between snapshots as the UI updates. Always take a fresh snapshot before interacting with elements.

---

## Keystroke reference

| Key | Usage |
|-----|-------|
| `Enter` | Confirm/submit |
| `Tab` | Next field |
| `Escape` | Cancel/close |
| `ArrowUp/Down/Left/Right` | Navigation |
| `Ctrl+C` | Interrupt/cancel |
| `Ctrl+A` | Select all |
| `F1`-`F12` | Function keys |
| `Backspace`, `Delete` | Delete characters |
| `Home`, `End` | Line navigation |
| `PageUp`, `PageDown` | Page navigation |

---

## Example: Interactive wizard

```bash
# Create a Next.js app
agent-tui spawn "npx create-next-app"
agent-tui wait "project name" --timeout 30000
agent-tui snapshot -i
agent-tui fill @inp1 "my-app"
agent-tui keystroke Enter
agent-tui wait --stable
agent-tui snapshot -i  # Fresh refs after UI update
# Continue answering prompts...
agent-tui kill
```

---

## Example: Claude Code automation

```bash
# Spawn Claude Code in permissive mode
agent-tui spawn "claude --dangerously-skip-permissions"

# Wait for initialization (the ">" prompt)
agent-tui wait ">" --timeout 120000

# Send a prompt
agent-tui type "Create hello.py that prints Hello World"
agent-tui keystroke Enter

# Wait for completion (screen stabilizes when Claude finishes)
agent-tui wait --stable --timeout 60000

# Verify result
agent-tui snapshot -i
cat hello.py

# Cleanup
agent-tui kill
```

### Multi-turn conversation

```bash
agent-tui spawn "claude --dangerously-skip-permissions"
agent-tui wait ">" --timeout 120000

# First prompt
agent-tui type "Create a Python function that reverses a string"
agent-tui keystroke Enter
agent-tui wait --stable --timeout 60000

# Follow-up
agent-tui type "Now add unit tests for that function"
agent-tui keystroke Enter
agent-tui wait --stable --timeout 60000

# Verify files were created
ls -la *.py

agent-tui kill
```

---

## Sessions (parallel TUIs)

```bash
# Start multiple sessions
agent-tui spawn "htop" --session monitoring
agent-tui spawn "npm run dev" --session dev-server

# Interact with specific session
agent-tui snapshot -i --session monitoring
agent-tui keystroke q --session monitoring

# List all sessions
agent-tui sessions

# Kill specific session
agent-tui kill --session dev-server
```

---

## JSON output (for parsing)

```bash
# Get structured element data
agent-tui snapshot -i -f json

# Parse session info
agent-tui sessions -f json

# Get health status programmatically
agent-tui health -f json
```

---

## Tips for AI agents

1. **Always snapshot first** - Before any interaction, take a snapshot to see current state
2. **Use wait after interactions** - UI may take time to update after input
3. **Element refs change** - Take a fresh snapshot when UI changes significantly
4. **Clean up sessions** - Always kill sessions when done to free resources
5. **Use JSON for parsing** - When processing output programmatically, use `-f json`
6. **Handle timeouts** - Use appropriate timeout values for slow operations
7. **Check health** - If commands fail, check `agent-tui health` for daemon status

---

## Common mistakes

### Not waiting for UI updates

```bash
# WRONG
agent-tui spawn htop
agent-tui click @btn1  # May fail - UI not loaded yet

# CORRECT
agent-tui spawn htop
agent-tui wait --stable --timeout 5000
agent-tui snapshot -i
agent-tui click @btn1
```

### Using stale element refs

```bash
# WRONG
agent-tui snapshot -i  # @btn1 = "Submit"
agent-tui fill @inp1 "value"
agent-tui keystroke Tab  # UI changes, refs shift!
agent-tui click @btn1    # May click wrong element!

# CORRECT
agent-tui snapshot -i
agent-tui fill @inp1 "value"
agent-tui keystroke Tab
agent-tui wait --stable
agent-tui snapshot -i  # Get fresh refs
agent-tui click @btn1
```

---

## Troubleshooting

### Daemon won't start

```bash
ps aux | grep agent-tui          # Check for existing daemon
rm -f /tmp/agent-tui.sock        # Remove stale socket
agent-tui health -v              # Restart daemon
```

### Element not found

1. Take fresh snapshot: `agent-tui snapshot -i`
2. Check if UI changed (refs shift when screen updates)
3. Verify ref format: Correct `@btn1`, Wrong `btn1`
4. Wait before action: `agent-tui wait --element @btn1`

### Wait timeout

```bash
agent-tui sessions               # Check if process is running
agent-tui snapshot               # See current state
agent-tui wait --stable          # Use stable wait for dynamic UIs
```

---

## Environment variables

| Variable | Description | Default |
|----------|-------------|---------|
| `AGENT_TUI_TRANSPORT` | Transport type (unix/tcp) | unix |
| `AGENT_TUI_TCP_PORT` | TCP port for daemon | 19847 |
| `AGENT_TUI_LOG_LEVEL` | Daemon log level | info |
| `AGENT_TUI_LOG_FILE` | Log file path | (console) |
