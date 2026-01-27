# agent-tui

CLI tool for AI agents to interact with TUI (Terminal User Interface) applications.

**agent-tui** enables AI agents to programmatically drive terminal applications by capturing screenshots, detecting UI elements, and sending input—making TUI automation accessible to LLM-powered agents.

## Features

- **Virtual Terminal Emulation** - Run TUI apps in isolated PTY sessions with full terminal emulation
- **Visual Object Model (VOM)** - AI-native UI element detection based on visual styling, not text patterns
- **Element References** - Interact with elements via refs (`@e1`, `@btn1`, `@inp1`) with role-based classification
- **Text Selectors** - Find elements by exact (`@"Submit"`) or partial (`:Submit`) text matching
- **Interactive Actions** - Fill inputs, toggle checkboxes, select options, click buttons, scroll
- **Wait Conditions** - Wait for text, element states, focus changes, or stability
- **Output Formats** - Human-readable text or JSON for automation pipelines
- **Live Preview API** - HTTP + WebSocket endpoints for real-time UI monitoring
- **Session Management** - Background daemon manages multiple concurrent TUI sessions

## Installation

### Quick Install

```bash
curl -fsSL https://raw.githubusercontent.com/pproenca/agent-tui/master/install.sh | bash
```

The installer detects your platform and installs the appropriate binary to `~/.local/bin`.

### Package Managers

```bash
# npm
npm install -g agent-tui

# pnpm
pnpm add -g agent-tui

# bun
bun add -g agent-tui
```

### From Source

```bash
git clone https://github.com/pproenca/agent-tui
cd agent-tui/cli
cargo build --release
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `AGENT_TUI_INSTALL_DIR` | Custom install location (default: `~/.local/bin`) |
| `AGENT_TUI_VERSION` | Install specific version |
| `AGENT_TUI_SKIP_PM` | Skip package manager, use binary download |
| `AGENT_TUI_SKIP_VERIFY` | Skip checksum verification |

## Quick Start

```bash
# Start the daemon
agent-tui daemon start

# Run a TUI application
agent-tui run htop

# Take a screenshot with element detection
agent-tui screenshot -e

# Find and interact with elements
agent-tui find --role button
agent-tui action @btn1 click

# Send keyboard input
agent-tui press Enter
agent-tui type "hello world"

# Wait for conditions
agent-tui wait "Loading complete"

# Stop the session
agent-tui kill
```

## Commands

### Session Management

```bash
agent-tui run [COMMAND] [ARGS]     # Run TUI app (--cols, --rows, --cwd)
agent-tui sessions list            # List active sessions
agent-tui sessions attach [ID]     # Attach to session
agent-tui kill                     # Kill current session
agent-tui restart                  # Restart session
```

### Screenshots & Elements

```bash
agent-tui screenshot               # Capture screenshot
agent-tui screenshot -e            # Include detected elements
agent-tui screenshot -a            # Accessibility tree format
agent-tui find --role button       # Find elements by role
agent-tui find --text "Submit"     # Find by text content
agent-tui count --role input       # Count matching elements
```

### Actions

```bash
agent-tui action @e1 click         # Click element
agent-tui action @inp1 fill "text" # Fill input field
agent-tui action @chk1 toggle      # Toggle checkbox
agent-tui action @sel1 select "A"  # Select option
agent-tui action @e1 focus         # Focus element
agent-tui action @e1 scroll up     # Scroll element
agent-tui scroll-into-view @e5     # Scroll element into view
```

### Input

```bash
agent-tui press Enter              # Press key
agent-tui press Ctrl+C             # Key combination
agent-tui press ArrowDown ArrowDown Enter  # Multiple keys
agent-tui type "hello"             # Type text character by character
agent-tui input Tab                # Unified input (keys or text)
```

### Wait Conditions

```bash
agent-tui wait "Ready"             # Wait for text
agent-tui wait -e @btn1            # Wait for element
agent-tui wait --focused @inp1     # Wait for focus
agent-tui wait --stable            # Wait for screen stability
agent-tui wait "Loading" -g        # Wait for text to disappear (gone)
agent-tui wait "Done" -t 5000      # Custom timeout (ms)
agent-tui wait "Error" --assert    # Assert condition (exit 1 if not met)
```

### Daemon & Live Preview

```bash
agent-tui daemon start             # Start daemon
agent-tui daemon start --foreground # Run in foreground
agent-tui daemon stop              # Stop daemon
agent-tui daemon status            # Check daemon health

