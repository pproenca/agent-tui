# OpenAI Codex Rust Findings Ledger

## Open Findings

- None.

## Completed Tranches

### `2026-04-12 22:03Z` `A08` Workspace architecture and crate graph

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/Cargo.toml`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/clippy.toml`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/*/Cargo.toml`
- `/Users/pedroproenca/Documents/Projects/agent-tui/ARCHITECTURE.md`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/architecture.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/common/mod.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/test_support/mod.rs`

Passes:

- Workspace lint policy is centralized and enforced through `/Users/pedroproenca/Documents/Projects/agent-tui/cli/Cargo.toml` plus `/Users/pedroproenca/Documents/Projects/agent-tui/cli/clippy.toml`.
- No per-crate Cargo feature sections were found in the audited manifests.
- Unix-only support is enforced consistently through `compile_error!` guards in the façade and layer crates.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/architecture.rs` verifies the intended Clean Architecture crate dependency matrix.
- `2026-04-13 verification` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/*/Cargo.toml` no longer repeats internal crate edges as per-crate path dependencies. A workspace-wide search for `agent-tui-* = { path = ... }` under the member manifests now returns no matches, so internal crate edges are sourced through `/Users/pedroproenca/Documents/Projects/agent-tui/cli/Cargo.toml`'s `workspace.dependencies`.

Findings:

- Shared test support is reusable in practice but not packaged as a workspace member crate.

Contextual non-applicability:

- The codex rule about `transport/api/core` crate stacking is not a direct fit here because this repository uses a Clean Architecture ring split instead of an HTTP client layering model.
- The codex microcrate fanout rule is not a direct fit for an 8-crate workspace with a cohesive `agent-tui-common` crate.

### `2026-04-12 22:06Z` `A01` Domain invariants and semantic types

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/session_types.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/types.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs`

Passes:

- `SessionId` implements the full ergonomic reference hierarchy (`Borrow`, `AsRef`, `Deref`, `TryFrom`) while preserving its non-empty invariant.
- The public condition enums in `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/types.rs` are marked `#[non_exhaustive]`.
- `2026-04-13 revalidation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/types.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/params.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs` now thread `TerminalSize` through the domain and RPC boundary. CLI commands still parse `u16` flags, but they validate at parse time and construct `TerminalSize` at the first app boundary before any use-case or runtime call.

Findings:

- None after revalidation.

Contextual non-applicability:

- The codex `Unknown`-variant forward-compat rule is not a direct fit for the current condition enums because they model validated command inputs rather than persisted open-ended wire unions.

### `2026-04-12 22:07Z` `A02` Error taxonomy and translation boundaries

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/errors.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-common/src/common/daemon_error.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/error.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/error.rs`

Passes:

- Retryability is determined by explicit variant matches in the shared terminal and daemon error types instead of ad hoc string checks.
- `AttachError` keeps presentation payload separate from the enum via `AttachErrorContext` and `AttachErrorPayload`.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/error.rs` now preserves persistence failures as a dedicated `DomainError::PersistenceError` variant, so adapter-facing code, context, display text, and retryability no longer collapse into a generic string payload.

Findings:

- None after remediation.

### `2026-04-12 22:12Z` `A10` Test harnesses and auditability of tests

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/common/mock_daemon.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/common/test_harness.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/common/real_test_harness.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/common/interactive_pty.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/cli_command_contracts.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/cli_smoke.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/system_e2e.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/transport.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/Cargo.toml`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/Cargo.toml`

Passes:

- The suite already covers three useful realism layers: `MockDaemon` drives real Unix-socket JSON-RPC, `RealTestHarness` boots the actual daemon subprocess, and `InteractivePtyRunner` exercises attach flows through a real PTY.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/transport.rs` uses `USE_DAEMON_START_STUB` to opt test-only daemon-autostart behavior into a single build instead of fragmenting the build graph with a cargo feature.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/common/mock_daemon.rs` can inject delays, malformed frames, disconnects, and junk lines while still exercising the real JSON-RPC transport boundary.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/Cargo.toml`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/Cargo.toml`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/Cargo.toml` now wire `insta` into the audited test manifests, and the terminal-facing suites in `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/render.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/vterm.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/snapshot.rs` now pin rendered output, whitespace, ANSI runs, and resize/reflow behavior as snapshot fixtures.
- `2026-04-13 remediation` the workspace has now completed the sibling-test migration across `cli/crates`. Large implementation files such as `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/transport.rs` now attach sibling `#[path = "..._tests.rs"]` modules, and `rg -n -F 'mod tests {' cli/crates -g '*.rs'` returns no matches.
- `2026-04-13 remediation` the audited timing-heavy suites now live in `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core_tests.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server_tests.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server_tests.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/transport_tests.rs`; a direct search for `park_timeout`, `Instant::elapsed()`, and `.elapsed()` across those sibling test files now returns no matches.
- `2026-04-13 verification` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/deny.toml` now scopes `cargo-deny` to the supported Unix targets and documents the remaining upstream-constrained duplicate versions explicitly, so `cd cli && cargo deny check advisories bans licenses sources` completes with `advisories ok, bans ok, licenses ok, sources ok`.

Findings:

- None after remediation.

Contextual non-applicability:

- The codex `wiremock` plus SSE helper rule does not map literally because this repository's main runtime boundary is Unix socket JSON-RPC plus WebSocket, not outbound HTTP SSE. The underlying "real wire, not trait mock" goal is still applicable and partly met by the existing harnesses.
- The closure-builder fixture pattern is not a strong fit for the current harness API because the scripting surface is still small and request-oriented rather than a large mutable config object.
- Some remaining workspace tests still use bounded real-time polling for OS-visible process, file-lock, and PTY state transitions. Those are not the async-timer cases audited here, and paused Tokio time cannot model those external kernel transitions directly.

### `2026-04-12 22:19Z` `F01` Session spawn and initial run

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/commands.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/params.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/handlers/session.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/session.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/spawn_error.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/repository.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/pty_session.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/pty.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/types.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/session_types.rs`

