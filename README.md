# agent-tui

**Let AI agents control any terminal application**

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Works with: **Claude Code** • **Cursor** • **Codex** • **Copilot** • Any AI agent

---

## Quick Start

### Install (30 seconds)

```bash
npm install -g agent-tui
```

Or with Homebrew:

```bash
brew install pproenca/tap/agent-tui
```

### Try the Built-in Demo

```bash
agent-tui demo                    # Start built-in demo TUI
agent-tui snapshot -i             # See detected elements
agent-tui fill @e1 "Hello World"  # Fill the input
agent-tui click @e3               # Click Submit
agent-tui kill                    # End session
```

### Automate Real Apps

```bash
agent-tui spawn htop              # Start any TUI app
agent-tui snapshot -i             # See elements
agent-tui keystroke q             # Press 'q' to quit
agent-tui kill                    # End session
```

That's it. You're automating TUI apps.

---

![Demo: agent-tui automating create-next-app](assets/demo.gif)

---

## AI controlling AI

agent-tui can automate Claude Code itself:

```bash
# Spawn Claude Code
agent-tui spawn "claude --dangerously-skip-permissions"

# Wait for initialization
agent-tui wait ">" --timeout 120000

# Send a prompt to Claude
agent-tui type "Create hello.py that prints Hello World"
agent-tui keystroke Enter

# Wait for Claude to finish
agent-tui wait --stable --timeout 60000

# Verify the result
cat hello.py  # It works!

# Cleanup
agent-tui kill
```

This enables automated testing of Claude Code, CI/CD pipelines for AI-assisted development, and QA for AI coding assistants.

---

## Why agent-tui?

| Problem | Solution |
|---------|----------|
| Interactive CLI wizards block automation | agent-tui captures screens and detects elements |
| No API for terminal UIs | Element refs (`@e1`, `@e2`) make TUIs scriptable |
| AI can't "see" terminal state | Snapshots provide structured view of screen + elements |
| TUIs require human interaction | AI agents can now fill forms, click buttons, navigate menus |

---

## Installation

### npm (Recommended)

```bash
npm install -g agent-tui
```

### Homebrew

```bash
brew install pproenca/tap/agent-tui
```

### Cargo

```bash
cargo install agent-tui
```

### Requirements

- macOS or Linux
- Node.js 16+ (for npm install)

<details>
<summary><strong>Build from Source</strong></summary>

```bash
git clone https://github.com/pproenca/agent-tui.git
cd agent-tui/cli
cargo build --release

# Add to PATH
export PATH="$PATH:$(pwd)/target/release"
```

Requires Rust 1.70+.

</details>

---

## Commands

| Command | Description | Example |
|---------|-------------|---------|
| `demo` | Start built-in demo TUI | `agent-tui demo` |
| `spawn <cmd>` | Start TUI app | `agent-tui spawn htop` |
| `snapshot` | Get screen state | `agent-tui snapshot -i` |
| `click <ref>` | Activate element | `agent-tui click @e1` |
| `fill <ref> <value>` | Fill input | `agent-tui fill @e1 "text"` |
| `keystroke <key>` | Send key | `agent-tui keystroke "Ctrl+C"` |
| `type <text>` | Type literal text | `agent-tui type "hello"` |
| `wait <text>` | Wait for text | `agent-tui wait "Ready"` |
| `kill` | Terminate TUI | `agent-tui kill` |
| `sessions` | List sessions | `agent-tui sessions` |

<details>
<summary><strong>Full Command Reference</strong></summary>

### Session Management

| Command | Description | Example |
|---------|-------------|---------|
| `spawn <cmd>` | Start a new TUI session | `agent-tui spawn "npm init"` |
| `sessions` | List all active sessions | `agent-tui sessions` |
| `kill` | Terminate the active session | `agent-tui kill` |
| `health` | Check daemon status | `agent-tui health -v` |

### Screen & Element Detection

| Command | Description | Example |
|---------|-------------|---------|
| `snapshot` | Capture screen text | `agent-tui snapshot` |
| `snapshot -i` | Capture screen + detect elements | `agent-tui snapshot -i` |
| `screen` | Raw screen text only | `agent-tui screen` |

### Interaction

| Command | Description | Example |
|---------|-------------|---------|
| `fill <ref> <value>` | Fill an input field | `agent-tui fill @e1 "my-app"` |
| `click <ref>` | Click/activate an element | `agent-tui click @e1` |
| `keystroke <key>` | Send a single key | `agent-tui keystroke Enter` |
| `type <text>` | Type literal text | `agent-tui type "hello world"` |

### Waiting & Synchronization

| Command | Description | Example |
|---------|-------------|---------|
| `wait <text>` | Wait for text to appear | `agent-tui wait "Continue"` |
| `wait --stable` | Wait for screen to stabilize | `agent-tui wait --stable` |

</details>

---

## Element Types

