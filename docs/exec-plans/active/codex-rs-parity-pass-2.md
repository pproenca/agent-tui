# Codex-rs Parity Pass 2

## Purpose / Big Picture

Implement the next high-value `codex-rs` parity slice in `/Users/pedroproenca/Documents/Projects/agent-tui` by tightening attach-session terminal ownership and stream-task teardown. The visible outcome is that attach will no longer proceed after a partial terminal setup failure, and stream output teardown will cancel the underlying RPC stream on every exit path rather than only on an explicit detach-key sequence.

## Progress

- [x] (2026-04-13 11:03Z) Create the parity-pass exec plan and pin the attach/runtime scope.
- [x] (2026-04-13 11:18Z) Make terminal setup rollback atomic when entering attach TTY mode fails.
- [x] (2026-04-13 11:18Z) Give attach stream output an owned abort path so teardown cancels the stream reader on all exits.
- [x] (2026-04-13 11:30Z) Add regression tests, trim the resolved audit findings, and pass focused plus full verification.

## Surprises & Discoveries

- `2026-04-13 11:03Z` The existing audit ledger already isolated these attach-runtime gaps under `F06`, so this pass is reducing known risk rather than exploring a new area.
- `2026-04-13 11:14Z` The terminal-setup rollback path was easiest to test by splitting rollback orchestration from the concrete `crossterm` setup function, because writing escape sequences into an in-memory buffer succeeds even when no real TTY exists.
- `2026-04-13 11:30Z` Full `just ready` stayed green after the attach changes; the only CI noise remained the pre-existing `cargo-deny` duplicate warnings.

## Decision Log

- `2026-04-13 11:03Z` Prioritize attach runtime over larger daemon-lifecycle changes because attach is user-facing, easier to verify locally, and can adopt `codex-rs`-style cancellation ownership without reworking the daemon architecture.

## Outcomes & Retrospective

This pass closed two attach-runtime gaps that map well to `codex-rs` discipline:

- Attach TTY setup now behaves atomically: if alternate-screen or bracketed-paste setup fails, raw mode is disabled and terminal modes are reset before the error is returned.
- The attach output worker now owns the stream abort path, so teardown cancels the blocked RPC stream reader on PTY write failures, event-read failures, EOF, and ordinary drop paths instead of only on explicit detach.

The next highest-value parity gap in this area is still the missing panic-hook chain for attach, followed by stronger paste-burst handling and explicit surfacing of attach-triggered resize failures.

## Context and Orientation

Files expected to change in this pass:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/rpc_client.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/error.rs`

Terms used in this plan:

- `terminal setup rollback`: disabling raw mode and restoring terminal modes if attach cannot finish its initial TTY setup.
- `owned abort path`: storing the stream abort handle next to the output-worker join handle so dropping the worker also cancels the blocked stream read.
- `explicit detach path`: the current behavior where only the detach-key branch aborts the stream directly.

Relevant existing audit findings:

- `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-findings.md` documents the partial terminal setup bug and attach output teardown gap under `F06`.

## Plan of Work

### Milestone 1: Make terminal setup atomic

Goal: never continue attach TTY mode after a failed alternate-screen or bracketed-paste setup step.

Work: teach `TerminalGuard::new()` to roll back raw mode and reset terminal modes when `prepare_terminal()` fails, then return the original error.

Result: attach either enters a known-good TTY mode or fails cleanly.

Proof: unit coverage shows the rollback helper is called on setup failure and ordinary attach tests still pass.

### Milestone 2: Own attach stream cancellation

Goal: ensure the attach output worker has cancellation ownership independent of the explicit detach-key branch.

Work: store the RPC stream abort handle inside `AttachOutputWorker`, abort it during shutdown/drop, and keep the worker join bounded.

Result: PTY write failure, event-read failure, EOF, and other early exits all stop the blocked stream reader promptly instead of waiting and then detaching the thread.

Proof: regression coverage shows shutdown flips the abort handle and joins the worker.

## Concrete Steps

1. Update `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs` to add terminal-setup rollback and owned stream abort logic.
2. Keep `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/rpc_client.rs` as the source of abort handles, only widening it if the attach worker needs a more ergonomic constructor.
3. Add attach-focused tests for the new rollback/cancellation behavior.
4. Run focused `agent-tui-app` tests and then repository verification if the change surface stays clean.
5. Update this plan with completion timestamps and retrospective notes.

## Validation and Acceptance

Validation commands:

1. `cargo test -p agent-tui-app attach`
2. `cargo test -p agent-tui-app daemon::rpc_core::tests::live_preview_selector_rejects_blank_explicit_session_id`
3. `env PATH="$HOME/.rustup/toolchains/1.93.0-aarch64-apple-darwin/bin:$PATH" just ready`

Expected results:

- Attach tests cover cancellation ownership and pass.
- Existing attach/live-preview regressions remain green.
- Full repo verification still passes.

Acceptance:

- Attach TTY setup no longer leaves the terminal partially configured on failure.
- Attach output teardown aborts the RPC stream on every exit path, not just explicit detach.

## Idempotence and Recovery

- Reapplying the attach worker changes is safe because they only add cancellation ownership to an existing shutdown path.
- If terminal rollback proves too invasive for tests, keep the owned abort path and record the rollback as deferred; the two changes are independent.
- If `just ready` is blocked by local toolchain shims, rerun verification with the pinned Rust `1.93.0` toolchain path and record that in the plan and final summary.
