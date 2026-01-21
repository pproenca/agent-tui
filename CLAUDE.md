# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

agent-tui enables AI agents to programmatically interact with TUI (Terminal User Interface) applications. It's a pure Rust system:

- **CLI + Daemon (Rust)** in `/cli` - Fast command-line client (~30ms startup) with embedded native daemon
- **Communication** - JSON-RPC 2.0 over Unix sockets (default) or TCP

## Build Commands

```bash
cd cli
cargo build                    # Debug build
cargo build --release          # Release build
cargo run -- <args>            # Build and run (dev workflow)
cargo fmt                      # Format code
cargo clippy --all-targets --all-features -- -D warnings  # Lint
cargo test                     # Run tests
cargo test test_name           # Run specific test
cargo test -- --nocapture      # Tests with output
```

## Architecture

### Data Flow
```
CLI parses args (clap) → JSON-RPC request → Unix socket → Native Daemon
                                                              ↓
                                                    Session Manager
                                                              ↓
                                                    PTY + Virtual Terminal
                                                              ↓
                                                    Target TUI app
```

### Key Modules (`/cli/src/`)
- `main.rs` - Entry point, command dispatch
- `commands.rs` - Clap CLI definitions
- `connection.rs` - IPC client with retry logic and circuit breaker
- `protocol.rs` - JSON-RPC types and method constants
- `daemon/` - Native Rust daemon implementation
  - `mod.rs` - Daemon entry point and IPC server
  - `session.rs` - Session lifecycle management
  - `pty.rs` - PTY creation and I/O
  - `terminal.rs` - Terminal emulation and screen buffer
  - `detection/` - Element detection (buttons, inputs, etc.)

### IPC Protocol
- JSON-RPC 2.0 over Unix socket at `$XDG_RUNTIME_DIR/agent-tui.sock` or `/tmp/agent-tui.sock`
- TCP fallback on port 19847 via `AGENT_TUI_TRANSPORT=tcp`
- Methods: `spawn`, `snapshot`, `click`, `fill`, `keystroke`, `type`, `wait`, `kill`, `sessions`, `health`

## Testing

### CLI Tests (`/cli/tests/`)
- Uses `assert_cmd` for CLI invocation testing
- `common/mock_daemon.rs` - Mock JSON-RPC server for E2E tests without real daemon
- `common/test_harness.rs` - Sync wrapper for async mock

## Code Style

### Rust
- Format: `cargo fmt`
- Lint: `cargo clippy` with `-D warnings` (warnings as errors)

### Comments
- Avoid unnecessary comments - code should be self-explanatory
- Only comment critical decisions that explain "why" not "what"
- Example: `// Use content-based stable refs to prevent ref drift when screen changes`

## Critical Development Rules

### NEVER Ignore Lint Rules or Tests
- **All code must pass `cargo clippy -- -D warnings`** - no exceptions
- **All tests must pass** - do not skip or disable tests
- If a feature is partially implemented:
  - Either fully implement it so tests pass
  - Or do not add tests/exports for unimplemented parts
  - Never use `#[allow(unused)]` to hide incomplete work
- If you add a public API, it must be used somewhere or have tests
- Lint suppressions like `#[allow(unused_imports)]` are only acceptable for intentionally public APIs that external consumers may use

### Dead Code Policy
- When encountering dead code (unused functions, constants, etc.), **do not immediately delete it**
- First investigate: Is this an incomplete feature that should be finished?
- Check if similar code nearby IS used - the dead code may be part of an incomplete pattern
- If the code represents unfinished work, complete the implementation rather than removing it
- Only delete dead code after confirming it's truly obsolete and not part of a planned feature

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

This command walks you through:
- The architecture and how it works
- Live demos of spawning apps and taking snapshots
- Element detection in real TUI applications
- The showstopper: controlling Claude Code programmatically
