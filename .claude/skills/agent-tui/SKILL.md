---
name: agent-tui
description: Automates TUI/CLI interactions for project wizards, terminal UIs, and interactive shell tools. Use when the user needs to interact with terminal apps like create-next-app, npm init, htop, vim, or any interactive CLI.
allowed-tools: Bash(agent-tui:*)
---

# agent-tui Skill

## Overview

agent-tui enables AI agents to programmatically interact with TUI (Terminal User Interface) applications. This skill provides patterns and best practices for automating terminal applications from Claude Code.

## When to Use

Use this skill when you need to:
- Automate interactive CLI wizards (e.g., `npx create-next-app`)
- Control TUI applications (e.g., htop, vim, nano)
- Test terminal-based interfaces
- Navigate menu systems programmatically
- Fill forms in terminal applications

## Core Workflow

The standard agent-tui workflow follows this pattern:

```bash
# 1. Spawn a TUI application
agent-tui spawn "npx create-next-app"

# 2. Take a snapshot to see current state and elements
agent-tui snapshot -i

# 3. Interact using element refs (@e1, @e2, @e3)
agent-tui fill @e1 "my-project-name"
agent-tui keystroke Tab
agent-tui click @e2

# 4. Wait for changes
agent-tui wait "Success"

# 5. Re-snapshot after screen changes
agent-tui snapshot -i

# 6. Clean up when done
agent-tui kill
```

## Element Refs

Element refs are simple sequential identifiers assigned per snapshot:
- `@e1`, `@e2`, `@e3`, ... - Elements in document order (top-to-bottom, left-to-right)
- Refs reset on each snapshot
- Always use the latest snapshot's refs

Example snapshot output (`agent-tui snapshot -i -f tree`):
```
- button "Submit" [ref=@e1]
- input "Project name" [ref=@e2] [focused]
- checkbox "Use TypeScript" [ref=@e3] [checked]
- select "Package Manager" [ref=@e4]
```

## Commands

### Core Commands

| Command | Description |
|---------|-------------|
| `spawn <cmd>` | Start a TUI application |
| `snapshot -i` | Take snapshot with element detection |
| `click @ref` | Click/activate an element |
| `fill @ref "value"` | Fill an input field |
| `type "text"` | Type literal text |
| `keystroke <key>` | Send a keystroke (Enter, Tab, Ctrl+C, etc.) |
| `wait "text"` | Wait for text to appear |
| `wait --stable` | Wait for screen to stabilize |
| `kill` | Terminate the session |

### Query Commands

| Command | Description |
|---------|-------------|
| `get-focused` | Get the currently focused element |
| `get-title` | Get session title/command |
| `get-text @ref` | Get element's text content |
| `get-value @ref` | Get input element's value |
| `is-visible @ref` | Check if element is visible |
| `is-focused @ref` | Check if element is focused |

### Navigation Commands

| Command | Description |
|---------|-------------|
| `scroll down` | Scroll down (arrow keys) |
| `scroll up` | Scroll up |
| `scrollintoview @ref` | Scroll until element is visible |
| `focus @ref` | Focus an element |

### Wait Conditions

| Condition | Description |
|-----------|-------------|
| `wait "text"` | Wait for text to appear |
| `wait --element @ref` | Wait for element to appear |
| `wait --visible @ref` | Same as --element |
| `wait --focused @ref` | Wait for element to be focused |
| `wait --not-visible @ref` | Wait for element to disappear |
| `wait --stable` | Wait for screen to stop changing |
| `wait --value @ref=val` | Wait for input to have value |

## Best Practices

1. **Always snapshot before interacting**: Elements may have changed since the last snapshot
2. **Use wait after actions**: Give the TUI time to update before re-snapshotting
3. **Handle dynamic content**: Use `--stable` wait for content that loads asynchronously
4. **Check for errors**: Use `snapshot -i` to see AI-friendly error hints

## Example: Automating Claude Code

```bash
#!/bin/bash
# Demo: Claude Code controlling itself

# Start Claude Code
agent-tui spawn "claude --dangerously-skip-permissions"

# Wait for prompt
agent-tui wait ">" --timeout 120000

# Take snapshot to see current state
agent-tui snapshot -i -f tree

# Type a task
agent-tui type "Create a file called hello.py that prints Hello World"

# Submit
agent-tui keystroke Enter

# Wait for completion
agent-tui wait --stable --timeout 60000

# Capture result
agent-tui snapshot -i

# Clean up
agent-tui kill
```

## Error Handling

agent-tui provides AI-friendly error messages with hints:

- **Element not found**: "Element @e5 not found. Run 'snapshot -i' to see current elements."
- **No active session**: "No active session. Run 'sessions' to list or 'spawn <cmd>' to start."
- **Timeout**: "Timeout waiting. Try 'wait --stable' or increase timeout with '-t'."

## See Also

- [agent-tui documentation](https://github.com/pproenca/agent-tui)
- `/onboard` command for interactive tour
