# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

agent-tui enables AI agents to programmatically interact with TUI (Terminal User Interface) applications. It's a pure Rust system with a CLI that embeds a native daemon.

**Key capability**: Element detection via the Visual Object Model (VOM) - treats the terminal as a grid of styled cells and identifies UI elements (buttons, inputs, tabs) using Connected-Component Labeling.

## Build Commands

```bash
cd cli
cargo build                    # Debug build
cargo build --release          # Release build
cargo run -- <args>            # Build and run
cargo fmt                      # Format code
cargo lint                     # Lint (alias for clippy with -D warnings)
cargo test                     # Run all tests
cargo test test_name           # Run specific test
cargo test -- --nocapture      # Tests with output
```

Or use `just` recipes from the project root:
```bash
just              # Show available recipes
just ready        # Run all checks (format-check, lint, test)
just lint         # Run clippy
just test         # Run tests
just watch        # Rebuild on changes (requires cargo-watch)
```

## Architecture

### Data Flow
```
CLI (clap) → JSON-RPC request → Unix socket → Daemon
                                                 ↓
                                         Session Manager
                                                 ↓
                                         PTY + vt100 emulation
                                                 ↓
                                         VOM Analysis → Elements
                                                 ↓
                                         Target TUI app
```

### Key Modules (`cli/src/`)

| Module | Purpose |
|--------|---------|
| `main.rs` | Entry point, command dispatch |
| `commands.rs` | Clap CLI definitions |
| `handlers.rs` | Command execution logic |
| `session.rs` | Session lifecycle and state management |
| `client.rs` | IPC client implementation |
| `pty.rs` | PTY creation and I/O handling |
| `terminal.rs` | Terminal emulation, screen buffer |
| `attach.rs` | Interactive terminal attach mode |
| `wait.rs` | Wait conditions and polling logic |

### VOM (Visual Object Model) - `cli/src/vom/`

The VOM is the core element detection system. Pipeline:

1. **Segmentation** (`segmentation.rs`): Raster scan → `Vec<Cluster>` (style-homogeneous runs)
2. **Classification** (`classifier.rs`): Geometric & attribute heuristics → `Vec<Component>` with roles
3. **Interaction** (`interaction.rs`): Mouse injection via ANSI CSI sequences
4. **Feedback** (`feedback.rs`): Layout signatures for change detection

Supported roles: `Button`, `Tab`, `Input`, `StaticText`, `Panel`, `Checkbox`, `MenuItem`

### Daemon (`cli/src/daemon/`)

| File | Purpose |
|------|---------|
| `server.rs` | JSON-RPC server, request routing |
| `rpc_types.rs` | Request/response type definitions |
| `error_messages.rs` | User-facing error formatting |

### IPC Protocol

- JSON-RPC 2.0 over Unix socket: `$XDG_RUNTIME_DIR/agent-tui.sock` or `/tmp/agent-tui.sock`
- TCP fallback on port 19847 via `AGENT_TUI_TRANSPORT=tcp`
- Methods: `spawn`, `snapshot`, `click`, `fill`, `keystroke`, `type`, `wait`, `scroll`, `kill`, `sessions`, `health`

## Testing

### Unit Tests
Run with `cargo test`. Tests are co-located in source files.

### Integration Tests (`cli/tests/`)

| File | Purpose |
|------|---------|
| `cli_spawn_tests.rs` | CLI invocation tests with mock daemon |
| `e2e_daemon_tests.rs` | Full daemon integration tests |
| `common/mock_daemon.rs` | Mock JSON-RPC server |
| `common/test_harness.rs` | Sync wrapper for async mock |

### CLI Snapshot Tests (`cli/tests/cmd/`)

Uses `trycmd` for file-based CLI snapshot testing. Each `.md` file contains expected input/output:

```
00-help.md       # Help text verification
01-health.md     # Health command
02-spawn.md      # Spawn command
...
99-errors.md     # Error message verification
```

Run: `cargo test --test cmd_e2e_tests`

### E2E Tests (`e2e/`)

Shell scripts for full system testing:
- `test-claude-code.sh` - Test agent-tui controlling Claude Code
- `demo-claude-code-self-test.sh` - Demo script

## Code Style

### Rust
- Format: `cargo fmt`
- Lint: `cargo lint` (warnings as errors)
- No unnecessary comments - code should be self-explanatory
- Comment only critical decisions explaining "why" not "what"

## Critical Development Rules

### NEVER Ignore Lint Rules or Tests
- **All code must pass `cargo lint`** - no exceptions
- **All tests must pass** - do not skip or disable tests
- If a feature is partially implemented:
  - Either fully implement it so tests pass
  - Or do not add tests/exports for unimplemented parts
  - Never use `#[allow(unused)]` to hide incomplete work
- Lint suppressions like `#[allow(dead_code)]` are only acceptable for intentionally public APIs

### Dead Code Policy
- When encountering dead code, **do not immediately delete it**
- First investigate: Is this an incomplete feature that should be finished?
- Check if similar code nearby IS used - dead code may be part of an incomplete pattern
- Only delete dead code after confirming it's truly obsolete

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `AGENT_TUI_TRANSPORT` | `unix` or `tcp` | unix |
| `AGENT_TUI_TCP_PORT` | TCP port | 19847 |
| `XDG_RUNTIME_DIR` | Socket directory | /tmp |

## Getting Started

New to agent-tui? Run the interactive guided tour:

```
/onboard
```

This walks through architecture, live demos, element detection, and controlling Claude Code programmatically.