Passes:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs` resolves the invocation cwd for local transport and intentionally omits a default cwd for WebSocket transport, with dedicated tests covering both branches.
- Spawn failure translation stays centralized through the stack: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/pty.rs` classifies missing-command and permission-denied failures, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/session.rs` maps `SessionError` into `SpawnError`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/handlers/session.rs` converts that boundary error once into RPC output.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs` preserves command, args, cwd, env, and active-session state across restart, and the marker-file test demonstrates that a restarted session inherits the stored launch spec correctly.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/repository.rs` keeps the `SessionRepository` trait object-safe by exposing handle adapters instead of leaking the concrete `Session` type across the use-case boundary.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/commands.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/mod.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/params.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs` now thread explicit request-scoped environment overrides from CLI `run --env KEY=VALUE` and JSON-RPC `spawn.env` through to `SpawnInput` instead of hardcoding `env: None`.
- `2026-04-13 revalidation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/pty.rs` now has a PTY-level regression test proving the spawned child process observes the explicit environment override, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/cli_smoke.rs` covers the CLI path end to end on the green base.
- `2026-04-13 revalidation` the stale terminal-size note from `A01` no longer applies here because CLI and RPC spawn flows now validate and carry `TerminalSize` before the runtime boundary.

Findings:

- None after remediation.

Contextual non-applicability:

- The codex sandbox rules about `env_clear`, `pre_exec` tethering, re-exec helpers, argv[0] binary multiplexing, and three-layer network isolation are not a direct fit for `agent-tui run`, which intentionally launches the operator-requested interactive process and must preserve a usable shell environment rather than starting from an empty sanitized sandbox.
- The codex closure-builder fixture rule is not a strong fit for this tranche because the existing spawn harness surface is narrow and scenario-oriented rather than a large mutable configuration object.

### `2026-04-12 23:19Z` `F05` Resize and terminal reflow

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/commands.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/params.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/handlers/session.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/types.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/session_types.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/session.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/errors.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/terminal_engine.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/error.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/pty.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/render.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/vterm.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/cli_command_contracts.rs`

Passes:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/error.rs` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/errors.rs` keep resize as a dedicated error variant with structured `operation` and `reason` payloads instead of collapsing it into opaque strings.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/handlers/session.rs` still translates the resize use case exactly once at the RPC handler boundary.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core.rs` already has a focused regression test proving that live preview subscribers receive a `resize` event with the updated `cols` and `rows`.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/render.rs` trims trailing blank rows and resets styles at the end of rendered output, which reduces stale-style leakage when the screen is re-rendered after a resize.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs` now routes both the initial attach-side terminal-size sync and live `Event::Resize` updates through a shared resize helper, formats any RPC failure once, and renders that warning back into the attach status line instead of discarding it silently. The attach-focused unit coverage now pins that warning formatting path.
- `2026-04-13 revalidation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/types.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/params.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/session.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/pty.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/vterm.rs` now keep resize semantics on validated `TerminalSize` values end to end.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/vterm.rs` now snapshots resize/reflow state before and after a terminal-width change, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/render.rs` now snapshots both the full rendered frame and the compact render so wrapped-text and whitespace regressions are pinned as fixtures instead of inferred from helper assertions.

Findings:

- None after remediation.

Contextual non-applicability:

- The codex `FrameRequester` and hysteresis chunking rules are not a direct fit for this tranche because resize propagation here is event-driven attach/live-preview signaling, not a local Ratatui animation loop or streamed line-chunk backlog.

### `2026-04-13 00:20Z` `A03` Session repository and persistence internals

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/session_repository.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/repository.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/terminal_state.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/system_clock.rs`

Passes:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/session_repository.rs` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/repository.rs` keep `SessionRepository` object-safe by routing all live-session behavior through `SessionHandle`, `SessionOps`, and `StreamWaiterHandle`, and the local tests pin both object safety and generic usability.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs` preserves forward compatibility for the JSONL session store even with a strict `SessionEvent` enum: unknown records increment `unknown_records`, compaction is skipped when those records exist, destructive `save()` rewrites are refused, and `cleanup_stale_sessions()` appends `remove` events instead of rewriting away future entries.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs` uses identity-aware stale-session cleanup rather than killing by PID alone. Startup cleanup cross-checks the persisted `created_at` and command against live process metadata before terminating a process group, and the tests cover both orphan cleanup and future-record preservation.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs` no longer treats persistence failure as a hidden side effect for the common spawn/kill paths. `spawn()` now refuses to register a live session if the JSONL append fails, and `kill()` now returns the persistence failure while keeping the stopped session registered until the log removal succeeds. Focused tests cover both failure paths directly.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-common/src/common/daemon_error.rs` now make startup and restart persistence failures part of the operation result. Session-manager construction fails when stale-session cleanup cannot acquire or repair storage, daemon startup surfaces that constructor failure structurally, and `restart()` now persists the replacement session before killing the original so a failed replacement append leaves the original session alive instead of returning false success.

Findings:

- None after remediation.

Contextual non-applicability:

- `async-abort-on-drop-handle` and `async-shared-boxfuture-joinhandle` are not a direct fit here because this slice uses a dedicated std-thread pump with a single explicit join owner, not a tokio task tree with multi-waiter futures.
- `testing-paused-runtime-advance` is not a direct fit for the persistence cleanup tests because they exercise real OS processes, file locks, and blocking waits instead of tokio timers.

