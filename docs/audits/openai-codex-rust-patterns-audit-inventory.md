# OpenAI Codex Rust Audit Inventory

This document defines the audit universe required to get full Rust-side coverage of this repository against the `openai-codex-rust-patterns` skill.

The goal is not just "review each crate". The goal is to review every user-visible flow and every shared runtime boundary against all 10 audit categories:

1. `defensive`
2. `errors`
3. `async`
4. `sandbox`
5. `types`
6. `testing`
7. `proto`
8. `workspace`
9. `otel`
10. `tui`

Every audit unit below gets a full 10-category pass. The "primary categories" column shows where findings are most likely to cluster first.

## Coverage Perimeter

Rust coverage perimeter:

- `cli/Cargo.toml`
- `cli/clippy.toml`
- `cli/crates/*/Cargo.toml`
- `cli/crates/agent-tui/build.rs`
- `cli/crates/agent-tui-common/src/**/*.rs`
- `cli/crates/agent-tui-domain/src/**/*.rs`
- `cli/crates/agent-tui-usecases/src/**/*.rs`
- `cli/crates/agent-tui-adapters/src/**/*.rs`
- `cli/crates/agent-tui-infra/src/**/*.rs`
- `cli/crates/agent-tui-app/src/**/*.rs`
- `cli/crates/agent-tui/src/**/*.rs`
- `cli/crates/xtask/src/**/*.rs`
- `cli/crates/agent-tui/tests/**/*.rs`
- `ARCHITECTURE.md`

Rust-owned protocol and contract perimeter:

- `docs/api/openapi.yaml`
- `docs/api/asyncapi.yaml`
- `docs/api/clients/rust/src/main.rs`

Boundary-only perimeter:

- `web/src/**/*.ts`
- `web/src/index.html`
- `web/src/styles.css`

The web UI is not audited against Rust implementation rules directly, but it must still be audited as a protocol and behavior boundary for the Rust daemon and live-preview streams.

## End-to-End Feature Slices

