# Workspace Migration Task

You are completing a Cargo workspace restructure for agent-tui following rust-coder patterns.

## Current State

Check what's been done:
1. `ls crates/` - see which crates exist
2. `cat .ralph/TODO.md` - see checklist status
3. `cargo build --workspace 2>&1 | head -50` - see current build state

## Your Task

1. **Check current progress** using the commands above
2. **Pick the next incomplete task** from TODO.md (first unchecked `[ ]` item)
3. **Complete that task**:
   - Create crate Cargo.toml if needed
   - Migrate/create the source file
   - Update imports to rust-coder style (std -> external -> workspace -> crate)
   - Update lib.rs exports
4. **Verify** with `cargo build -p <crate-name>`
5. **Update TODO.md** - mark the task as `[x]` complete
6. **Report** what you did and what's next

## Import Mapping

| Old Import | New Import |
|------------|------------|
| `crate::color` | `agent_tui_common::Colors` |
| `crate::json_ext` | `agent_tui_common::ValueExt` |
| `crate::sync_utils` | `agent_tui_common::{mutex_lock_or_recover, ...}` |
| `crate::terminal` | `agent_tui_terminal::{VirtualTerminal, ScreenBuffer, ...}` |
| `crate::pty` | `agent_tui_terminal::{PtyHandle, PtyError, ...}` |
| `crate::vom` | `agent_tui_core::{Component, Role, analyze, ...}` |
| `crate::session::Element*` | `agent_tui_core::{Element, ElementType, Position}` |
| `crate::session::Session*` | `agent_tui_daemon::{Session, SessionManager, ...}` |
| `crate::client` | `agent_tui_ipc::{DaemonClient, ...}` |
| `crate::daemon::rpc_types` | `agent_tui_ipc::{Request, Response, ...}` |
| `crate::wait` | `agent_tui_daemon::{WaitCondition, ...}` |

## Dependency Order

Create crates in this order (dependencies flow upward):
1. `agent-tui-common` (leaf - no internal deps)
2. `agent-tui-terminal` (depends on common)
3. `agent-tui-core` (depends on terminal, common)
4. `agent-tui-ipc` (depends on common)
5. `agent-tui-daemon` (depends on all above)
6. `agent-tui` (binary - depends on all)

## Stop Condition

When all tasks in TODO.md are marked `[x]` and `cargo build --workspace` succeeds, report:

<promise>RALPH_COMPLETE</promise>