### `2026-04-13 06:53Z` `A04` PTY and virtual terminal engine

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/pty.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/render.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/vterm.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/pty_session.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/core/mod.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/core/screen.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/core/style.rs`

Passes:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/pty.rs` handles partial PTY writes defensively: it retries `Interrupted`, waits for `POLLOUT` on `WouldBlock`, and treats `write == 0` as a closed-terminal error instead of silently dropping input.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/pty.rs` already prefers process-group-aware termination before falling back to a direct child kill, which reduces orphaned descendants when the PTY child is the process-group leader.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/render.rs` still preserves the key rendering guarantees previously observed in `F05`: compact rendering trims trailing blank rows and the renderer explicitly resets terminal style state at the end of the frame.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/vterm.rs` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/core/{screen,style}.rs` keep the terminal-engine boundary clean by translating wezterm cell/cursor data into domain `ScreenSnapshot`, `ScreenCell`, `CursorPosition`, `CellStyle`, and `Color` types instead of leaking engine-specific types upward.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/pty_session.rs` remains a thin boundary adapter that converts `PtyError` into `SessionError` exactly once for the rest of the daemon runtime.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs` already caps retained live stream bytes in `StreamBuffer` and tracks `dropped_bytes`, so long-running sessions do not let the in-memory output history grow without bound even when consumers lag behind.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/pty.rs` now drives PTY reader delivery through an unbounded event queue, owns a shutdown signal for the `pty-reader` thread, and joins that thread with a bounded timeout plus background reaper fallback during teardown. Focused PTY tests now prove both the non-blocking event-queue behavior and the blocked-reader shutdown path.
- `2026-04-13 revalidation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/pty_session.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/pty.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/vterm.rs` already accept `TerminalSize`, not raw `(u16, u16)` pairs, so the invariant gap previously noted in this tranche is no longer present.

Findings:

- None after remediation and revalidation. The remaining rendered-screen snapshot gap is tracked in `F02`/`F05` instead of duplicating it here.

Contextual non-applicability:

- `tui-drop-guard-panic-hook-chain` and `tui-event-broker-pause-resume` are not direct fits for this tranche because the audited files implement PTY transport and screen modeling, not raw-mode/stdin ownership during full-screen attach.
- `async-abort-on-drop-handle` and `async-shared-boxfuture-joinhandle` are not direct fits because this slice uses std threads and crossbeam channels rather than a tokio task tree with shared join futures.
- `types-non-exhaustive-public-enums` and `types-unknown-variant-forward-compat` are not direct fits for `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/core/{screen,style}.rs` because those types model internal screen state, not versioned external wire enums.

### `2026-04-13 06:59Z` `A05` Concurrency, shutdown, and thread/task ownership

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/shutdown.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/shutdown_notifier.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-common/src/common/sync.rs`

Passes:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs` applies bounded backpressure at the listener boundary: accepted Unix connections enter a bounded `sync_channel` via `try_send`, so overload drops work at the edge instead of stalling the accept loop or worker runtime.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core.rs` rechecks shutdown and connection cancellation before every `STREAM_WAIT_SLICE`, and the stream-wait tests show that long-heartbeat streams still terminate promptly once cancellation flips.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs` has a solid per-connection cancellation path: `cancel_stream_task` drops the bounded receiver to release blocked `blocking_send`, `wait_for_stream_task` bounds the grace wait, and timed-out stream tasks are explicitly aborted instead of hanging teardown.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-common/src/common/sync.rs` recovers poisoned mutex and `RwLock` guards with logging instead of panicking, keeping the daemon runtime alive after a background thread unwinds while preserving a recovery breadcrumb.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/shutdown_notifier.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/shutdown.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/signal_handler.rs` now make shutdown wake delivery fallible end to end: notifier writes return `io::Result`, the shutdown use case reports `acknowledged: false` when the wake byte cannot be delivered, and signal-driven wake failures are logged instead of being silently swallowed.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs` no longer drop the last owner of timed-out shutdown threads. Both shutdown paths now hand unfinished `JoinHandle`s to named background reaper threads, and the WS shutdown path keeps the state file in place when the runtime thread is still alive instead of deleting it optimistically.

Findings:

- None after remediation.

Contextual non-applicability:

- `async-child-cancellation-tokens` and `async-shared-boxfuture-joinhandle` are not direct fits because this slice mainly uses std threads, watch channels, condvars, and atomics rather than nested tokio task trees with shared join futures.
- `testing-wiremock-sse-fakes` is not a direct fit because the concurrency boundary under review is local daemon/runtime coordination rather than an outbound protocol fake.

### `2026-04-13 07:33Z` `F08` Daemon lifecycle control plane

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/client.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/daemon_lifecycle.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/transport.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/file_lock.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/signal_handler.rs`

Passes:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs` keeps the daemon operator UX idempotent and structured: start/stop/restart/status all have dedicated JSON outputs plus dry-run and confirmation flows, and `daemon start` only auto-starts after retryable local connect failures instead of masking arbitrary transport errors.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/transport.rs` already uses single-build test hooks for daemon autostart, with `USE_DAEMON_START_STUB` and `DAEMON_START_TEST_REAPED` covering stale-socket recovery, early child exit, recursive-spawn refusal, and reaper fallback without introducing a cargo feature split.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/file_lock.rs` keeps lock-file failures structured: `open`, `flock`, `truncate`, and PID-write errors all become `DaemonError::LockFailed { operation, source }`, so the lifecycle boundary does not have to reverse-engineer raw `io::Error` strings later.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/signal_handler.rs` use an explicit wake pipe so signal-triggered shutdown can break the accept loop out of `poll()` without waiting for another client connection.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/daemon_lifecycle.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/process.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs` now re-check daemon liveness after acknowledged RPC shutdown, fall back to signal-based stop when the process remains alive, escalate from `SIGTERM` to `SIGKILL` after the bounded grace window, and treat unreaped zombie children as exited instead of still-running. Focused lifecycle tests plus the ignored real-daemon core E2E suite now cover the fixed path.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/client.rs` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs` now preserve invalid daemon lock state as `DaemonStateInvalid { path, message }` instead of collapsing it into a fake signal error. Status, stop, and restart now report coordination-file corruption with the correct boundary context.

