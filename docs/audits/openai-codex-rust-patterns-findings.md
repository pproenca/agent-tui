# OpenAI Codex Rust Findings Ledger

## Open Findings

- `[F07][types-try-from-newtype-validation]` Invalid explicit session selectors are silently coerced to "active": `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs` makes `parse_session_selector()` return `None` when `SessionId::try_new()` fails, and `parse_session_input()` then feeds that selector into `kill`, `restart`, and the other session-targeted RPCs as though the caller had asked for the active session. A malformed `session` parameter can therefore mutate the wrong session instead of being rejected at the boundary.
- `[F07][errors-struct-display-payload]` Stopped-session switch errors are flattened into a fake "not found" identifier: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/session.rs` returns `SessionError::NotFound(format!("{} (session not running)", input.session_id))` from `AttachUseCase`. The adapter and CLI layers therefore lose the ability to distinguish "missing session" from "known session that has stopped" and can only surface a preformatted string.
- `[F07][errors-boundary-error-translator]` Session lifecycle state is flattened into misleading stopped/absent outputs: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs` turns lock-timeout sessions into synthetic `running: false` / `"(locked)"` entries in `list()`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/session.rs` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs` both trust that lossy state during cleanup, and `SessionManager::kill()` removes a session from the registry before `sess.kill()` succeeds. A locked or kill-failing session can therefore be reported as stopped/cleaned or disappear from `sessions` while its process is still alive.
- `[F08][async-graceful-then-forceful-cancel]` Daemon control-plane shutdown still never escalates from graceful to forcible automatically: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/signal_handler.rs` logs the second signal as "forcing shutdown" but only reissues the same shutdown flag and notifier, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/daemon_lifecycle.rs` treats acknowledged RPC shutdown as success after a bounded socket wait and then unlinks socket and lock files if they still exist, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs` returns that result to the operator without re-checking process liveness. A wedged daemon can therefore survive a reported graceful stop while its coordination files are removed.
- `[F08][errors-boundary-error-translator]` Daemon PID and lock corruption is flattened and misclassified: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/client.rs` exposes lock-file failures as `PidLookupResult::Error(String)`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs` maps them to `ClientError::SignalFailed { pid: 0, ... }`. Status, stop, and restart errors therefore report stale local coordination metadata as a signal failure instead of a structured daemon-state problem.
- `[A05][async-graceful-then-forceful-cancel]` Bounded shutdown still detaches owner threads instead of forcefully cancelling them: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs` drains `stream_threads` and drops any `JoinHandle` that misses the deadline in `join_stream_threads`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs` does the same in `WsServerHandle::shutdown` while still deleting the WS state file. Timed-out daemon stream threads or the outer WS runtime thread can therefore outlive shutdown with no remaining owner to join or signal.
- `[A05][errors-boundary-error-translator]` Shutdown requests are acknowledged even if the wakeup path fails: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/shutdown.rs` always returns `acknowledged: true`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/shutdown_notifier.rs` makes `notify()` infallible, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs` ignores lock/write failures in `ShutdownNotify::notify`. If the wake byte is not delivered, `run_accept_loop` can stay blocked in `poll()` on an otherwise idle daemon after the caller has already been told shutdown succeeded.
- `[A04][async-bounded-vs-unbounded-channel-split]` PTY output delivery still blocks on an internal bounded event queue: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/pty.rs` uses `channel::bounded(PTY_READ_CHANNEL_CAPACITY)` for `ReadEvent`s and `spawn_reader` calls blocking `send(ReadEvent::Data(...))`. If `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs` cannot keep `pump_loop` moving because the session lock is contended or output bursts outrun processing, the reader stops draining the PTY master and the child process is backpressured by its own output.
- `[A04][defensive-io-drain-timeout-grandchildren]` PTY shutdown still has no reader-drain timeout or join path: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/pty.rs` spawns a detached `pty-reader` thread but `PtyHandle::kill` and `Drop` never join or time out that thread. When process-group signaling falls back to direct child kill, or descendants keep the slave side open, the reader can stay blocked on `read()` indefinitely and leak a stuck background thread even after the session is considered dead.
- `[A03][errors-boundary-error-translator]` Session persistence is still treated as best-effort even though startup cleanup depends on it: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs` suppresses `cleanup_stale_sessions()` failure in `SessionManager::with_max_sessions`, and `spawn`, `restart`, and `kill` only warn when `add_session` or `remove_session` fail. The CLI/RPC path can therefore report success while the JSONL store diverges from live state, leaving the next daemon startup to recover from stale or missing metadata with no operator-visible signal.
- `[F05][types-try-from-newtype-validation]` The resize path still bypasses the `TerminalSize` invariant end to end: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/commands.rs` accepts unrestricted `u16`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/params.rs` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/types.rs` model resize DTOs as raw integers, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs` forwards them without `TerminalSize::try_new`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/session.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/terminal_engine.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/pty.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/vterm.rs` accept raw sizes, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs` later masks invalid live sizes in `list()` via `TerminalSize::try_new(cols, rows).unwrap_or_default()`.
- `[F05][errors-boundary-error-translator]` Attach-triggered resize failures are silently discarded in `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs`: both the initial `terminal::size()` sync and the `Event::Resize` branch ignore `call_with_params(..., "resize", ...)` errors, so the operator gets no warning when the local terminal and remote session diverge on size.
- `[F05][testing-insta-snapshot-tui-rendering]` Resize/reflow coverage stops before rendered-screen correctness: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core.rs` only asserts that a resize event is emitted, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/render.rs` checks trimming/reset behavior with hand-built buffers, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/vterm.rs` has no resize/reflow assertions, and the relevant test manifests still omit `insta`, so wrapped content and whitespace regressions after resize are not snapshot-protected.
- `[A08][workspace-single-source-dependencies]` Internal workspace crate edges are still repeated as per-crate path dependencies instead of using `workspace = true` from `/Users/pedroproenca/Documents/Projects/agent-tui/cli/Cargo.toml`. Reviewed examples: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/Cargo.toml`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/Cargo.toml`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/Cargo.toml`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/Cargo.toml`.
- `[A08][workspace-test-support-as-member-crates]` Shared test helpers are split between `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/common/` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/test_support/` instead of a dedicated workspace member test-support crate.
- `[A01][types-try-from-newtype-validation]` Terminal size invariants are encoded in `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/session_types.rs` as `TerminalSize`, but `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/types.rs` still models spawn and resize inputs as raw `u16` pairs, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/commands.rs` still accepts raw `u16` run dimensions, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs` clamps or forwards raw values instead of constructing a validated type. This was directly revalidated in `F01`.
- `[F01][sandbox-env-clear-pre-exec]` Session startup still relies on ambient daemon environment rather than explicit request-scoped data: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs` only forwards `cwd`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/params.rs` has no `env` field, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs` hardcodes `env: None`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/pty.rs` inherits the daemon env without `env_clear` or `pre_exec` tethering.
- `[A02][errors-boundary-error-translator]` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/error.rs` translates `SessionError::Persistence { operation, reason, source }` into `DomainError::Generic { message }`, which discards the specific persistence error code and collapses structured boundary context into a string.
- `[A10][testing-path-attribute-sibling-tests]` The repository still relies on inline `#[cfg(test)] mod tests` blocks across at least `50` Rust files and currently has zero sibling `#[path = "..._tests.rs"]` stubs. Representative files: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/transport.rs`.
- `[A10][testing-insta-snapshot-tui-rendering]` Terminal-oriented smoke and E2E coverage still depends on substring and field assertions instead of stable render snapshots, and the audited test manifests do not include `insta`. Reviewed anchors: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/cli_smoke.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/system_e2e.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/Cargo.toml`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/Cargo.toml`.
- `[A10][testing-paused-runtime-advance]` Async timing tests in `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/transport.rs` still rely on wall-clock `park_timeout` and `Instant::elapsed()` bounds rather than paused-runtime or equivalent virtual-time control.

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

Findings:

- Internal workspace dependency declarations are not fully sourced through `workspace.dependencies`.
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

Findings:

- Terminal dimension validation is not threaded through the request DTOs and RPC parsing path even though the domain already owns a `TerminalSize` invariant type.

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

Findings:

- Persistence errors lose their specific code and structured payload when translated from `SessionError` into adapter-facing `DomainError`.

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

Findings:

- The workspace has not adopted sibling `#[path]` test modules, so implementation files still drown in inline test blocks and test churn remains attached to production modules in blame/history.
- Terminal rendering behavior is not snapshot-tested, which leaves layout, whitespace, ANSI, and reflow regressions under-protected.
- Time-sensitive async tests still depend on wall-clock sleeps and elapsed-time ceilings instead of deterministic virtual-time control.

