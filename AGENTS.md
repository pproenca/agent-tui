# AGENTS.md

Quick reference for AI agents using agent-tui.

## What is agent-tui?

agent-tui enables AI agents to interact with interactive CLI/TUI applications programmatically. It provides:
- Screen capture with element detection
- Element refs (`@e1`, `@e2`) for addressing UI components
- Commands to fill inputs, click buttons, send keystrokes
- Waiting/synchronization primitives

## Optimal Workflow

```bash
# 1. Start application
agent-tui spawn "your-command"

# 2. Wait for UI to load
agent-tui wait --stable --timeout 5000

# 3. Take snapshot with element detection
agent-tui snapshot -i

# 4. Interact with elements
agent-tui fill @e1 "value"
agent-tui keystroke Enter

# 5. Wait for result
agent-tui wait --stable

# 6. Repeat 3-5 as needed

# 7. Cleanup
agent-tui kill
```

## Essential Commands

| Command | Purpose | Example |
|---------|---------|---------|
| `spawn <cmd>` | Start TUI | `agent-tui spawn "npm init"` |
| `snapshot -i` | See screen + elements | `agent-tui snapshot -i` |
| `fill <ref> <val>` | Fill input | `agent-tui fill @e1 "text"` |
| `click <ref>` | Click element | `agent-tui click @e1` |
| `keystroke <key>` | Send key | `agent-tui keystroke Enter` |
| `type <text>` | Type text | `agent-tui type "hello"` |
| `wait <text>` | Wait for text | `agent-tui wait "Done"` |
| `wait --stable` | Wait for screen to stop changing | `agent-tui wait --stable` |
| `kill` | End session | `agent-tui kill` |

## Element Refs

Snapshots assign refs like `@e1`, `@e2` to detected elements:

```
Elements:
@e1 [input:name] "" (3,25) *focused*
@e2 [button] "Submit" (5,25)
@e3 [checkbox] "Agree" [ ] (7,10)
```

**Important:** Refs change between snapshots. Always take a fresh snapshot before interacting.

## Common Patterns

### Fill a form wizard

```bash
agent-tui spawn "npx create-react-app"
agent-tui wait "project name"
agent-tui snapshot -i
agent-tui fill @e1 "my-app"
agent-tui keystroke Enter
agent-tui wait --stable
agent-tui kill
```

### Navigate menus

```bash
agent-tui spawn htop
agent-tui wait --stable
agent-tui keystroke F10
agent-tui snapshot -i
agent-tui keystroke ArrowDown
agent-tui keystroke Enter
agent-tui kill
```

### Automate Claude Code

```bash
agent-tui spawn "claude --dangerously-skip-permissions"
agent-tui wait ">" --timeout 120000
agent-tui type "Create hello.py that prints Hello World"
agent-tui keystroke Enter
agent-tui wait --stable --timeout 60000
cat hello.py  # Verify output
agent-tui kill
```

### Form with Multiple Fields

```bash
agent-tui spawn "./setup.sh"
agent-tui wait "Enter your details"
agent-tui snapshot -i

# Fill first field
agent-tui fill @e1 "John Doe"
agent-tui keystroke Tab

# Snapshot again after UI change
agent-tui snapshot -i
agent-tui fill @e2 "john@example.com"
agent-tui keystroke Enter

agent-tui wait "Success"
agent-tui kill
```

## Key Rules

1. **Always snapshot before interacting** - Get fresh element refs
2. **Use `wait --stable` after actions** - UI may take time to update
3. **Set appropriate timeouts** - Slow operations need longer waits
4. **Handle errors gracefully** - Check if commands succeed
5. **Always cleanup** - Run `agent-tui kill` when done

## Keystrokes

```bash
# Navigation
agent-tui keystroke Tab
agent-tui keystroke Enter
agent-tui keystroke Escape
agent-tui keystroke ArrowUp / ArrowDown / ArrowLeft / ArrowRight

# Modifiers
agent-tui keystroke "Ctrl+C"
agent-tui keystroke "Ctrl+D"
agent-tui keystroke "Shift+Tab"

# Function keys
agent-tui keystroke F1 ... F12
```

## JSON Output

For programmatic processing, add `-f json`:

```bash
agent-tui snapshot -i -f json
agent-tui sessions -f json
```

Example: Extract only input elements:
```bash
agent-tui snapshot -i -f json | jq '.elements[] | select(.type == "input")'
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Element not found | Take fresh snapshot - refs may have changed |
| Timeout | Increase timeout: `--timeout 60000` |
| No active session | Run `agent-tui sessions` to check state |
| Stale refs | Refs change on UI updates - re-snapshot |
| Daemon unresponsive | Run `agent-tui health -v` to check status |

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `AGENT_TUI_TRANSPORT` | `unix` | `unix` or `tcp` |
| `AGENT_TUI_TCP_PORT` | `19847` | TCP port if using tcp transport |

## Limitations

- Cannot interact with GUI applications (X11, Wayland)
- Limited support for complex terminal graphics
- Element detection works best with standard TUI frameworks
- Very fast UI updates may be missed between snapshots

## More Information

- [README.md](./README.md) - Overview and installation
- [SKILL.md](./SKILL.md) - Comprehensive guide with decision trees and advanced patterns