Findings:

- The previously recorded timing-test gap extends into this slice too: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/daemon_lifecycle.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/transport.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs` still verify shutdown and startup timeouts with real polling and elapsed-time bounds instead of deterministic time control.

Contextual non-applicability:

- `sandbox-env-clear-pre-exec`, `sandbox-three-layer-network-isolation`, and the other codex sandbox-helper rules are not direct fits for `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/transport.rs` because this slice is spawning the daemon itself, which is intentionally supposed to inherit operator environment and outlive the initiating CLI process rather than act like a tightly sandboxed child workload.
- `async-abort-on-drop-handle`, `async-child-cancellation-tokens`, and `async-shared-boxfuture-joinhandle` are not direct fits because this control plane is primarily std-process and std-thread lifecycle code, not a nested tokio task tree.

### `2026-04-13 07:48Z` `F07` Session lifecycle management

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/handlers/session.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/router.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/session.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/session_repository.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/test_support/mock_repository.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/repository.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs`

Passes:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs` keeps the destructive lifecycle commands operator-safe at the CLI boundary: `kill`, `restart`, and `sessions cleanup` all expose `--dry-run` previews and require explicit confirmation before mutating live sessions.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/session_repository.rs` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/repository.rs` keep the lifecycle boundary thin and object-safe by delegating `resolve`, `set_active`, `list`, `kill`, and `restart` straight through the port instead of mixing session-state policy into adapter code.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs` already repairs stale active-session pointers defensively: `resolve(None)` falls back to the most recent running session, clears `active_session` when none remain, and has focused tests for stale-active repair, explicit-session resolution, fallback promotion, and the no-running-sessions case.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs` preserves launch context across restart. The restart path reuses the prior command, args, cwd, env, and terminal size, promotes the replacement session to active, and has a regression test that proves the restarted process inherits the original working directory and environment.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/session.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/error.rs` now keep explicit session-selector validation and stopped-session attachment errors structured end to end. Malformed selectors fail as invalid params, and attaching to a known-but-stopped session stays a dedicated `NotRunning` error instead of a fake `NotFound`.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs` now treats lock-contention rows conservatively and keeps killed sessions registered until persistence removal succeeds. `list()` falls back to persisted metadata plus `running: true` on a timed-out lock instead of synthesizing a fake stopped `"(locked)"` row, `spawn()` refuses to register a session when persistence append fails, and `kill()` no longer drops the registry entry before the process and persistence removal both succeed.

Findings:

- None after remediation.

Contextual non-applicability:

- `async-abort-on-drop-handle`, `async-child-cancellation-tokens`, and `async-shared-boxfuture-joinhandle` are not direct fits for this tranche because the lifecycle surface is primarily lock/PTY/session-manager code over std threads and shared state, not a nested tokio task tree with owned join futures.
- `testing-wiremock-sse-fakes`, `testing-insta-snapshot-tui-rendering`, and the TUI-specific rendering rules are not direct fits here because this slice is session selection and metadata mutation rather than outbound protocol fakes or rendered-screen correctness.

### `2026-04-13 08:04Z` `F02` Snapshot and screenshot rendering

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/commands.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/handlers/snapshot.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/params.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/types.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/snapshot.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/test_support/mock_session.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/repository.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/render.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/vterm.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/cli_smoke.rs`

Passes:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs` preserve the richer rendered path when ANSI output is desired: the CLI text presenter prefers `compact_rendered` and then `rendered`, and the RPC responder also prefers rendered output over the raw `screenshot` fallback when `retain_ansi` is enabled.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/handlers/snapshot.rs` remains a thin boundary translator. It parses once, forwards the use case exactly once, and translates `SessionError` exactly once on the way back out.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs` has a focused regression test proving that the initial attach render prefers the full `rendered` payload over the plain `screenshot` fallback when both are present.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/snapshot.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/repository.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/handlers/snapshot.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/system_e2e.rs` now reject named regions as structured invalid input, propagate flush-ack timeout or disconnect failures instead of returning stale snapshots, and cover the real-daemon named-region failure path end to end.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/Cargo.toml`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/Cargo.toml`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/Cargo.toml` now wire `insta` into the render-owning crates, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/render.rs` now snapshots ANSI-heavy rendered output plus trimmed blank-row layout, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/vterm.rs` now snapshots pre/post-resize render state, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/snapshot.rs` now snapshots the rendered screenshot contract returned by the use case.

Findings:

- None after remediation.

Contextual non-applicability:

- `tui-drop-guard-panic-hook-chain`, `tui-event-broker-pause-resume`, and `tui-schedule-frame-coalescer` are not direct fits for this tranche because the audited path is request/response screen capture, not raw-mode ownership or a local frame scheduler.
- The codex sandbox rules are not direct fits for this slice because screenshot rendering itself does not spawn or isolate new subprocesses beyond the already-audited session runtime.

