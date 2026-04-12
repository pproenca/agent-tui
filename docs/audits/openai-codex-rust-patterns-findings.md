# OpenAI Codex Rust Findings Ledger

## Open Findings

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

## Next Queue

- `F05` Resize and terminal reflow
- `A03` Session repository and persistence internals
- `F07` Session lifecycle management

## Notes

- This ledger is the human-readable companion to `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-rule-matrix.tsv`.
- Use the audit-unit identifiers from `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-audit-inventory.md`.
