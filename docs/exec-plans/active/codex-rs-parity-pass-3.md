# Codex-rs Parity Pass 3

## Purpose / Big Picture

Implement the next high-value `codex-rs` parity slice in `/Users/pedroproenca/Documents/Projects/agent-tui` by adding a chained panic-hook guard to interactive attach. The visible outcome is that a panic during attach restores raw mode and the alternate screen before the previous panic hook renders output, which avoids wedged terminals and matches the `codex-rs` `Drop guard + chained panic hook` pattern more closely.

## Progress

- [x] (2026-04-13 11:42Z) Re-run `just ready` and confirm the repository is green before starting this pass.
- [x] (2026-04-13 11:43Z) Create the parity-pass exec plan and pin the panic-hook scope.
- [x] (2026-04-13 11:55Z) Add a chained attach panic-hook guard that restores the terminal before delegating to the previous hook.
- [x] (2026-04-13 12:07Z) Add regression coverage for hook chaining/restoration, clear the resolved audit finding, and pass full verification.

## Surprises & Discoveries

- `2026-04-13 11:42Z` The baseline repo gate remained green before any further changes; the only recurring noise is the known `cargo-deny` duplicate warnings.
- `2026-04-13 11:58Z` The hook tests emit terminal reset escape sequences during intentional panic-path coverage because the new hook correctly restores terminal modes before chaining.
- `2026-04-13 12:07Z` Full `just ready` stayed green after the panic-hook change; no new policy or test regressions appeared.

## Decision Log

- `2026-04-13 11:43Z` Take the panic-hook pass ahead of paste-burst handling because it is the remaining `F06` gap directly called out by the audit and it hardens the attach runtime without changing input semantics.

## Outcomes & Retrospective

This pass closed the remaining attach panic-safety gap from the audit:

- TTY attach now installs a chained panic hook that restores raw mode and alternate-screen state before delegating to the previously installed hook.
- The attach-scoped hook is restored on the normal teardown path, so the global panic hook returns to its prior state once attach exits cleanly.

The highest-value remaining attach parity gaps are now paste-burst handling for terminals that do not emit explicit paste events and surfacing attach-triggered resize RPC failures instead of silently discarding them.

## Context and Orientation

Files expected to change in this pass:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-findings.md`

Terms used in this plan:

- `chained panic hook`: a hook that restores terminal state and then delegates to the previously installed hook instead of replacing it outright.
- `hook restoration`: returning the global panic hook to its pre-attach state on the normal non-panicking teardown path.

Relevant existing audit findings:

- `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-findings.md` documents the missing panic-hook half of the attach terminal-restore pattern under `F06`.

## Plan of Work

### Milestone 1: Add attach panic-hook chaining

Goal: restore terminal state before panic output is rendered during interactive attach.

Work: install a panic hook when entering TTY attach, restore terminal state silently inside that hook, and then call the previously installed hook.

Result: attach no longer leaves panic output trapped inside raw mode or the alternate screen.

Proof: unit coverage confirms the chained hook delegates to the previous hook and ordinary attach tests remain green.

### Milestone 2: Restore hooks on the normal path

Goal: keep the hook change scoped to attach rather than mutating global process state permanently.

Work: add a small guard that restores the previous panic hook on normal teardown and verify it through tests that run under a serialized panic-hook lock.

Result: attach gets codex-style panic safety without leaking a custom hook into later process work.

Proof: tests confirm the previous hook is called both while the guard is active and after explicit restoration.

## Concrete Steps

1. Extend `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs` with a panic-hook guard and shared restore helper.
2. Add attach-focused tests that serialize panic-hook mutations and verify chaining/restoration behavior.
3. Run focused `agent-tui-app` attach tests, then `just ready`.
4. Update this plan and the audit ledger once the pass is complete.

## Validation and Acceptance

Validation commands:

1. `cargo test -p agent-tui-app attach`
2. `env PATH="$HOME/.rustup/toolchains/1.93.0-aarch64-apple-darwin/bin:$PATH" just ready`

Expected results:

- The new attach panic-hook tests pass reliably.
- Existing attach tests remain green.
- Full repo verification passes end to end.

Acceptance:

- Panic output during interactive attach restores terminal modes before chaining to the previous hook.
- The previous panic hook is restored on the normal non-panicking teardown path.

## Idempotence and Recovery

- Reapplying the hook guard is safe because it only wraps the existing attach terminal lifecycle.
- If hook restoration proves too brittle, keep the chained restore hook and record normal-path restoration as deferred; the terminal-restore safety improvement still stands on its own.
- If verification is blocked by local toolchain shims, rerun with the pinned Rust `1.93.0` toolchain path and record that in the plan and final summary.