### `2026-04-13 08:13Z` `F03` Input injection

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/commands.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/mod.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/handlers/input.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/handlers/diagnostics.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/input.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/diagnostics.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/pty.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/cli_command_contracts.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/system_e2e.rs`

Passes:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs` refuses `type -` when stdin is an interactive TTY, and when it does read from stdin it attaches useful context to read failures instead of silently blocking or swallowing the error.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/handlers/{input,diagnostics}.rs` remain thin boundary translators: each handler parses once, runs exactly one use case, and maps `SessionError` exactly once on the way back out.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/pty.rs` still provides the core injection safety guarantees already observed in `A04`: `write()` retries `Interrupted`, waits on `WouldBlock`, treats zero-byte writes as terminal closure, and `key_to_escape_sequence()` rejects unsupported symbolic keys instead of guessing.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs` enables bracketed paste and already has a dedicated raw-byte path for explicit `Event::Paste(data)` frames, so terminals that emit real paste events avoid per-character translation on the attach path.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/cli_command_contracts.rs` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/system_e2e.rs` cover the basic routing surface: `press`, `press --hold`, `press --release`, `type`, and `scroll` map to the expected RPCs, and the slow end-to-end runtime test exercises `type` plus `press Enter` successfully against a real session.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs` now buffers likely unbracketed paste bursts behind a timing-based state machine, flushes buffered text in order before non-character events, and bypasses detach detection for confirmed paste bursts while still preserving custom detach-key semantics for ordinary typed characters. The attach-focused unit coverage now pins the burst transitions and detach-prefix cancellation behavior.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs` now applies held modifiers to subsequent injected keys and typed text, and the ignored real-daemon input E2E now proves that modifier hold changes the resulting terminal bytes instead of remaining dead state.

Findings:

- None after remediation.

Contextual non-applicability:

- `testing-insta-snapshot-tui-rendering` is not a direct fit for this tranche because the audited behavior is byte/escape-sequence semantics and attach event handling rather than rendered screen output.
- `async-bounded-vs-unbounded-channel-split` is not a direct fit for the CLI `press` and `scroll` loops because they synchronously issue one RPC at a time instead of owning a long-lived submission/event pair; the main input-specific risk here sits in semantic translation rather than channel topology.

### `2026-04-13 08:27Z` `F04` Wait and assert semantics

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/commands.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/handlers/wait.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/handlers/session.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/types.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/wait.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/wait_condition.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/session.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/session_repository.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/repository.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/common/test_harness.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/common/mock_daemon.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/cli_command_contracts.rs`

Passes:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/types.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/wait_condition.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/session.rs` keep wait/assert condition handling future-proof: the public enums are `#[non_exhaustive]`, and the use-case layer turns unexpected future variants into explicit unsupported-condition errors instead of assuming the current variant set is complete.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/handlers/{wait,session}.rs` remain thin wait/assert boundary translators: each handler parses once, executes one use case, and maps `SessionError` exactly once on the way back out.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/session_repository.rs` still exposes the wait path through object-safe `SessionHandle` and `StreamWaiterHandle` traits instead of concrete daemon-session types, so the use cases stay decoupled from the runtime implementation details below them.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/cli_command_contracts.rs` already covers the public wait surface against `MockDaemon` over real JSON-RPC frames, including route selection for text/stable/gone wait modes and exit-code `75` for `wait --assert` timeouts.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/wait.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/wait_condition.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs` now keep refresh ownership in the outer poll loop, reject blank explicit session selectors at parse time, and cover poll-interval limits, stream wakeups, stability convergence, and timeout boundaries through the existing mock clock seam.

Findings:

- None after remediation.

Contextual non-applicability:

- `testing-insta-snapshot-tui-rendering` is not a direct fit for this tranche because the audited behavior is predicate evaluation, freshness, and timeout semantics rather than full rendered-screen diffs.
- `async-bounded-vs-unbounded-channel-split` is not a direct fit here because the wait path consumes an existing `StreamWaiter` subscription instead of designing a new submission/event channel pair.

### `2026-04-13 08:37Z` `F06` Interactive attach session

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/rpc_client.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/error.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/system_e2e.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/common/interactive_pty.rs`

Passes:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/error.rs` still keeps attach failures structured instead of flattening them into one display string: `AttachError` exposes stable `code`, `category`, `retryable`, `context`, and `suggestion` data through `AttachErrorPayload`.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core.rs` keeps the attach-stream server boundary narrow: it parses once, resolves the session once, emits explicit `ready`/`output`/`dropped`/`heartbeat`/`closed` events, and translates parse/session/update failures back into JSON-RPC responses at the edge instead of leaking transport details through the loop.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/system_e2e.rs` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/common/interactive_pty.rs` already cover the happy detach surface with real PTY interaction: slow end-to-end tests exercise both default and custom detach keys, and the interactive PTY helper deliberately breaks a full sync-channel send before joining its reader thread during teardown.

Findings:

- Terminal setup failures are still swallowed before attach starts. `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs` enables raw mode and then ignores `prepare_terminal()` errors, so attach can continue after alternate-screen or bracketed-paste setup has already failed.
- Attach output teardown is only graceful on the explicit detach path. `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/rpc_client.rs` exposes an abort handle, but `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs` only uses it when the detach-key sequence completes. Other exits rely on `AttachOutputWorker::drop()`, which waits briefly and then detaches the helper thread if `StreamResponse::next_result()` is still blocked.
- Terminal restoration still lacks a chained panic hook. `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs` restores terminal state via `Drop`, but there is still no repo-wide `panic::set_hook`/`take_hook` chain to restore the terminal before panic output is emitted.

Contextual non-applicability:

- `testing-paused-runtime-advance` is not a direct fit for this tranche because the attach runtime and its PTY harness rely on real terminal I/O, `crossterm` polling, and std threads rather than tokio timer-driven tasks.
- `tui-schedule-frame-coalescer` and `tui-two-gear-hysteresis-chunking` remain non-applicable here because attach is a passthrough stream consumer, not a local redraw scheduler or backlog chunker.

### `2026-04-13 08:48Z` `F09` Live preview control plane

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/process.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/cli_command_contracts.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/docs/api/openapi.yaml`

