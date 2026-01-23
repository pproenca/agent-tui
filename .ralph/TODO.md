# Migration TODO Checklist

## Phase 0: Workspace Setup ✅
- [x] Create root `Cargo.toml` with workspace manifest
- [x] Create `crates/` directory structure
- [x] Move `cli/rustfmt.toml` to root
- [x] Move `cli/clippy.toml` to root

## Phase 1: agent-tui-common (Leaf Crate) ✅

| Status | File | Notes |
|--------|------|-------|
| [x] | `color.rs` | OnceLock NO_COLOR, Colors struct |
| [x] | `json_ext.rs` | ValueExt trait |
| [x] | `sync_utils.rs` → `sync.rs` | Lock helper functions |
| [x] | Create `lib.rs` | Export all public API |
| [x] | Create `Cargo.toml` | Minimal deps: serde_json |
| [x] | Verify: `cargo build -p agent-tui-common` | |

## Phase 2: agent-tui-terminal ✅

| Status | File | Notes |
|--------|------|-------|
| [x] | `pty.rs` | PtyHandle, PtyError |
| [x] | `terminal.rs` | VirtualTerminal, ScreenBuffer, Cell, CellStyle |
| [x] | Extract `keys.rs` | key_to_escape_sequence if in pty.rs |
| [x] | Create `lib.rs` | Export terminal types |
| [x] | Create `Cargo.toml` | Deps: vt100, portable-pty, agent-tui-common |
| [x] | Verify: `cargo build -p agent-tui-terminal` | |

## Phase 3: agent-tui-core (VOM) ✅

| Status | File | Notes |
|--------|------|-------|
| [x] | `vom/mod.rs` | VOM types, analyze() |
| [x] | `vom/segmentation.rs` | Cluster detection |
| [x] | `vom/classifier.rs` | Role classification |
| [x] | Extract `element.rs` | Element, ElementType, Position FROM session.rs |
| [x] | Create `lib.rs` | Export VOM + Element API |
| [x] | Create `Cargo.toml` | Deps: agent-tui-terminal, agent-tui-common |
| [x] | Verify: `cargo build -p agent-tui-core` | |

## Phase 4: agent-tui-ipc ✅

| Status | File | Notes |
|--------|------|-------|
| [x] | `daemon/rpc_types.rs` → `types.rs` | Request, Response, RpcError |
| [x] | `client.rs` | DaemonClient |
| [x] | Extract `socket.rs` | socket_path() from server.rs |
| [x] | Create `error.rs` | ClientError enum |
| [x] | Create `lib.rs` | Export IPC types |
| [x] | Create `Cargo.toml` | Deps: serde, serde_json, tokio, agent-tui-common |
| [x] | Verify: `cargo build -p agent-tui-ipc` | |

## Phase 5: agent-tui-daemon ✅

| Status | File | Notes |
|--------|------|-------|
| [x] | `session.rs` (main) | Session, SessionManager, SessionError |
| [x] | `wait.rs` | WaitCondition, StableTracker |
| [x] | `daemon/server.rs` | DaemonServer, start_daemon |
| [x] | `daemon/error_messages.rs` | ai_friendly_error |
| [x] | `daemon/lock_helpers.rs` | acquire_session_lock |
| [x] | `daemon/select_helpers.rs` | navigate_to_option |
| [x] | `daemon/ansi_keys.rs` | ANSI sequences |
| [x] | Create `lib.rs` | Export daemon API |
| [x] | Create `Cargo.toml` | Deps: all workspace crates |
| [x] | Verify: `cargo build -p agent-tui-daemon` | |

## Phase 6: agent-tui (Binary) ✅

| Status | File | Notes |
|--------|------|-------|
| [x] | `main.rs` | Entry point with full command dispatch |
| [x] | `attach.rs` | Interactive attach mode |
| [x] | Create `lib.rs` | Re-exports for library consumers |
| [x] | Create `Cargo.toml` | Deps: all workspace crates, clap |
| [x] | `commands.rs` | Full Clap CLI definitions (1752 lines) |
| [x] | `handlers.rs` | Full command handlers (1547 lines) |
| [x] | Verify: `cargo build -p agent-tui` | ✓ Build successful |

## Phase 7: Cleanup & Finalization ✅

| Status | Task |
|--------|------|
| [x] | Move `cli/tests/` → `crates/agent-tui/tests/` |
| [x] | Update test imports (added tokio, uuid dev-deps) |
| [x] | Remove old `cli/` directory |
| [x] | Update `justfile` paths (already using workspace commands) |
| [x] | Update `package.json` paths |
| [x] | Update GitHub workflows (ci.yml, release.yml, release-manual.yml) |
| [x] | Update scripts (sync-version.js, release.sh) |
| [x] | Update README.md project structure |
| [x] | Update `CLAUDE.md` with new structure |
| [x] | Run `cargo fmt && cargo clippy && cargo test` - full verification |
| [x] | Run E2E tests (39 daemon tests + 15 workflow tests pass) |

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