Contextual non-applicability:

- The codex `wiremock` plus SSE helper rule does not map literally because this repository's main runtime boundary is Unix socket JSON-RPC plus WebSocket, not outbound HTTP SSE. The underlying "real wire, not trait mock" goal is still applicable and partly met by the existing harnesses.
- The closure-builder fixture pattern is not a strong fit for the current harness API because the scripting surface is still small and request-oriented rather than a large mutable config object.

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

Findings:

- Spawn-time environment control is still implicit and ambient rather than explicit: the current `run` and JSON-RPC spawn boundary can steer cwd but not env, so session startup behavior depends on the daemon's inherited environment.
- The `TerminalSize` validation gap recorded in `A01` remains directly visible in the `F01` flow because CLI `run` and RPC `spawn` still move raw `u16` sizes until the adapter clamps them.

Contextual non-applicability:

- The codex sandbox rules about re-exec helpers, argv[0] binary multiplexing, and three-layer network isolation are not a direct fit for `agent-tui run`, which intentionally launches the operator-requested interactive process instead of a tightly sandboxed helper binary.
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

Findings:

- The `TerminalSize` invariant from `A01` remains bypassed across the full resize/reflow path, from CLI and RPC DTOs down through the session, PTY, and virtual-terminal layers.
- Attach-triggered resize RPC failures are dropped silently, so a failed daemon-side resize can leave the operator attached to a terminal whose local and remote dimensions no longer match.
- Resize/reflow behavior still lacks full-screen snapshot coverage, leaving wrapped-text and whitespace regressions under-protected.

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