Snapshots detect these interactive elements:

| Type | Description | Example Appearance |
|------|-------------|-------------------|
| `button` | Clickable buttons | `[OK]`, `<Submit>` |
| `input` | Text input fields | `[____________]` |
| `checkbox` | Checkboxes | `[x]`, `[ ]` |
| `radio` | Radio buttons | `(•)`, `( )` |
| `select` | Dropdowns | `▼ Option` |
| `menuitem` | Menu items | `> File` |
| `listitem` | List items | `• Item` |

---

## Snapshot Format

```
Elements:
@e1 [button] "Submit" (10,30) *focused*
@e2 [input:Name] "John" (5,10)
@e3 [checkbox] "Remember me" [x] (7,10)

Screen:
┌─────────────────────────────────────┐
│  Name: [John________________]       │
│  [x] Remember me                    │
│        [ OK ]    [Cancel]           │
└─────────────────────────────────────┘
```

---

## Keystroke Reference

```bash
# Navigation
agent-tui keystroke Tab
agent-tui keystroke Enter
agent-tui keystroke Escape

# Arrows
agent-tui keystroke Up / Down / Left / Right

# Control combinations
agent-tui keystroke "Ctrl+C"
agent-tui keystroke "Ctrl+D"

# Function keys
agent-tui keystroke F1 ... F12
```

---

## For AI Agents

### Key Patterns

1. **Always snapshot before interacting** — Element refs reset on each snapshot
2. **Use `wait --stable` for completion** — Waits until screen stops changing
3. **Handle timeouts gracefully** — Use `wait -t <ms>` with appropriate timeouts
4. **Use JSON output for parsing** — Add `-f json` when processing programmatically

### JSON Output

All commands support `-f json` for machine-readable output:

```bash
agent-tui snapshot -i -f json | jq '.elements'
agent-tui sessions -f json
```

### Named Sessions

Run multiple TUI apps simultaneously:

```bash
agent-tui spawn "npm init" --session project-a
agent-tui spawn "npm init" --session project-b

agent-tui --session project-a snapshot -i
agent-tui --session project-b fill @e1 "value"
```

For comprehensive AI agent integration, see [SKILL.md](./SKILL.md) and [AGENTS.md](./AGENTS.md).

---

## Architecture

<details>
<summary><strong>System Architecture Diagram</strong></summary>

```
┌─────────────────────────────────────────────────────────────────┐
│                     AI Agent (Claude, etc.)                     │
└─────────────────────────────────────────────────────────────────┘
                                │
                                │ CLI commands via Bash
                                v
┌─────────────────────────────────────────────────────────────────┐
│                    agent-tui CLI (Rust)                         │
│  - Fast startup (~30ms)                                         │
│  - Command parsing & validation                                 │
│  - IPC client to daemon                                         │
└─────────────────────────────────────────────────────────────────┘
                                │
                                │ JSON-RPC over Unix Socket
                                v
┌─────────────────────────────────────────────────────────────────┐
│                   agent-tui Daemon (Rust)                       │
│  - Native PTY management                                        │
│  - Terminal emulation                                           │
│  - Element detection                                            │
│  - Session management                                           │
└─────────────────────────────────────────────────────────────────┘
                                │
                                │ PTY I/O
                                v
┌─────────────────────────────────────────────────────────────────┐
│              Target TUI Application                             │
│  (Ink, Bubble Tea, Textual, ncurses, htop, vim, etc.)          │
└─────────────────────────────────────────────────────────────────┘
```

</details>

### Project Structure

<details>
<summary><strong>Directory Layout</strong></summary>

```
agent-tui/
├── cli/                          # Rust CLI + Daemon
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs               # Entry point
│       ├── commands.rs           # Command parsing
│       ├── connection.rs         # IPC client
│       ├── protocol.rs           # JSON-RPC types
│       └── daemon/               # Native daemon
│           ├── mod.rs            # Daemon entry point
│           ├── session.rs        # Session management
│           ├── pty.rs            # PTY handling
│           ├── terminal.rs       # Terminal emulation
│           └── detection/        # Element detection
│
├── docs/                         # Documentation site
├── assets/                       # Demo GIFs, images
└── README.md
```

</details>

---

## Development

```bash
cd cli
cargo build                    # Debug build
cargo build --release          # Release build
cargo test                     # Run tests
cargo fmt                      # Format code
cargo clippy                   # Lint
```

---

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `AGENT_TUI_TRANSPORT` | `unix` or `tcp` | unix |
| `AGENT_TUI_TCP_PORT` | TCP port | 19847 |
| `XDG_RUNTIME_DIR` | Socket directory | /tmp |

---

## Documentation

- [SKILL.md](./SKILL.md) — Full guide for AI agent integration
- [AGENTS.md](./AGENTS.md) — Quick reference for AI agents
- [docs/](./docs/) — Full documentation site

---

## License

MIT