Passes:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs` still preserves the most important local-exposure guardrail on the listener itself: `bind_listener()` rejects non-loopback binds unless `AGENT_TUI_WS_ALLOW_REMOTE=1`, and the WS state file is written with best-effort `0600` permissions.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/process.rs` keep UI-stop ownership bounded: `stop_ui_server_with_controller_and_timeouts()` escalates from `SIGTERM` to `SIGKILL` with explicit grace windows, and the process controller maps liveness checks into typed `ProcessStatus` values instead of leaking raw errno inspection through the handler logic.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/cli_command_contracts.rs` already covers one useful control-plane guarantee: `live start`, `live status`, and `live stop` are exercised as standalone local commands even when `AGENT_TUI_TRANSPORT=ws` points at a remote address, so the CLI does not accidentally route this surface over the selected remote transport.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/docs/api/openapi.yaml` now enforce the local-only live-preview boundary coherently. `build_ui_url()` rejects cross-origin `AGENT_TUI_UI_URL` overrides, browser WebSocket upgrades must present an `Origin` matching the daemon UI origin, and the published HTTP contract now documents the authenticated `/ws` plus `/api/v1/stream` surface instead of the removed session/snapshot routes. Listener-level tests now boot the axum server and prove `/` redirect, `/ws` auth rejection, `/api/v1/stream` aliasing, and cross-origin browser rejection end to end.

Findings:

- None after remediation.

Contextual non-applicability:

- `proto-sse-idle-timeout-terminator` is not a direct fit for this tranche because the live preview control plane uses WebSocket plus JSON-RPC rather than SSE framing or terminators.
- `testing-insta-snapshot-tui-rendering` is not a direct fit here because this slice is about URLs, auth, state files, and browser/control-plane behavior rather than terminal rendering diffs.

### `2026-04-13 08:54Z` `F10` Live preview data plane

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/session_repository.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/docs/api/openapi.yaml`
- `/Users/pedroproenca/Documents/Projects/agent-tui/docs/api/asyncapi.yaml`

Passes:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core.rs` keeps the live-preview and flightdeck stream boundary narrow: it translates `SessionError` values back into JSON-RPC error envelopes exactly at the stream edge, while successful `ready`/`init`/`output`/`dropped`/`resize`/`heartbeat`/`closed` payloads stay as separate wire events.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core.rs` already applies the Codex debug-assert pattern for invalid preview dimensions. `validated_terminal_size()` raises a `debug_assert!` in debug builds, logs the bad payload, and falls back to `TerminalSize::default()` instead of panicking in release.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/session_repository.rs` keep the stream semantics explicit for the wire layer: subscribers own independent cursors, dropped bytes are surfaced numerically, and `live_preview_snapshot()` couples a full-screen init frame with the current stream sequence so the data plane can resynchronize after buffer loss.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs` already bounds the inner stream bridge: `run_stream_connection()` uses a bounded queue, cancels cooperatively by setting a flag and dropping the receiver to release blocked senders, and then aborts the blocking stream task if it misses the shutdown deadline.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/docs/api/asyncapi.yaml` now documents the actual authenticated JSON-RPC stream protocol, including `live_preview_stream` and `flightdeck_stream` request envelopes, `/ws` plus `/api/v1/stream` channel aliasing, and the real `ready`/`init`/`output`/`dropped`/`resize`/`heartbeat`/`closed` result shapes. `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs` now has listener-level tests for authenticated flightdeck streaming and live-preview `ready`/`init`/`output`/`closed` delivery over a real WebSocket connection.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs` now add deterministic test seams for live-preview stream timing and stream-buffer size, and the listener suite covers both idle-heartbeat delivery and `dropped -> init` re-synchronization over a real WebSocket connection instead of only through in-process `rpc_core` tests.

Findings:

- None after remediation.

Contextual non-applicability:

- `proto-sse-idle-timeout-terminator` is not a direct fit for this tranche because the live-preview data plane is WebSocket plus JSON-RPC with explicit heartbeat events rather than SSE framing and terminators.
- `tui-schedule-frame-coalescer` and `tui-two-gear-hysteresis-chunking` are not a direct fit here because this slice forwards PTY bytes and full-screen init snapshots rather than running a local Ratatui redraw scheduler or chunk-hysteresis backlog.

### `2026-04-13 09:04Z` `A11` Rust-to-web contract boundary

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/web/src/app.ts`
- `/Users/pedroproenca/Documents/Projects/agent-tui/web/src/session_view_model.ts`
- `/Users/pedroproenca/Documents/Projects/agent-tui/web/src/session_view_model.test.ts`
- `/Users/pedroproenca/Documents/Projects/agent-tui/web/src/ui_layout_regression.test.ts`
- `/Users/pedroproenca/Documents/Projects/agent-tui/docs/api/openapi.yaml`
- `/Users/pedroproenca/Documents/Projects/agent-tui/docs/api/asyncapi.yaml`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs`

Passes:

- `/Users/pedroproenca/Documents/Projects/agent-tui/web/src/session_view_model.ts` keeps the browser-side session contract narrow and aligned with the Rust publisher: the reducer logic only depends on `id`, `command`, `pid`, `running`, `created_at`, `size`, and the active-session identifier, and `/Users/pedroproenca/Documents/Projects/agent-tui/web/src/session_view_model.test.ts` already pins the reconnect and active-session selection semantics around that reduced shape.
- `/Users/pedroproenca/Documents/Projects/agent-tui/web/src/app.ts` and `/Users/pedroproenca/Documents/Projects/agent-tui/web/src/ui_layout_regression.test.ts` preserve an important control-boundary guarantee for the live preview surface: session switching stays preview-local, the browser issues an explicit `resize` RPC with `session`, `cols`, and `rows`, and the static regression test asserts that this path has not regressed into an attach-style RPC.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/web/src/connection_config.ts`, `/Users/pedroproenca/Documents/Projects/agent-tui/web/src/live_preview_contract.ts`, `/Users/pedroproenca/Documents/Projects/agent-tui/web/src/app.ts`, `/Users/pedroproenca/Documents/Projects/agent-tui/web/src/live_preview_contract.test.ts`, and `/Users/pedroproenca/Documents/Projects/agent-tui/docs/api/{openapi,asyncapi}.yaml` now keep the browser contract in lockstep with the Rust publisher. The web client no longer falls back to same-origin `/ws` when `ws` is missing, it parses real JSON-RPC envelopes through explicit contract helpers, it ignores unsupported `command` events instead of carrying a dead branch, the dead command-timeline UI has been removed, and the web test suite now feeds real `RpcResponse` envelopes and daemon-provided endpoint failures directly.