agent-tui live start               # Start HTTP/WebSocket API
agent-tui live status              # Show API status
agent-tui live stop                # Stop API
```

### Utilities

```bash
agent-tui health                   # CLI health check
agent-tui version                  # Version info
agent-tui env                      # Environment diagnostics
agent-tui resize --cols 120 --rows 40  # Resize terminal
agent-tui completions bash         # Generate shell completions
```

### Global Options

| Option | Description |
|--------|-------------|
| `-s, --session ID` | Specify session ID |
| `-f, --format FMT` | Output format: `text` or `json` |
| `--json` | Shorthand for `--format json` |
| `--no-color` | Disable colored output |
| `-v, --verbose` | Verbose output |

## Element Selectors

agent-tui supports multiple ways to reference UI elements:

```bash
# By element ref (from screenshot -e output)
@e1, @btn1, @inp1

# By exact text match
@"Yes, proceed"

# By partial text match (prefix with colon)
:Submit

# Combined with role filtering
agent-tui find --role button --text "OK"
```

## Output Formats

### Text (default)

Human-readable output for interactive use:

```
Screenshot captured (80x24)

Detected Elements:
  @btn1  button   "Submit"     row=10 col=5
  @inp1  input    ""           row=8  col=5  focused
  @chk1  checkbox "Remember"   row=12 col=5  checked
```

### JSON

Machine-readable output for automation:

```bash
agent-tui screenshot -e --json
```

```json
{
  "screen": { "rows": 24, "cols": 80 },
  "elements": [
    { "ref": "@btn1", "role": "button", "text": "Submit", "row": 10, "col": 5 },
    { "ref": "@inp1", "role": "input", "text": "", "row": 8, "col": 5, "focused": true }
  ]
}
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `AGENT_TUI_SOCKET` | IPC socket path | `~/.agent-tui/daemon.sock` |
| `AGENT_TUI_API_LISTEN` | API bind address | `127.0.0.1:0` |
| `AGENT_TUI_API_ALLOW_REMOTE` | Allow remote API connections | `false` |
| `AGENT_TUI_API_TOKEN` | API authentication token | Auto-generated |
| `AGENT_TUI_API_STATE` | API state file path | `~/.agent-tui/api.json` |
| `AGENT_TUI_UI_URL` | External UI URL | - |
| `AGENT_TUI_DETACH_KEYS` | Attach detach key sequence | `Ctrl+]` |
| `NO_COLOR` | Disable colored output | - |

## Architecture

```
agent-tui/
├── cli/                    # Rust workspace
│   └── crates/agent-tui/   # Main binary
│       ├── app/            # Application layer (CLI, handlers)
│       ├── adapters/       # Infrastructure adapters (IPC, RPC)
│       ├── domain/         # Domain models (elements, screen, VOM)
│       ├── usecases/       # Business logic (snapshot, input, wait)
│       └── infra/          # Infrastructure (daemon, terminal)
├── web/                    # Bun-based web UI
├── scripts/                # Automation scripts
└── docs/                   # Documentation
```

The tool follows Clean Architecture principles:
- **Domain** - Core models (Element, Screen, Style) and VOM detection
- **Use Cases** - Business logic (screenshot, input, wait conditions)
- **Adapters** - External interfaces (CLI, RPC, HTTP API)
- **Infrastructure** - Terminal emulation, daemon runtime

## Development

### Prerequisites

- Rust 1.70+
- Bun (for web UI)
- just (task runner)

### Build Commands

```bash
just build           # Build Rust crate
just build-release   # Optimized release build
just web-build       # Build web UI
just test            # Run tests
just ready           # Full CI checks (fmt, clippy, tests)
just lint            # Run Clippy
just format          # Format code
just doc             # Build and open docs
```

### Running Locally

```bash
just dev             # Run daemon in dev mode
just health          # Check CLI health
```

## License

MIT

## Links

- [Repository](https://github.com/pproenca/agent-tui)
- [Issues](https://github.com/pproenca/agent-tui/issues)