Findings:

- Persistence failures are still best-effort after the live runtime mutates: `SessionManager::with_max_sessions` suppresses `cleanup_stale_sessions()` failure, while `spawn`, `restart`, and `kill` only log `add_session` and `remove_session` failures. Operators can therefore get a successful result while the persisted session log no longer matches the live daemon state.
- The existing `TerminalSize` validation gap remains visible inside the persistence/runtime internals: `Session::new`, `SessionManager::persisted_session`, `PersistedSession`, and `TerminalState::{new, resize}` still store raw `u16` dimensions, so this tranche revalidates the broader `A01` and `F05` finding instead of closing it.

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

Findings:

- PTY output delivery still blocks on a bounded internal event channel, so a slow or lock-blocked session pump can stop draining the master PTY and backpressure the child process.
- PTY shutdown still lacks a reader-drain timeout or join path, so descendants that keep the slave open can leave a detached `pty-reader` thread blocked on `read()` indefinitely.
- The existing `TerminalSize` invariant gap remains visible inside the engine layer because `PtySession::resize`, `PtyHandle::resize`, and `VirtualTerminal::resize` still accept raw `u16` dimensions.
- The previously recorded snapshot gap remains visible at the engine layer: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/render.rs` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/vterm.rs` still rely on focused assertion tests rather than full rendered-screen snapshots.

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

Findings:

- The daemon's owner-thread shutdown paths stop at detachment: `DaemonServer::join_stream_threads` and `WsServerHandle::shutdown` bound their waits but then drop unfinished `JoinHandle`s, which means still-running stream/runtime threads can outlive daemon shutdown with no remaining owner.
- Shutdown acknowledgement is optimistic because notifier delivery failures are swallowed even though the accept loop depends on the wake byte to break out of `poll()`.
- The previously recorded wall-clock timing gap remains visible in this slice: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/server.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/ws_server.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/daemon/rpc_core.rs` still test bounded shutdown and cancellation with `park_timeout`, `recv_timeout`, and elapsed-time assertions instead of deterministic virtual-time control.

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

Findings:

- The daemon lifecycle still lacks a true graceful-then-forceful shutdown path: the second signal path only reissues the same notifier, the bounded stream-thread join gap from `A05` still applies, and the RPC stop helper can report success after unlinking daemon coordination files even if the daemon process never exited.
- PID lookup and lock-file corruption are flattened too early, then translated into the wrong operator-facing category. Invalid or unreadable lock metadata loses structured file context in `PidLookupResult::Error(String)` and is later surfaced as `ClientError::SignalFailed { pid: 0, ... }`.
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

Findings:

- Invalid explicit session selectors are silently treated as "active" because `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs` drops `SessionId::try_new()` failures inside `parse_session_selector()` and `parse_session_input()` instead of returning an RPC validation error. `kill`, `restart`, and the other selector-based lifecycle RPCs can therefore target the wrong session when given malformed ids.
- Stopped-session lifecycle errors are flattened into a display string in `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/session.rs`: `AttachUseCase` reports `SessionError::NotFound(format!("{} (session not running)", input.session_id))` rather than a structured stopped-session variant. `sessions switch` and other attach callers cannot distinguish "missing" from "stopped" or preserve structured remediation context.
- Session lifecycle state is translated too aggressively at the runtime boundary. `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session.rs` maps lock-acquisition failure in `list()` to a fake stopped `"(locked)"` row, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/session.rs` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/handlers.rs` both trust that lossy `running` bit during cleanup, and `SessionManager::kill()` drops the registry entry before `sess.kill()` succeeds. A temporarily locked or kill-failing session can therefore be shown as stopped, selected for cleanup, or disappear from `sessions` while its process is still alive.

Contextual non-applicability:

- `async-abort-on-drop-handle`, `async-child-cancellation-tokens`, and `async-shared-boxfuture-joinhandle` are not direct fits for this tranche because the lifecycle surface is primarily lock/PTY/session-manager code over std threads and shared state, not a nested tokio task tree with owned join futures.
- `testing-wiremock-sse-fakes`, `testing-insta-snapshot-tui-rendering`, and the TUI-specific rendering rules are not direct fits here because this slice is session selection and metadata mutation rather than outbound protocol fakes or rendered-screen correctness.

## Next Queue

- `F02` Snapshot and screenshot rendering
- `F03` Input injection
- `F04` Wait and assert semantics

## Notes

- This ledger is the human-readable companion to `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-rule-matrix.tsv`.
- Use the audit-unit identifiers from `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-audit-inventory.md`.