Findings:

- None after remediation.

Contextual non-applicability:

- `proto-sse-idle-timeout-terminator` is not a direct fit for this tranche because the Rust-to-web boundary here is WebSocket plus JSON-RPC, not SSE framing.
- `tui-schedule-frame-coalescer` and `tui-two-gear-hysteresis-chunking` are not a direct fit here because this slice audits browser contract semantics and wire payload assumptions rather than a local Ratatui redraw scheduler.

### `2026-04-13 09:16Z` `F11` IPC transport and client/server RPC

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/client.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/transport.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/error.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/socket.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/polling.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/daemon_lifecycle.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/rpc_client.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/transport/error.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/transport/unix_socket.rs`

Passes:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/transport/error.rs` keeps the daemon-side transport taxonomy explicit: timeout, closed-connection, parse, serialize, size-limit, and raw I/O failures are represented as separate variants before `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs` translates them once at the RPC boundary.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/error.rs` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/client.rs` preserve useful structured RPC error metadata on the client side: code, category, retryable, context, and suggestion survive parsing and can be rendered as JSON or surfaced to the operator without flattening everything into a single string.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/transport/unix_socket.rs` now applies the request-size guard per request line instead of cumulatively across the socket, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/client.rs` now retries retryable unary RPC failures using `DaemonClientConfig.max_retries` and `initial_retry_delay` while preserving `retry_delay_ms` wire hints. Focused tests now cover a real Unix-socket round-trip and per-request multi-line accounting.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/client.rs` now routes streaming handshakes through the same retry/backoff surface as unary RPC calls, including `retry_delay_ms` hints, and now has a real Unix-socket timeout regression test that exercises the client/server boundary end to end.

Findings:

- None after remediation.

Contextual non-applicability:

- `proto-sse-idle-timeout-terminator` is not a direct fit for this tranche because the IPC boundary is line-delimited JSON-RPC over Unix sockets and WebSocket, not SSE.
- `sandbox-three-layer-network-isolation` is not a direct fit here because the remaining transport findings are about framing, retry, and request accounting; the bind and process-isolation policy sits in `A06` and the browser-exposure review already landed in `F09`.

### `2026-04-13 09:24Z` `A06` Process control and isolation boundary

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/process.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/daemon_lifecycle.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/transport.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/build.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/src/main.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/src/lib.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/lib.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/lib.rs`

Passes:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs` avoids the most obvious browser-launch injection trap: `open_in_browser()` parses the override with `shell_words` and executes an explicit program plus argv via `Command::new(...)` instead of routing through `sh -c`.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs` still preserves the most important bind-safety guardrail on the live preview listener: `bind_listener()` rejects non-loopback addresses unless `AGENT_TUI_WS_ALLOW_REMOTE=1`, and that rejection already has a focused regression test.
- Unix-only runtime assumptions remain explicit and compile-time enforced in `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/src/main.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/src/lib.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/lib.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/lib.rs` via `#[cfg(not(unix))] compile_error!(...)`.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/process.rs` keeps the raw signal/liveness translation narrow: `kill(pid, 0)` results are converted into typed `Running`, `NotFound`, and `NoPermission` states before the higher-level daemon/UI control paths consume them.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/process.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/client.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/file_lock.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs` now persist daemon/UI process identity as `pid + process_started_at`, reject mismatched or stale state files, and cover reused PID scenarios in focused tests.

Findings:

- None after remediation.

Contextual non-applicability:

- `sandbox-env-clear-pre-exec` is not a direct fit for background daemon self-spawn here because `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/transport.rs` intentionally carries operator-selected environment configuration into the daemon child; the earlier `F01` finding remains the relevant env-propagation issue for spawned terminal sessions.
- `sandbox-argv0-multiplex-binary` is not a direct fit for this tranche because the reviewed process launches use the current binary with an explicit subcommand or a direct browser opener command, not symlinked helper binaries selected through `argv[0]`.

### `2026-04-13 09:31Z` `F12` CLI admin and operator UX

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/mod.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/commands.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/error.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/presenter.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-common/src/common/error_codes.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-common/src/common/daemon_error.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/cli_command_contracts.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/cli_smoke.rs`

Passes:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/error.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-common/src/common/error_codes.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/mod.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/presenter.rs` keep the shared error and exit-surface mostly centralized: attach/client/daemon failures preserve category, retryability, suggestion, and context, and JSON mode wraps those structured payloads instead of flattening everything into ad hoc strings.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/mod.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/commands.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/cli_command_contracts.rs` still preserve useful operator guidance on the command line itself: parse errors append a command-specific `Example:` help hint, help entrypoints are regression-tested across the subcommand tree, and the destructive daemon/session flows retain dry-run plus explicit confirmation semantics.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/mod.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/cli_command_contracts.rs` now give `completions` a structured JSON contract, return only one `live stop` failure for a failing managed UI helper, and cover JSON completions plus the `--no-input` and failing `live stop` standalone paths end to end.

Findings:

- None after remediation.

Contextual non-applicability:

- `testing-insta-snapshot-tui-rendering` is not a direct fit for this tranche because the audited surface is plain text or JSON CLI output plus shell-completion scripts, not a Ratatui render tree.
- `workspace-layered-transport-api-core` is not a direct fit for this tranche because the remaining questions are operator-facing command contracts and error presentation, not transport/api/core crate factoring.

