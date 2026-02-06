# agent-tui

CLI tool for AI agents to interact with TUI (Terminal User Interface) applications.

**agent-tui** enables AI agents to programmatically drive terminal applications by capturing screenshots and sending input—making TUI automation accessible to LLM-powered agents.

## Features

- **Virtual Terminal Emulation** - Run TUI apps in isolated PTY sessions with full terminal emulation
- **Keyboard & Text Input** - Press keys, type text, or send unified input
- **Wait Conditions** - Wait for text or screen stability
- **Output Formats** - Human-readable text or JSON for automation pipelines
- **Live Preview WebSocket** - JSON-RPC over WebSocket for real-time UI monitoring
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

# Take a screenshot
agent-tui screenshot

# Send keyboard input
agent-tui press Enter
agent-tui type "hello world"

# Wait for conditions
agent-tui wait "Loading complete"

# Stop the session
agent-tui kill
```

### Session Recording (VHS)

```bash
# Record active session (background)
agent-tui sessions record

# Record active session to a directory
agent-tui sessions record -o docs/recordings

# Stop recording for a specific session
agent-tui --session <id> sessions record stop
```

## CLI Reference

For the full CLI reference (auto-generated from clap), see `docs/cli/agent-tui.md`.

You can also run:
- `agent-tui --help`
- `agent-tui <command> --help`

## Output Formats

### Text (default)

Human-readable output for interactive use:

```
Screenshot:
<screen contents here>
```

### JSON

Machine-readable output for automation:

```bash
agent-tui screenshot --json
```

```json
{
  "session_id": "abc123",
  "screenshot": "..."
}
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `AGENT_TUI_SOCKET` | IPC socket path | `~/.agent-tui/daemon.sock` |
| `AGENT_TUI_TRANSPORT` | CLI transport (`unix` or `ws`) | `unix` |
| `AGENT_TUI_WS_ADDR` | Remote WS-RPC URL when transport is `ws` | - |
| `AGENT_TUI_WS_LISTEN` | Daemon WS bind address | `127.0.0.1:0` |
| `AGENT_TUI_WS_ALLOW_REMOTE` | Allow non-loopback WS bind | `false` |
| `AGENT_TUI_WS_STATE` | WS state file path | `~/.agent-tui/api.json` |
| `AGENT_TUI_WS_DISABLED` | Disable daemon WS server | `false` |
| `AGENT_TUI_WS_MAX_CONNECTIONS` | Max WS connections | `32` |
| `AGENT_TUI_WS_QUEUE` | WS outbound queue size | `128` |
| `AGENT_TUI_API_LISTEN` / `AGENT_TUI_API_ALLOW_REMOTE` / `AGENT_TUI_API_STATE` | Deprecated aliases for WS settings | - |
| `AGENT_TUI_API_TOKEN` | Deprecated and ignored | - |
| `AGENT_TUI_SESSION_STORE` | Session metadata log path | `~/.agent-tui/sessions.jsonl` |
| `AGENT_TUI_RECORD_STATE` | Recording state file path | `~/.agent-tui/recordings.json` |
| `AGENT_TUI_RECORDINGS_DIR` | Default recordings output directory | current working directory |
| `AGENT_TUI_UI_URL` | External UI URL | - |
| `AGENT_TUI_DETACH_KEYS` | Attach detach key sequence | `Ctrl+]` |
| `AGENT_TUI_LOG` | Log file path (optional) | - |
| `AGENT_TUI_LOG_FORMAT` | Log format (`text` or `json`) | `text` |
| `AGENT_TUI_LOG_STREAM` | Log output stream (`stderr` or `stdout`) | `stderr` |
| `PORT` | Fallback port for API listen | - |
| `NO_COLOR` | Disable colored output | - |

## Architecture

```
agent-tui/
├── cli/                    # Rust workspace
│   └── crates/agent-tui/   # Main binary
│       ├── app/            # Application layer (CLI, handlers)
│       ├── adapters/       # Infrastructure adapters (IPC, RPC)
│       ├── domain/         # Domain models (screen, snapshot, style)
│       ├── usecases/       # Business logic (snapshot, input, wait)
│       └── infra/          # Infrastructure (daemon, terminal)
├── web/                    # Bun-based web UI
├── scripts/                # Automation scripts
└── docs/                   # Documentation
```

The tool follows Clean Architecture principles:
- **Domain** - Core models (Screen, Snapshot, Style)
- **Use Cases** - Business logic (screenshot, input, wait conditions)
- **Adapters** - External interfaces (CLI, RPC, WebSocket)
- **Infrastructure** - Terminal emulation, daemon runtime

See `docs/ops/process-model.md` for process types and deployment guidance.

## Development

### Prerequisites

- Rust stable (1.85+)
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
```

## License

MIT

## Links

- [Repository](https://github.com/pproenca/agent-tui)
- [Issues](https://github.com/pproenca/agent-tui/issues)