| ID | Feature slice | What must be audited end to end | Primary categories | Main Rust areas |
|---|---|---|---|---|
| `F01` | Session spawn and initial run | `agent-tui run`, command parsing, cwd/env propagation, PTY creation, session id creation, session limit enforcement, active-session behavior | `defensive`, `errors`, `sandbox`, `types`, `testing` | `agent-tui-app::{commands,handlers}`, `agent-tui-adapters::daemon::handlers::session`, `agent-tui-usecases::session`, `agent-tui-infra::daemon::{session,pty_session,repository}`, `agent-tui-infra::terminal::pty`, `agent-tui-domain::{types,session_types}` |
| `F02` | Snapshot and screenshot rendering | `agent-tui screenshot`, raw screen capture, rendered ANSI output, cursor inclusion, region/filter behavior, stale-screen update behavior | `defensive`, `errors`, `types`, `testing`, `tui` | `agent-tui-app::handlers`, `agent-tui-adapters::daemon::handlers::snapshot`, `agent-tui-usecases::snapshot`, `agent-tui-infra::terminal::{render,vterm}`, `agent-tui-domain::core` |
| `F03` | Input injection | `press`, `type`, `scroll`, `keydown`, `keyup`, raw `pty_write`, modifier handling, escape-sequence correctness, partial-write behavior | `defensive`, `errors`, `async`, `testing`, `tui` | `agent-tui-app::handlers`, `agent-tui-adapters::daemon::handlers::{input,diagnostics}`, `agent-tui-usecases::{input,diagnostics}`, `agent-tui-infra::terminal::pty`, `agent-tui-app::attach` |
| `F04` | Wait and assert semantics | `wait`, `assert`, text detection, stability detection, timeout behavior, stream-driven polling, active-session resolution | `defensive`, `errors`, `async`, `types`, `testing` | `agent-tui-app::{commands,handlers}`, `agent-tui-adapters::daemon::handlers::{wait,session}`, `agent-tui-usecases::{wait,wait_condition,session}`, `agent-tui-usecases::ports::session_repository`, `agent-tui-domain::types` |
| `F05` | Resize and terminal reflow | CLI resize, attach-triggered resize, live-preview resize RPC, terminal size validation, screen reflow safety | `defensive`, `errors`, `types`, `testing`, `tui` | `agent-tui-app::handlers`, `agent-tui-app::attach`, `agent-tui-usecases::session`, `agent-tui-infra::terminal::{pty,vterm}`, `agent-tui-domain::session_types` |
| `F06` | Interactive attach session | `sessions attach`, raw-mode entry/exit, detach-key parsing, attach stream setup, terminal restoration on panic/error, output draining | `defensive`, `errors`, `async`, `testing`, `tui` | `agent-tui-app::attach`, `agent-tui-app::rpc_client`, `agent-tui-app::error`, `agent-tui-app::daemon::rpc_core`, `agent-tui-infra::terminal` |
| `F07` | Session lifecycle management | `sessions`, `sessions show`, `sessions switch`, `restart`, `kill`, `cleanup`, active-session mutation, stopped-session handling | `defensive`, `errors`, `async`, `types`, `testing` | `agent-tui-app::handlers`, `agent-tui-adapters::daemon::{router,handlers::session}`, `agent-tui-usecases::session`, `agent-tui-infra::daemon::{repository,session}` |
| `F08` | Daemon lifecycle control plane | `daemon start/run/stop/status/restart`, foreground/background behavior, pid/lock coordination, graceful shutdown, force-stop path, stale state cleanup | `defensive`, `errors`, `async`, `sandbox`, `testing` | `agent-tui-app::handlers`, `agent-tui-app::daemon::{mod,server}`, `agent-tui-infra::ipc::daemon_lifecycle`, `agent-tui-infra::daemon::{config,file_lock,lock_helpers,signal_handler}` |
| `F09` | Live preview control plane | `live start/status/stop`, UI URL resolution, ws state file, ui state file, browser launch, external UI override behavior | `defensive`, `errors`, `sandbox`, `proto`, `testing` | `agent-tui-app::handlers`, `agent-tui-app::daemon::ws_server`, `agent-tui-infra::ipc::process`, `docs/api/openapi.yaml` |
| `F10` | Live preview data plane | HTTP routes, WebSocket upgrade, auth token/query shaping, `live_preview_stream`, `flightdeck_stream`, heartbeats, dropped bytes, stream close semantics | `defensive`, `errors`, `async`, `proto`, `testing`, `tui` | `agent-tui-app::daemon::{ws_server,rpc_core,server,transport}`, `agent-tui-adapters::rpc`, `agent-tui-usecases::ports::session_repository`, `docs/api/{openapi,asyncapi}.yaml` |
| `F11` | IPC transport and client/server RPC | Unix socket transport, WebSocket transport, autostart, connect/read/write timeouts, parse failures, client retryability, request/response framing | `defensive`, `errors`, `async`, `proto`, `testing` | `agent-tui-infra::ipc::{client,transport,socket,error,polling}`, `agent-tui-app::rpc_client`, `agent-tui-app::daemon::{server,transport}` |
| `F12` | CLI admin and operator UX | `version`, `env`, `completions`, prompt/confirmation flows, `--json` and text presenter behavior, exit-code discipline, shell detection/install paths | `defensive`, `errors`, `types`, `workspace`, `testing` | `agent-tui-app::{mod,commands,handlers,error}`, `agent-tui-adapters::presenter`, `agent-tui-common::{color,daemon_error,error_codes}` |

## Shared Runtime and Architecture Areas

