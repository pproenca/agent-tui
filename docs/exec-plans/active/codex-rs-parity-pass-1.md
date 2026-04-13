# Codex-rs Parity Pass 1

## Purpose / Big Picture

Implement the first high-value `codex-rs` parity pass in `/Users/pedroproenca/Documents/Projects/agent-tui` by tightening request-boundary validation and error semantics in the daemon RPC surface. The visible outcome is safer behavior for explicit session-targeted commands: malformed `session` selectors are rejected instead of silently hitting the active session, stopped-session attach failures preserve their true state instead of collapsing into "not found", and wait/assert no longer performs an optimistic duplicate refresh that can mask stale reads.

## Progress

- [x] (2026-04-13 09:52Z) Create the parity-pass exec plan and pin the first implementation scope.
- [x] (2026-04-13 10:31Z) Tighten RPC/session selector parsing so malformed explicit session ids fail at the boundary.
- [x] (2026-04-13 10:31Z) Replace the stopped-session string flattening with a structured session error variant and propagate it through adapter presentation.
- [x] (2026-04-13 10:46Z) Remove duplicate wait refreshes, add regression coverage, and run targeted and full verification.

## Surprises & Discoveries

- `2026-04-13 09:52Z` The repository still does not contain `/Users/pedroproenca/Documents/Projects/agent-tui/docs/PLANS.md`, so this plan follows the `exec-plan` skill structure directly.
- `2026-04-13 09:52Z` The open audit program at `/Users/pedroproenca/Documents/Projects/agent-tui/docs/exec-plans/active/openai-codex-rust-audit-program.md` already identified these same gaps, so this plan is acting on existing findings rather than rediscovering them.
- `2026-04-13 10:31Z` Tightening selector parsing touched more entry points than expected because the same permissive helper was shared across spawn, wait/assert, resize, keyboard, terminal write, and live preview request parsing.
- `2026-04-13 10:46Z` Full `just ready` passed under the pinned `1.93.0` toolchain without needing any follow-on cleanup beyond `cargo fmt`.

## Decision Log

- `2026-04-13 09:52Z` Scope this first parity pass to request-boundary and error-semantics fixes because they are broadly applicable, low-risk, and directly improve safety for session-targeted operations.
- `2026-04-13 09:52Z` Defer deeper task/thread ownership changes from the audit until a later pass because they touch more runtime architecture and require a larger validation surface.

## Outcomes & Retrospective

This pass delivered three concrete `codex-rs`-aligned improvements that fit this repository well:

- Explicit blank or whitespace `session` selectors now fail fast with `-32602 Invalid params` instead of silently targeting the active session.
- Attaching to a known but stopped session now preserves that state as a structured error instead of disguising it as a synthetic `NotFound("...session not running")` string.
- Wait/assert now refresh session state exactly once per polling iteration, which removes hidden duplicated reads and makes the refresh boundary explicit.

The highest-value remaining parity work is still deeper runtime discipline rather than more TOML: cancellation ownership, broader observability, and richer protocol forward-compatibility patterns.

## Context and Orientation

Files expected to change in this pass:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/error.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/errors.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/session.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/wait.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/wait_condition.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/test_support/mock_error.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/test_support/mock_repository.rs`

Terms used in this plan:

- `explicit session selector`: a caller-provided `session` parameter in CLI or JSON-RPC input, as opposed to the implicit "active session" default.
- `boundary validation`: rejecting malformed transport input before it reaches a use case or repository call.
- `stopped-session error`: the case where a known session id resolves successfully but the underlying session is no longer running.
- `duplicate refresh`: the current behavior where wait does `session.update()?` in the polling loop and then `check_condition()` immediately calls `session.update()` again and discards the result.

Relevant existing audit findings:

- `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-findings.md` documents the malformed-selector issue under `F07`, `F03`, and `F04`.
- The same findings ledger documents the stopped-session string flattening under `F07`.
- The same findings ledger documents the duplicate wait refresh under `F04`.

## Plan of Work

### Milestone 1: Fix explicit selector validation

Goal: make malformed explicit `session` parameters fail at the RPC boundary instead of being treated as `None`.

Work: split selector parsing into an explicit `Result<Option<SessionId>, RpcResponse>` path, update all session-targeted RPC parsers to use it, and add regression tests for malformed ids across representative command families.

Result: bad selectors become `-32602 Invalid params` responses and can no longer target the wrong live session.

Proof: parser tests assert invalid explicit selectors return an error response instead of `None`.

### Milestone 2: Preserve stopped-session semantics

Goal: stop flattening "known but not running" into a `NotFound(String)` payload.

Work: add a structured `SessionError` variant for stopped sessions, return it from attach-related use cases, and teach adapter presentation to surface the correct context and suggestion.

Result: callers can distinguish "missing", "no active session", and "stopped session" without parsing strings.

Proof: use-case and adapter tests assert the correct variant, code, context, and user-facing message.

### Milestone 3: Remove optimistic duplicate wait refresh

Goal: ensure wait/assert refresh once per poll iteration and never silently discard a refresh result.

Work: remove the hidden `session.update()` from `check_condition()`, keep refresh ownership in `WaitUseCase`, and add regression coverage for text and stable checks.

Result: wait semantics become simpler and consistent with `codex-rs` boundary discipline.

Proof: tests still pass, and no `session.update()` call remains inside `check_condition()`.

## Concrete Steps

1. Update `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/rpc/mod.rs` to add strict explicit-selector parsing helpers and route all selector-based parsers through them.
2. Update `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/errors.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/session.rs`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters/src/adapters/daemon/error.rs` to preserve stopped-session semantics as a structured error.
3. Update `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/wait_condition.rs` and `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/wait.rs` to remove duplicate refresh behavior and expand tests.
4. Run targeted Rust tests for adapters and use cases, then run `just ready`.
5. Update this plan with completed timestamps, discoveries, and retrospective notes.

## Validation and Acceptance

Validation commands:

1. `cargo test -p agent-tui-adapters`
2. `cargo test -p agent-tui-usecases`
3. `env PATH="$HOME/.rustup/toolchains/1.93.0-aarch64-apple-darwin/bin:$PATH" just ready`

Expected results:

- Selector parser tests cover invalid explicit ids and pass.
- Attach/use-case tests distinguish stopped sessions from missing sessions.
- Wait-condition tests pass with no hidden `update()` call inside the condition evaluator.
- Full repo verification passes under the pinned toolchain.

Acceptance:

- Explicit malformed session selectors no longer mutate or read the active session by accident.
- Attach/session-switch callers receive a structured stopped-session error.
- Wait/assert refresh only once per poll iteration.

## Idempotence and Recovery

- Reapplying the parser change is safe because the helper semantics are deterministic and unit-tested.
- If the stopped-session variant proves too disruptive at the adapter boundary, revert just that variant and keep the selector-validation and wait-refresh changes; they are independent.
- If full `just ready` is blocked by the local `mise` shim again, rerun verification with the direct Rust `1.93.0` toolchain path and record that fact in the plan and final summary.