### `2026-04-13 09:40Z` `A07` Observability and runtime diagnostics

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-common/src/common/telemetry.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/transport.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/error.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-common/src/common/daemon_error.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-common/src/common/error_codes.rs`

Passes:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-common/src/common/telemetry.rs` still keeps tracing bootstrap centralized and failure-tolerant: log format, stream, and file selection are environment-driven in one place, file-open failure degrades to stderr with a warning instead of panicking, and a second subscriber installation simply disables the guard instead of crashing.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs` already emits useful operational spans and timings around the daemon boundary: worker, connection, request, and stream handling each have dedicated spans, and request or stream completion records `elapsed_ms` so slow-path diagnostics are at least visible when tracing is enabled.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/error.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-common/src/common/daemon_error.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-common/src/common/error_codes.rs` preserve structured diagnostic payloads across CLI, daemon, and attach surfaces instead of reducing runtime failures to opaque strings.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs` no longer logs tokenized startup URLs. Startup diagnostics now record only the bound listen address plus fixed UI/WS paths, which removes the most obvious live-preview bearer-token leak from ordinary logs.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-common/src/common/telemetry.rs` now supports layered telemetry when `AGENT_TUI_LOG` is configured: a human-oriented stream sink can keep terse text output on stdout/stderr while a file sink retains richer diagnostics, targets, and its own filter via `AGENT_TUI_LOG_FILTER` / `AGENT_TUI_LOG_STREAM_FILTER`.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/transport.rs` now redacts token-bearing WebSocket URLs before logging connection attempts, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs` now records session-selector classes (`implicit-active`, `explicit-active`, `explicit`, `blank`) instead of raw selector values in request spans.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/transport.rs` now has a log-capture regression test that proves token-bearing WebSocket URLs are redacted, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs` now captures `rpc_request` span fields directly to prove explicit session IDs are not recorded, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-common/src/common/telemetry.rs` now regression-tests per-sink filter selection.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs` now propagates the active tracing dispatcher into the dedicated WS runtime thread, emits the startup `WS server listening` diagnostic on that owning thread under `runtime.enter()`, and proves through a capture-layer regression test that the startup log remains token-free while still being observable under the caller's subscriber.

Findings:

- None after remediation.

Contextual non-applicability:

- `otel-w3c-traceparent-propagation` is not a direct fit for this tranche because the reviewed daemon/UI diagnostics slice is still local-only: the current RPC and WebSocket payloads do not carry trace context, and there is no outbound HTTP or trace-export pipeline in this surface to correlate with a wider distributed trace.

### `2026-04-13 09:47Z` `A09` Build, version, release, and dist tooling

Reviewed targets:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/Cargo.toml`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/package.json`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/npm/agent-tui-darwin-arm64/package.json`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/npm/agent-tui-darwin-x64/package.json`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/npm/agent-tui-linux-arm64/package.json`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/npm/agent-tui-linux-x64/package.json`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/build.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/build.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/build.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/xtask/Cargo.toml`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/xtask/src/main.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/justfile`

Passes:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/Cargo.toml`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/package.json`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/npm/*/package.json`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/xtask/src/main.rs` already keep the version surface unusually centralized for a mixed Rust/npm release: Rust crates inherit `workspace.package.version`, and `version_check()` plus `set_version()` enforce that the root npm package, optional platform dependencies, and per-platform package manifests all move in lockstep.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/xtask/src/main.rs` keeps artifact naming explicit and deterministic. `required_artifacts()` defines the release and npm package sets in one place, `dist_release()` refuses duplicate filenames and emits `checksums-sha256.txt`, and `dist_npm()` stages binaries into the expected `bin/agent-tui` layout for each platform package.
- `/Users/pedroproenca/Documents/Projects/agent-tui/justfile` still funnels the main operator entrypoints through opinionated tooling wrappers: `ready` delegates to `xtask ci`, `build` and `build-release` force `web-sync` first so embedded assets stay fresh, and the npm `version` script in `/Users/pedroproenca/Documents/Projects/agent-tui/cli/package.json` explicitly blocks ad hoc version bumps in favor of the documented release flow.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/build.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/build.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/build.rs` now distinguish permissive and strict build metadata modes. Non-release builds still warn when git metadata is unavailable, while `--release` builds now fail hard instead of silently embedding an unverifiable git SHA.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/xtask/src/main.rs` now routes `release()` through the existing validation helpers before tagging: it validates the requested version against the manifests, runs the full CI gate, performs a local release build, and verifies the release artifact set before creating or pushing the tag. `/Users/pedroproenca/Documents/Projects/agent-tui/justfile` now passes the artifact directory explicitly through the `release` recipe.
- `2026-04-13 remediation` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/xtask/src/main_tests.rs` adds sibling tempdir-based coverage for version mismatch detection, release-input refusal, checksum emission, and npm package staging. The `xtask` crate now has direct regression tests around the real filesystem boundary instead of relying only on manual release flows.

Findings:

- None after remediation.

Contextual non-applicability:

- `otel-w3c-traceparent-propagation` is not a direct fit for this tranche because the reviewed build and release tooling is local CLI and filesystem orchestration with no runtime request path or distributed trace boundary.
- The Ratatui-specific rules are not a direct fit here because this slice packages binaries and metadata rather than running a terminal UI event loop or render pipeline.

## Next Queue

- `(none - audit inventory exhausted)`

## Notes

- `2026-04-13 18:28Z` synchronization pass: the rule matrix cleared `89` stale `finding` cells to match the completed tranche ledger, and the audit queue remains exhausted.
- This ledger is the human-readable companion to `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-rule-matrix.tsv`.
- Use the audit-unit identifiers from `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-audit-inventory.md`.
