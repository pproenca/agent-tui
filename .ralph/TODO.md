# Migration TODO Checklist

## Phase 0: Workspace Setup
- [ ] Create root `Cargo.toml` with workspace manifest
- [ ] Create `crates/` directory structure
- [ ] Move `cli/rustfmt.toml` to root
- [ ] Move `cli/clippy.toml` to root

## Phase 1: agent-tui-common (Leaf Crate)

| Status | File | Notes |
|--------|------|-------|
| [ ] | `color.rs` | OnceLock NO_COLOR, Colors struct |
| [ ] | `json_ext.rs` | ValueExt trait |
| [ ] | `sync_utils.rs` → `sync.rs` | Lock helper functions |
| [ ] | Create `lib.rs` | Export all public API |
| [ ] | Create `Cargo.toml` | Minimal deps: serde_json |
| [ ] | Verify: `cargo build -p agent-tui-common` | |

## Phase 2: agent-tui-terminal

| Status | File | Notes |
|--------|------|-------|
| [ ] | `pty.rs` | PtyHandle, PtyError |
| [ ] | `terminal.rs` | VirtualTerminal, ScreenBuffer, Cell, CellStyle |
| [ ] | Extract `keys.rs` | key_to_escape_sequence if in pty.rs |
| [ ] | Create `lib.rs` | Export terminal types |
| [ ] | Create `Cargo.toml` | Deps: vt100, portable-pty, agent-tui-common |
| [ ] | Verify: `cargo build -p agent-tui-terminal` | |

## Phase 3: agent-tui-core (VOM)

| Status | File | Notes |
|--------|------|-------|
| [ ] | `vom/mod.rs` | VOM types, analyze() |
| [ ] | `vom/segmentation.rs` | Cluster detection |
| [ ] | `vom/classifier.rs` | Role classification |
| [ ] | Extract `element.rs` | Element, ElementType, Position FROM session.rs |
| [ ] | Create `lib.rs` | Export VOM + Element API |
| [ ] | Create `Cargo.toml` | Deps: agent-tui-terminal, agent-tui-common |
| [ ] | Verify: `cargo build -p agent-tui-core` | |

## Phase 4: agent-tui-ipc

| Status | File | Notes |
|--------|------|-------|
| [ ] | `daemon/rpc_types.rs` → `types.rs` | Request, Response, RpcError |
| [ ] | `client.rs` | DaemonClient |
| [ ] | Extract `socket.rs` | socket_path() from server.rs |
| [ ] | Create `error.rs` | ClientError enum |
| [ ] | Create `lib.rs` | Export IPC types |
| [ ] | Create `Cargo.toml` | Deps: serde, serde_json, tokio, agent-tui-common |
| [ ] | Verify: `cargo build -p agent-tui-ipc` | |

## Phase 5: agent-tui-daemon

| Status | File | Notes |
|--------|------|-------|
| [ ] | `session.rs` (main) | Session, SessionManager, SessionError |
| [ ] | `wait.rs` | WaitCondition, StableTracker |
| [ ] | `daemon/server.rs` | DaemonServer, start_daemon |
| [ ] | `daemon/error_messages.rs` → `handlers/` | ai_friendly_error |
| [ ] | `daemon/lock_helpers.rs` → `handlers/` | acquire_session_lock |
| [ ] | `daemon/select_helpers.rs` → `handlers/` | navigate_to_option |
| [ ] | `daemon/ansi_keys.rs` → `handlers/` | ANSI sequences |
| [ ] | Create `handlers/mod.rs` | Re-export handlers |
| [ ] | Create `lib.rs` | Export daemon API |
| [ ] | Create `Cargo.toml` | Deps: all workspace crates |
| [ ] | Verify: `cargo build -p agent-tui-daemon` | |

## Phase 6: agent-tui (Binary)

| Status | File | Notes |
|--------|------|-------|
| [ ] | `main.rs` | Entry point |
| [ ] | `commands.rs` | Clap CLI definitions |
| [ ] | `handlers.rs` | Command handlers |
| [ ] | `attach.rs` | Interactive attach mode |
| [ ] | Create `lib.rs` (optional) | Re-exports for library consumers |
| [ ] | Create `Cargo.toml` | Deps: all workspace crates, clap |
| [ ] | Verify: `cargo build -p agent-tui` | |

## Phase 7: Cleanup & Finalization

| Status | Task |
|--------|------|
| [ ] | Move `cli/tests/` → `tests/` |
| [ ] | Update test imports |
| [ ] | Remove old `cli/` directory |
| [ ] | Update `justfile` paths |
| [ ] | Update `package.json` paths |
| [ ] | Update `CLAUDE.md` with new structure |
| [ ] | Run `just ready` - full verification |
| [ ] | Run E2E tests |

## Verification Commands

```bash
# Per-crate verification
cargo build -p agent-tui-common
cargo build -p agent-tui-terminal
cargo build -p agent-tui-core
cargo build -p agent-tui-ipc
cargo build -p agent-tui-daemon
cargo build -p agent-tui

# Full workspace
cargo build --workspace
cargo test --workspace
cargo clippy --workspace
cargo fmt --check

# E2E
cargo run -p agent-tui -- --help
cargo run -p agent-tui -- daemon &
cargo run -p agent-tui -- spawn bash
cargo run -p agent-tui -- snapshot -i
```

## Notes

- Dependencies flow upward: common → terminal/core → ipc/daemon → binary
- Never have circular dependencies between crates
- Each crate has its own `Result<T>` type alias with its error type
- Import style: std → external → workspace → crate (one per line)
