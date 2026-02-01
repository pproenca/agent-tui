# Clean Architecture Refactor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete the Clean Architecture refactor by fixing broken module declarations in `lib.rs` and `daemon/mod.rs`, removing backward-compatibility stubs, and ensuring all dependencies flow inward per the target architecture.

**Architecture:** The crate uses a single-crate Clean Architecture pattern with layers: `domain/` (pure business rules) → `usecases/` (application logic) → `adapters/` (interface adapters) → `infra/` (infrastructure) → `app/` (CLI orchestration). The `daemon/` module contains server infrastructure that orchestrates usecases.

**Tech Stack:** Rust, Cargo, thiserror for errors, clap for CLI, serde for serialization.

---

## Current State Analysis

### What Exists
The codebase has been **partially refactored**:
- New CA directories exist: `adapters/`, `app/`, `common/`, `domain/`, `infra/`, `usecases/`
- Files have been moved to new locations
- Backward-compatibility stub files exist at root: `attach.rs`, `commands.rs`, `error.rs`, `handlers.rs`, `presenter.rs`
- Backward-compatibility stub directories exist: `core/`, `ipc/`, `terminal/`

### What's Broken
- `lib.rs` declares modules that don't exist (`core`, `ipc`, `terminal`) or declares stubs that re-export
- `daemon/mod.rs` declares `adapters`, `domain`, `usecases` as children (but they're at crate root)
- Cargo can't compile due to missing module files

### Target State (from `docs/architecture/clean_arch_target.md`)
```
src/
  main.rs               // composition root only
  lib.rs                // module declarations + minimal re-exports
  domain/               // pure business rules, no IO
  usecases/             // application logic + ports
  adapters/             // RPC/CLI presenters
  infra/                // OS/socket/PTY/filesystem
  app/                  // CLI application flow
```

---

## Phase 1: Fix lib.rs Module Declarations

### Task 1: Read current broken lib.rs and understand dependencies

**Files:**
- Read: `crates/agent-tui/src/lib.rs`

**Step 1: Examine current declarations**
The current `lib.rs` declares:
```rust
pub mod common;
pub mod domain;
pub mod usecases;
pub mod adapters;
pub mod infra;
pub mod app;

// These don't exist as proper modules:
pub mod core;        // stub → re-exports domain::core
pub mod daemon;
pub mod ipc;         // stub → re-exports infra::ipc
pub mod terminal;    // stub → re-exports infra::terminal

// These are stubs that re-export from app/:
pub mod attach;
pub mod commands;
pub mod error;
pub mod handlers;
pub mod presenter;   // re-exports adapters::presenter

// Re-exports:
pub use app::Application;
pub use common::Colors;
pub use daemon::Session;
pub use daemon::SessionError;
pub use daemon::SessionId;
pub use daemon::SessionManager;
pub use ipc::ClientError;        // broken path
pub use ipc::DaemonClient;       // broken path
pub use error::AttachError;
pub use handlers::HandlerResult;
```

**Step 2: Note what needs to change**
- Remove stub modules: `attach`, `commands`, `error`, `handlers`, `presenter`
- Remove compatibility modules: `core`, `ipc`, `terminal`
- Fix re-export paths

---

### Task 2: Remove backward-compatibility stub files

**Files:**
- Delete: `crates/agent-tui/src/attach.rs`
- Delete: `crates/agent-tui/src/commands.rs`
- Delete: `crates/agent-tui/src/error.rs`
- Delete: `crates/agent-tui/src/handlers.rs`
- Delete: `crates/agent-tui/src/presenter.rs`
- Delete: `crates/agent-tui/src/core/mod.rs` (and `core/` directory)
- Delete: `crates/agent-tui/src/ipc/mod.rs` (and `ipc/` directory)
- Delete: `crates/agent-tui/src/terminal/mod.rs` (and `terminal/` directory)

**Step 1: Delete stub files**
```bash
rm crates/agent-tui/src/attach.rs
rm crates/agent-tui/src/commands.rs
rm crates/agent-tui/src/error.rs
rm crates/agent-tui/src/handlers.rs
rm crates/agent-tui/src/presenter.rs
rm -r crates/agent-tui/src/core/
rm -r crates/agent-tui/src/ipc/
rm -r crates/agent-tui/src/terminal/
```

**Step 2: Verify deletion**
```bash
ls -la crates/agent-tui/src/*.rs
ls -d crates/agent-tui/src/*/
```

Expected: Only `lib.rs` and `main.rs` at root, plus `adapters/`, `app/`, `common/`, `daemon/`, `domain/`, `infra/`, `usecases/` directories.

---

### Task 3: Rewrite lib.rs with correct module structure

**Files:**
- Modify: `crates/agent-tui/src/lib.rs`

**Step 1: Write the new lib.rs**

```rust
#![deny(clippy::all)]

// Clean Architecture layers (dependencies flow inward)
pub mod common;      // shared utilities, no business logic
pub mod domain;      // pure business rules, no IO/frameworks
pub mod usecases;    // application logic, orchestrates domain
pub mod adapters;    // interface adapters (RPC/CLI presenters)
pub mod infra;       // infrastructure (IPC, PTY, filesystem)
pub mod app;         // CLI application flow (thin)
pub mod daemon;      // daemon server infrastructure

// Primary public API
pub use app::Application;

// Common utilities
pub use common::Colors;

// Domain types (via domain::core)

// Daemon types
pub use daemon::Session;
pub use daemon::SessionError;
pub use daemon::SessionId;
pub use daemon::SessionManager;

// IPC types (via infra::ipc)
pub use infra::ipc::ClientError;
pub use infra::ipc::DaemonClient;

// App types
pub use app::error::AttachError;
pub use app::handlers::HandlerResult;
```

**Step 2: Run cargo check**
```bash
cargo check -p agent-tui 2>&1 | head -50
```
Expected: May still have errors from daemon/mod.rs (to fix next)

---

## Phase 2: Fix daemon/mod.rs Module Declarations

### Task 4: Fix daemon/mod.rs broken module declarations

**Files:**
- Modify: `crates/agent-tui/src/daemon/mod.rs`

**Step 1: Read current daemon/mod.rs**
Current content declares:
```rust
pub mod adapters;    // doesn't exist in daemon/
pub mod domain;      // doesn't exist in daemon/
pub mod usecases;    // doesn't exist in daemon/
```

**Step 2: Remove broken declarations**
Remove lines 3, 7, 26 (or wherever `adapters`, `domain`, `usecases` are declared as submodules).

The daemon module should only declare its own submodules:
- `ansi_keys`
- `config`
- `error`
- `file_lock`
- `handlers/`
- `lock_helpers`
- `metrics`
- `pty_session`
- `repository`
- `router`
- `select_helpers`
- `server`
- `session`
- `signal_handler`
- `sleeper`
- `terminal_state`
- `test_support/`
- `transport/`
- `usecase_container`
- `wait`

**Step 3: Update the file**
```rust
#![deny(clippy::all)]

pub mod ansi_keys;
mod config;
mod error;
mod file_lock;
pub mod handlers;
mod lock_helpers;
mod metrics;
mod pty_session;
mod repository;
mod router;
mod select_helpers;
mod server;
mod session;
mod signal_handler;
mod sleeper;
mod terminal_state;
#[cfg(test)]
pub mod test_support;
pub mod transport;
mod usecase_container;
mod wait;

// Re-exports (keep existing ones that reference local modules only)
pub use config::DaemonConfig;
pub use error::DaemonError;
pub use error::DomainError;
pub use error::SessionError;
pub use lock_helpers::LOCK_TIMEOUT;
pub use lock_helpers::MAX_BACKOFF;
pub use lock_helpers::acquire_session_lock;
pub use metrics::DaemonMetrics;
pub use pty_session::PtySession;
pub use repository::SessionRepository;
pub use repository::SessionSnapshot;
pub use router::Router;
pub use select_helpers::navigate_to_option;
pub use select_helpers::parse_select_options;
pub use server::start_daemon;
pub use session::ErrorEntry;
pub use session::PersistedSession;
pub use session::RecordingFrame;
pub use session::RecordingStatus;
pub use session::Session;
pub use session::SessionId;
pub use session::SessionInfo;
pub use session::SessionManager;
pub use session::SessionPersistence;
pub use session::TraceEntry;
pub use sleeper::MockSleeper;
pub use sleeper::RealSleeper;
pub use sleeper::Sleeper;
pub use terminal_state::TerminalState;
pub use usecase_container::InputUseCases;
pub use usecase_container::SessionUseCases;
pub use usecase_container::UseCaseContainer;
pub use wait::StableTracker;
pub use wait::WaitCondition;
pub use wait::check_condition;

pub use transport::TransportConnection;
pub use transport::TransportError;
pub use transport::TransportListener;
pub use transport::unix_socket::UnixSocketConnection;
pub use transport::unix_socket::UnixSocketListener;

pub type Result<T> = std::result::Result<T, SessionError>;
```

**Step 4: Run cargo check**
```bash
cargo check -p agent-tui 2>&1 | head -100
```

---

## Phase 3: Fix app/mod.rs Import Paths

### Task 5: Update app/mod.rs to use correct import paths

**Files:**
- Modify: `crates/agent-tui/src/app/mod.rs`

**Step 1: Identify broken imports**
Current app/mod.rs has imports like:
```rust
use crate::commands::OutputFormat;
use crate::ipc::{ClientError, DaemonClient, UnixSocketClient, ensure_daemon};
use crate::attach::AttachError;
use crate::commands::{Cli, Commands, DaemonCommand, DebugCommand, RecordAction};
use crate::handlers::{self, HandlerContext};
```

These reference the old stub locations.

**Step 2: Update to correct paths**
```rust
use crate::app::commands::OutputFormat;
use crate::infra::ipc::{ClientError, DaemonClient, UnixSocketClient, ensure_daemon};
use crate::app::attach::AttachError;
use crate::app::commands::{Cli, Commands, DaemonCommand, DebugCommand, RecordAction};
use crate::app::handlers::{self, HandlerContext};
```

Or use local module paths:
```rust
use commands::OutputFormat;
use crate::infra::ipc::{ClientError, DaemonClient, UnixSocketClient, ensure_daemon};
use attach::AttachError;
use commands::{Cli, Commands, DaemonCommand, DebugCommand, RecordAction};
use handlers::{self, HandlerContext};
```

**Step 3: Run cargo check**
```bash
cargo check -p agent-tui 2>&1 | head -100
```

---

## Phase 4: Verify and Fix Remaining Compilation Errors

### Task 6: Fix any remaining import errors across codebase

**Files:**
- May need to modify: various files in `adapters/`, `daemon/handlers/`, etc.

**Step 1: Run full cargo check and capture errors**
```bash
cargo check -p agent-tui 2>&1
```

**Step 2: For each error, identify the broken import path**
Common patterns to fix:
- `crate::core::*` → `crate::domain::core::*`
- `crate::ipc::*` → `crate::infra::ipc::*`
- `crate::terminal::*` → `crate::infra::terminal::*`
- `crate::commands::*` → `crate::app::commands::*`
- `crate::handlers::*` → `crate::app::handlers::*`
- `crate::attach::*` → `crate::app::attach::*`
- `crate::error::*` → `crate::app::error::*`
- `crate::presenter::*` → `crate::adapters::presenter::*`

**Step 3: Apply fixes and re-check**
```bash
cargo check -p agent-tui 2>&1 | head -50
```

---

## Phase 5: Run Full Build and Tests

### Task 7: Verify clean build

**Files:**
- None (verification only)

**Step 1: Run cargo build**
```bash
cargo build -p agent-tui
```
Expected: Build succeeds

**Step 2: Run cargo clippy**
```bash
cargo clippy -p agent-tui -- -D warnings
```
Expected: No warnings

**Step 3: Run cargo test**
```bash
cargo test -p agent-tui
```
Expected: All tests pass

---

### Task 8: Commit the changes

**Step 1: Stage all changes**
```bash
git add crates/agent-tui/src/lib.rs
git add crates/agent-tui/src/daemon/mod.rs
git add crates/agent-tui/src/app/mod.rs
# Add any other modified files
git add -u  # stages all modifications
```

**Step 2: Verify nothing sensitive is staged**
```bash
git diff --cached --stat
```

**Step 3: Commit**
```bash
git commit -m "$(cat <<'EOF'
refactor: complete Clean Architecture module reorganization

- Remove backward-compatibility stub files (attach.rs, commands.rs, etc.)
- Remove compatibility redirect modules (core/, ipc/, terminal/)
- Fix lib.rs to declare correct module structure
- Fix daemon/mod.rs to remove non-existent submodule declarations
- Update import paths throughout crate to use new locations

Dependencies now flow inward:
  domain → usecases → adapters → infra → app
EOF
)"
```

---

## Summary

| Phase | Tasks | Description |
|-------|-------|-------------|
| 1 | 1-3 | Fix lib.rs module declarations, remove stubs |
| 2 | 4 | Fix daemon/mod.rs module declarations |
| 3 | 5 | Update app/mod.rs import paths |
| 4 | 6 | Fix remaining compilation errors |
| 5 | 7-8 | Verify build/tests, commit |

**Total tasks:** 8

**Key files modified:**
- `crates/agent-tui/src/lib.rs`
- `crates/agent-tui/src/daemon/mod.rs`
- `crates/agent-tui/src/app/mod.rs`
- Various files with broken imports

**Files deleted:**
- `crates/agent-tui/src/attach.rs`
- `crates/agent-tui/src/commands.rs`
- `crates/agent-tui/src/error.rs`
- `crates/agent-tui/src/handlers.rs`
- `crates/agent-tui/src/presenter.rs`
- `crates/agent-tui/src/core/` (directory)
- `crates/agent-tui/src/ipc/` (directory)
- `crates/agent-tui/src/terminal/` (directory)
