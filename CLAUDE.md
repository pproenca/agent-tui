# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

agent-tui enables AI agents to programmatically interact with TUI (Terminal User Interface) applications. It's a pure Rust system with a CLI that embeds a native daemon.

**Key capability**: Element detection via the Visual Object Model (VOM) - treats the terminal as a grid of styled cells and identifies UI elements (buttons, inputs, tabs) using Connected-Component Labeling.

## Build Commands

```bash
cargo build --workspace              # Debug build all crates
cargo build --workspace --release    # Release build
cargo run -p agent-tui -- <args>     # Run CLI
cargo fmt                            # Format code
cargo clippy --workspace -- -D warnings  # Lint all crates
cargo test --workspace               # Run all tests
cargo test test_name                 # Run specific test
cargo test -- --nocapture            # Tests with output
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

### Workspace Crates

| Crate | Purpose |
|-------|---------|
| `agent-tui` | CLI binary and command handlers |
| `agent-tui-core` | VOM, session management, core types |
| `agent-tui-terminal` | Terminal emulation, PTY handling |
| `agent-tui-daemon` | JSON-RPC server, daemon logic |
| `agent-tui-ipc` | Client/server IPC protocol |
| `agent-tui-common` | Shared utilities and types |

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

### Key Modules

| Module | Purpose |
|--------|---------|
| `crates/agent-tui/src/main.rs` | Entry point, command dispatch |
| `crates/agent-tui/src/commands.rs` | Clap CLI definitions |
| `crates/agent-tui/src/handlers.rs` | Command execution logic |
| `crates/agent-tui-core/src/session.rs` | Session lifecycle and state management |
| `crates/agent-tui-ipc/src/client.rs` | IPC client implementation |
| `crates/agent-tui-terminal/src/pty.rs` | PTY creation and I/O handling |
| `crates/agent-tui-terminal/src/terminal.rs` | Terminal emulation, screen buffer |

### VOM (Visual Object Model) - `crates/agent-tui-core/src/vom/`

The VOM is the core element detection system. Pipeline:

1. **Segmentation** (`segmentation.rs`): Raster scan → `Vec<Cluster>` (style-homogeneous runs)
2. **Classification** (`classifier.rs`): Geometric & attribute heuristics → `Vec<Component>` with roles

Supported roles: `Button`, `Tab`, `Input`, `StaticText`, `Panel`, `Checkbox`, `MenuItem`

### Daemon (`crates/agent-tui-daemon/src/`)

| File | Purpose |
|------|---------|
| `server.rs` | JSON-RPC server, request routing |
| `session.rs` | Session management, state |
| `wait.rs` | Wait conditions, stable tracking |

#### Clean Architecture Layers

The daemon follows Clean Architecture with these layers (dependencies point inward):

```
┌─────────────────────────────────────────────────────────────┐
│                    Infrastructure Layer                      │
│  server.rs, session.rs, repository.rs, pty_session.rs       │
│  (External interfaces: JSON-RPC, PTY, file I/O)             │
├─────────────────────────────────────────────────────────────┤
│                  Interface Adapters Layer                    │
│  handlers/*.rs, adapters/rpc.rs                             │
│  (Request/response conversion, orchestration)               │
├─────────────────────────────────────────────────────────────┤
│                      Use Cases Layer                         │
│  usecases/*.rs (44 use cases in 6 files)                    │
│  (Business logic, orchestrates domain operations)           │
├─────────────────────────────────────────────────────────────┤
│                       Domain Layer                           │
│  domain/types.rs, domain/session_types.rs                   │
│  (Core types: SessionId, SessionInfo, input/output DTOs)    │
└─────────────────────────────────────────────────────────────┘
```

**Dependency Rule**: Inner layers NEVER import from outer layers. Domain types must not depend on infrastructure.

#### Intentional Partial Boundaries

These root-level modules are acceptable as partial boundaries:

| Module | Purpose | Rationale |
|--------|---------|-----------|
| `wait.rs` | Wait condition algorithms | Uses `SessionOps` trait for dependency inversion |
| `ansi_keys.rs` | ANSI key sequence mappings | Static data, no I/O |
| `select_helpers.rs` | Element selection algorithms | Uses `SessionOps` trait for dependency inversion |
| `adapters/domain_adapters.rs` | Core -> Domain translation | Pure functions, no I/O |
| `adapters/snapshot_adapters.rs` | Domain -> DTO translation | Pure functions, no I/O |

These modules are used by use cases. The `wait.rs` and `select_helpers.rs` modules use the `SessionOps` trait (defined in `repository.rs`) for dependency inversion, allowing them to work with any session-like type without depending on the concrete `Session` implementation.

### IPC (`crates/agent-tui-ipc/src/`)

| File | Purpose |
|------|---------|
| `client.rs` | DaemonClient for CLI |
| `types.rs` | Request/response type definitions |
| `error_messages.rs` | User-facing error formatting |

### IPC Protocol

- JSON-RPC 2.0 over Unix socket: `$XDG_RUNTIME_DIR/agent-tui.sock` or `/tmp/agent-tui.sock`
- Methods: `spawn`, `snapshot`, `click`, `fill`, `keystroke`, `type`, `wait`, `scroll`, `kill`, `sessions`, `health`

## Testing

### Unit Tests
Run with `cargo test`. Tests are co-located in source files.

### Integration Tests (`crates/agent-tui/tests/`)

| File | Purpose |
|------|---------|
| `e2e_daemon_tests.rs` | Mock daemon integration tests |
| `e2e_workflow_tests.rs` | Real daemon E2E workflow tests |
| `common/mock_daemon.rs` | Mock JSON-RPC server |
| `common/test_harness.rs` | Sync wrapper for async mock |
| `common/real_test_harness.rs` | Real daemon test harness |

Each E2E workflow test spawns an isolated daemon instance on a unique socket, allowing parallel execution.

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
| `AGENT_TUI_SOCKET` | Custom socket path | (uses XDG_RUNTIME_DIR) |
| `XDG_RUNTIME_DIR` | Socket directory | /tmp |
| `AGENT_TUI_MAX_CONNECTIONS` | Max concurrent connections | 64 |
| `AGENT_TUI_LOCK_TIMEOUT` | Session lock timeout (seconds) | 5 |
| `AGENT_TUI_IDLE_TIMEOUT` | Idle connection timeout (seconds) | 300 |
| `AGENT_TUI_MAX_REQUEST` | Max request size (bytes) | 1048576 (1MB) |

## Feature Development Pipeline

For complex features, use the structured pipeline:

```
/brainstorm → /spec → /plan-tdd → Execute
```

### Pipeline Commands

| Command | Input | Output | Purpose |
|---------|-------|--------|---------|
| `/brainstorm <name>` | Ideas | `brainstorm.md` | Capture raw ideas, explore problem space |
| `/spec <name>` | `brainstorm.md` | `spec.md` | Formalize decisions, define interfaces |
| `/plan-tdd <name>` | `spec.md` | TaskList + `plan-summary.md` | Create executable TDD tasks |

### Artifact Location

All artifacts live in `.claude/specs/<feature-name>/`:
```
.claude/specs/plugin-system/
├── brainstorm.md     ← Raw ideas, open questions
├── spec.md           ← Formal spec with decisions
└── plan-summary.md   ← Task plan overview
```

### When to Use Each Stage

**Full Pipeline** (`/brainstorm` → `/spec` → `/plan-tdd`):
- New features with unclear requirements
- Architectural changes
- Features needing stakeholder input

**Skip to Spec** (`/spec` → `/plan-tdd`):
- Requirements already clear
- Enhancing existing feature

**Direct to Tasks** (`/plan-tdd` only):
- Well-defined small features
- Bug fixes with known solution
- Refactoring with clear target

### TDD Task Pattern
Every feature follows Red-Green-Refactor with enforced dependencies:
1. `[RED]` Write failing test → blocks implementation
2. `[GREEN]` Implement to pass → blocked by test
3. `[REFACTOR]` Clean up → blocked by implementation

**Dependency Setup (TaskUpdate):**
```
TaskCreate: "[RED] Write test for X" → returns id "1"
TaskCreate: "[GREEN] Implement X" → returns id "2"
TaskUpdate(taskId: "2", addBlockedBy: ["1"])  # enforces test-first
```

### Carpaccio Slicing
Tasks are atomic and demonstrable:
- Each completable in a single focused action
- One clear outcome per task
- Always includes verification command
- If you say "and", split it

## Getting Started

New to agent-tui? Check RALPH loop status:

```
/ralph-status
```

Use `/ralph-init` to initialize a new RALPH loop for task tracking.
