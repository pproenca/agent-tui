# Codex-rs Timing Probe Audit

## Purpose / Big Picture

Probe the remaining real-time polling tests in `/Users/pedroproenca/Documents/Projects/agent-tui` after the main `codex-rs` remediation program completed. The visible outcome is a repo that stays green on `just ready`, `just test-core-e2e`, and `cargo deny`, while the leftover timing-based tests are either converted to deterministic synchronization or explicitly documented as OS-bound integration probes rather than silently carried as unresolved caveats.

## Progress

- [x] (2026-04-13 16:59Z) Re-establish the green base and inventory all remaining `park_timeout` / `elapsed` usage under sibling `*_tests.rs` files.
- [x] (2026-04-13 17:00Z) Classify the remaining uses into deterministic placeholders versus OS-bound integration probes.
- [x] (2026-04-13 18:34Z) Remove the low-value synchronization sleeps from the remaining unit-style tests.
- [x] (2026-04-13 18:34Z) Update the audit documentation with the surviving OS-bound probes and rerun the full green gate.

## Surprises & Discoveries

- `2026-04-13 17:00Z` The remaining timing list is small, but it is mixed: a few tests still use sleeps only as handshakes, while the rest are genuinely observing file visibility, process exit, zombie state, socket timeout, or lock backoff against the real OS.
- `2026-04-13 18:34Z` This repo's Clippy policy caught the first rewrite immediately: `std::sync::mpsc::channel` is disallowed even in tests, so the handshake conversions had to use bounded `crossbeam-channel` and, for `agent-tui`, a new test-only dependency edge.

## Decision Log

- `2026-04-13 17:00Z` Do not pretend every remaining wall-clock wait is the same problem. Pure synchronization placeholders should be removed; kernel-visible integration probes should be audited and justified rather than forced into fake virtual-time abstractions.
- `2026-04-13 18:34Z` Keep the residual timing inventory narrow and honest. `process_tests`, `session_tests`, and `lock_helpers_tests` remain because they are directly probing kernel-visible state or the real backoff algorithm, not because the repo still lacks deterministic test seams.

## Outcomes & Retrospective

This follow-up pass removed the last incidental timing placeholders from the sibling test tree without reopening the formal findings ledger.

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/common/interactive_pty_tests.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach_tests.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/pty_tests.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/client_tests.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/process_tests.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session_tests.rs`

The surviving timing-based tests are now explicitly audited in `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-timing-probe-followup.md` as OS-bound integration probes or direct lock-backoff behavior. The repo finished green again under `just ready` and `just test-core-e2e`.

## Context and Orientation

Files currently under probe:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/common/interactive_pty_tests.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach_tests.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/terminal/pty_tests.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/lock_helpers_tests.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/process_tests.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/ipc/client_tests.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra/src/infra/daemon/session_tests.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-findings.md`

Terms used in this plan:

- `deterministic placeholder`: a test sleep or polling loop that exists only to handshake between test threads and can be replaced with a channel, condvar, barrier, or direct state injection.
- `OS-bound integration probe`: a test that is intentionally waiting for the kernel or filesystem to expose a real external transition such as process exit, zombie state, file creation, or socket timeout.

## Plan of Work

### Milestone 1: Remove deterministic placeholder sleeps

Goal: eliminate the remaining sleeps that are only papering over test coordination.

Work: rewrite the remaining unit-style tests to use direct synchronization and test seams instead of ad hoc short waits.

Result: the residual timing list better reflects true integration boundaries rather than incidental test scaffolding.

Proof: the touched test files no longer rely on those placeholder waits and their focused suites pass.

### Milestone 2: Audit the true OS-bound probes

Goal: leave a precise record of which timing-based tests remain and why they are justified.

Work: document the surviving process/file/socket/lock probes in the audit ledger, distinguishing them from the already-closed async-timer findings.

Result: the repo has an explicit, narrow explanation for the remaining real-time polling instead of a vague caveat.

Proof: the ledger names the surviving files and the full green gate still passes.

## Concrete Steps

1. Re-run the repo gate (`just ready` and `just test-core-e2e`) before touching the follow-up pass.
2. Rewrite the deterministic placeholder waits in the targeted test files.
3. Run focused tests for the touched crates.
4. Update the audit ledger with the surviving OS-bound probes.
5. Re-run the full repo gate and confirm it stays green.

## Validation and Acceptance

Validation commands:

1. `cargo test -p agent-tui-app attach`
2. `cargo test -p agent-tui-infra`
3. `cargo test -p agent-tui --test cli_command_contracts`
4. `env PATH="$HOME/.rustup/toolchains/1.93.0-aarch64-apple-darwin/bin:$PATH" just ready`
5. `env PATH="$HOME/.rustup/toolchains/1.93.0-aarch64-apple-darwin/bin:$PATH" just test-core-e2e`

Expected results:

- The deterministic placeholder waits are removed from the touched tests.
- The remaining timing-based tests are explicitly documented as OS-bound probes.
- The repo remains green end to end.

## Idempotence and Recovery

- Re-running the inventory search is safe: `rg -n 'park_timeout|Instant::elapsed\\(|\\.elapsed\\(' cli/crates -g '*_tests.rs'`.
- If a supposed placeholder turns out to be guarding a real OS transition, stop rewriting it and record it as an audited integration probe instead.
- If the follow-up pass destabilizes the test suite, revert only the touched test rewrites and keep the audit classification notes.