| ID | Shared area | What must be audited | Primary categories | Main Rust areas |
|---|---|---|---|---|
| `A01` | Domain invariants and semantic types | `SessionId`, `TerminalSize`, wait/assert condition enums, screen/style model, parse-time validation, forward-compat enum design | `defensive`, `errors`, `types`, `proto`, `testing` | `agent-tui-domain::{session_types,types,core}` |
| `A02` | Error taxonomy and translation boundaries | domain errors, use-case errors, spawn classification, CLI error envelopes, transport errors, WS errors, exit-code mapping | `defensive`, `errors`, `proto`, `testing` | `agent-tui-common::{daemon_error,error_codes}`, `agent-tui-usecases::{spawn_error,ports::errors}`, `agent-tui-adapters::daemon::error`, `agent-tui-infra::{ipc::error,terminal::error}`, `agent-tui-app::error`, `agent-tui-app::handlers` |
| `A03` | Session repository and persistence internals | session metadata storage, active-session tracking, stale-session cleanup, restart semantics, persistence failure handling, state-file corruption behavior | `defensive`, `errors`, `async`, `types`, `testing` | `agent-tui-infra::daemon::{repository,session,terminal_state,system_clock}`, `agent-tui-usecases::ports::session_repository` |
| `A04` | PTY and virtual terminal engine | PTY spawn/read/write/resize behavior, render correctness, output buffering, cursor semantics, closed-stream behavior, terminal parser robustness | `defensive`, `errors`, `async`, `testing`, `tui` | `agent-tui-infra::terminal::{pty,render,vterm}`, `agent-tui-domain::core`, `agent-tui-infra::daemon::pty_session` |
| `A05` | Concurrency, shutdown, and thread/task ownership | worker thread pools, stream threads, cancellation paths, idle timeouts, connection counting, shutdown notification, signal wakeups | `defensive`, `errors`, `async`, `testing` | `agent-tui-app::daemon::{server,rpc_core,ws_server}`, `agent-tui-usecases::shutdown`, `agent-tui-usecases::ports::shutdown_notifier`, `agent-tui-common::sync` |
| `A06` | Process control and isolation boundary | background daemon spawn, process lookup, signals, browser subprocesses, environment inheritance, remote-bind safety, Unix-only assumptions | `defensive`, `errors`, `sandbox`, `testing`, `workspace` | `agent-tui-infra::ipc::{process,daemon_lifecycle,transport}`, `agent-tui-app::handlers`, `agent-tui-app::daemon::ws_server`, `cli/crates/agent-tui/build.rs` |
| `A07` | Observability and runtime diagnostics | tracing init, log sinks, env-driven log config, warning/error signal quality, cardinality and privacy of emitted fields | `defensive`, `errors`, `otel`, `testing` | `agent-tui-common::telemetry`, `agent-tui-app::daemon::{server,ws_server}`, `agent-tui-infra::ipc::transport`, `agent-tui-app::handlers` |
| `A08` | Workspace architecture and crate graph | dependency flow, facade-shell enforcement, shared dependency pinning, lint policy, Unix boundary enforcement, doc/spec drift | `defensive`, `workspace`, `testing` | `cli/Cargo.toml`, `ARCHITECTURE.md`, `cli/crates/*/src/lib.rs`, `cli/crates/agent-tui/tests/architecture.rs`, `cli/docs/architecture/*` |
| `A09` | Build, version, release, and dist tooling | build metadata injection, version sync, release bump flow, package artifact verification, npm/release packaging correctness | `defensive`, `errors`, `workspace`, `testing` | `cli/crates/agent-tui/build.rs`, `cli/crates/xtask/src/main.rs` |
| `A10` | Test harnesses and auditability of tests | mock daemon fidelity, real daemon harness, interactive PTY harness, contract test coverage, E2E realism, timing determinism, missing snapshot/wiremock gaps | `defensive`, `errors`, `async`, `testing`, `tui` | `cli/crates/agent-tui/tests/{common,cli_*,system_e2e}.rs`, unit tests across domain/usecases/common |
| `A11` | Rust-to-web contract boundary | session list payloads, flightdeck stream payloads, resize RPC contract, auto-connect semantics, UI assumptions that Rust must preserve | `errors`, `proto`, `testing`, `tui` | `docs/api/{openapi,asyncapi}.yaml`, `agent-tui-app::daemon::{ws_server,rpc_core}`, `web/src/{app.ts,session_view_model.ts}` |

## Audit Execution Rule

To claim full coverage, each audit unit above must be checked against these questions:

- `defensive`: Are all failure paths bounded, explicit, and panic-free? Are unsafe fallbacks or state-recovery paths present where needed?
- `errors`: Are errors structured, translated once at boundaries, and specific enough for operators and callers?
- `async`: Are shutdown, cancellation, timeouts, and cross-thread ownership deterministic and leak-free?
- `sandbox`: Are subprocess, env, cwd, signal, socket, and remote-bind behaviors constrained and explicit enough for this tool's threat model?
- `types`: Are invariants enforced by types instead of call-site discipline? Are public enums and wire values forward-compatible?
- `testing`: Does the code have the right kind of test, not just any test? Are race, timeout, PTY, and stream behaviors exercised realistically?
- `proto`: Are JSON-RPC, HTTP, and WebSocket contracts versionable, stable, and accurately translated?
- `workspace`: Does the code live in the right crate and preserve forward-only dependencies and workspace policy?
- `otel`: Are logs and traces useful, low-noise, and safe?
- `tui`: Does terminal behavior remain correct under redraws, resizes, attach/detach, ANSI rendering, and streaming?

## Suggested Audit Order

If this is going to be run as a large review program, the highest-yield order is:

1. `A08`, `A01`, `A02` first, because architecture drift, weak invariants, and bad error boundaries pollute every feature slice.
2. `F01` through `F08` next, because they cover the core CLI and daemon runtime.
3. `F09` through `F11` after that, because live preview and transport bugs sit on top of the core session model.
4. `A03` through `A07` next, because they validate the shared runtime guarantees.
5. `A09`, `A10`, and `A11` last, because they close the gaps around release quality, test realism, and UI contract drift.

## Completion Criteria

The audit is only complete when:

- every Rust file in the coverage perimeter has been assigned to at least one audit unit above
- every audit unit has received an explicit pass across all 10 categories
- every protocol doc and Rust/web boundary contract has been checked against the corresponding runtime code
- every identified gap is recorded as either fixed, accepted, or deferred with a reason
