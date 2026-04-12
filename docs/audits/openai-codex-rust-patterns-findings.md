# OpenAI Codex Rust Findings Ledger

## Open Findings

- `[A08][workspace-single-source-dependencies]` Internal workspace crate edges are still repeated as per-crate path dependencies instead of using `workspace = true` from `/Users/pedroproenca/Documents/Projects/agent-tui/cli/Cargo.toml`. Reviewed examples: `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/Cargo.toml`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/Cargo.toml`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/Cargo.toml`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/Cargo.toml`.
- `[A08][workspace-test-support-as-member-crates]` Shared test helpers are split between `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/common/` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/test_support/` instead of a dedicated workspace member test-support crate.
- `[A01][types-try-from-newtype-validation]` Terminal size invariants are encoded in `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/session_types.rs` as `TerminalSize`, but `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/domain/types.rs` still models spawn and resize inputs as raw `u16` pairs, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs` clamps or forwards raw values instead of constructing a validated type.
- `[A02][errors-boundary-error-translator]` `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/error.rs` translates `SessionError::Persistence { operation, reason, source }` into `DomainError::Generic { message }`, which discards the specific persistence error code and collapses structured boundary context into a string.

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

## Next Queue

- `A10` Test harnesses and auditability of tests
- `F01` Session spawn and initial run
- `F05` Resize and terminal reflow

## Notes

- This ledger is the human-readable companion to `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-rule-matrix.tsv`.
- Use the audit-unit identifiers from `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-audit-inventory.md`.
