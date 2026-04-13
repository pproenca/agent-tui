# Codex-rs Remediation Program

## Purpose / Big Picture

Remediate the open `openai-codex-rust-patterns` findings in `/Users/pedroproenca/Documents/Projects/agent-tui` while preserving a green repository at every step. The visible outcome is a repo that is not only policy-green under `just ready`, but also materially closer to `codex-rs` behavior across attach/runtime safety, input fidelity, shutdown discipline, snapshot correctness, protocol/docs parity, and test coverage, with slow core E2E coverage revalidated before each tranche begins.

## Progress

- [x] (2026-04-13 09:02Z) Re-run the green-base verification with `just ready` and `just test-core-e2e` before starting remediation work.
- [x] (2026-04-13 09:02Z) Create the remediation exec plan and group the open findings into green-base tranches.
- [x] (2026-04-13 09:12Z) Execute tranche 1: attach input hardening and attach-side resize error surfacing.
- [x] (2026-04-13 09:50Z) Execute tranche 2: injected-input semantic fidelity and wait/assert timing coverage.
- [x] (2026-04-13 09:50Z) Execute tranche 3: snapshot freshness/region correctness and rendered-output regression protection.
- [ ] Execute tranche 4: session lifecycle, persistence, and shutdown ownership fixes.
- [ ] Execute tranche 5: live-preview security/contract parity plus listener-level tests.
- [ ] Execute tranche 6: workspace/test-support cleanup, cargo-deny duplicate reduction, and any remaining docs/spec drift.
- [x] (2026-04-13 09:50Z) Close the standalone admin-surface gaps from `F12`: structured JSON completions, non-duplicated `live stop` errors, and standalone contract tests.
- [ ] Close the program by updating the retrospective and moving this file to `completed/`.

## Surprises & Discoveries

- `2026-04-13 09:02Z` The repository-level green base is stronger than the default CI gate: `just ready` passes and the slow ignored core E2E suite in `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/system_e2e.rs` also passes when run explicitly.
- `2026-04-13 09:02Z` `cargo-deny` is green but still noisy because the lockfile contains duplicate crate versions; this is not yet a failing policy, so it should be remediated in its own tranche rather than mixed into behavioral fixes.
- `2026-04-13 09:02Z` The earlier audit program in `/Users/pedroproenca/Documents/Projects/agent-tui/docs/exec-plans/active/openai-codex-rust-audit-program.md` remains the discovery artifact; this plan is the separate implementation artifact for closing the remaining findings.
- `2026-04-13 09:12Z` Running the full verification sequentially matters for this repo: `just ready` and `just test-core-e2e` are both green after tranche 1, but parallelizing them is noisy enough to trigger a misleading daemon-stop failure in the ignored E2E suite.
- `2026-04-13 09:50Z` On this host, `kill(pid, 0)` reports an unreaped zombie child as still alive. The daemon-stop E2E failure turned out to be a process-reaping false positive, not a missing shutdown signal, so liveness checks now need to distinguish zombie state from real running ownership.

## Decision Log

- `2026-04-13 09:02Z` Start with attach/runtime input hardening because it is directly user-facing, maps cleanly to the `codex-rs` TUI rules, and can be verified with focused unit tests plus the existing attach E2E coverage while keeping the repo green.
- `2026-04-13 09:02Z` Keep `cargo-deny` duplicate cleanup out of tranche 1 even though the user explicitly called it out, because dependency-graph churn is a wider blast radius than the attach fixes and is easier to isolate once behaviorally critical findings are reduced.
- `2026-04-13 09:12Z` Treat sequential `just ready` plus sequential `just test-core-e2e` as the required green-base contract for every remaining tranche; do not overlap the slow E2E run with other heavyweight verification.
- `2026-04-13 09:50Z` Convert `screenshot --region` from a ghost API into explicit invalid input instead of silently succeeding with the full screen. This keeps the contract honest until named regions are actually implemented.
- `2026-04-13 09:50Z` Give standalone `completions` a first-class JSON contract rather than carving it out as a text-only exception. The CLI advertises `--format json` globally, so the standalone admin surface should honor that promise.

## Outcomes & Retrospective

(fill when complete)

## Context and Orientation

Authoritative inputs for this program:

- Findings ledger: `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-findings.md`
- Audit inventory: `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-audit-inventory.md`
- Discovery plan: `/Users/pedroproenca/Documents/Projects/agent-tui/docs/exec-plans/active/openai-codex-rust-audit-program.md`
- `codex-rs` parity work already completed in `/Users/pedroproenca/Documents/Projects/agent-tui/docs/exec-plans/active/codex-rs-parity-pass-{1,2,3}.md`

Terms used in this plan:

- `green base`: a state where the repository passes `/Users/pedroproenca/Documents/Projects/agent-tui/justfile`'s `ready` workflow and the slow ignored core E2E tests invoked by `just test-core-e2e`.
- `tranche`: one bounded remediation batch that can be implemented, tested, and documented without mixing unrelated risk domains.
- `attach input hardening`: changes in `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs` that make interactive attach safer and closer to `codex-rs`, especially around paste bursts, detach handling, and resize sync visibility.
- `listener-level tests`: tests that boot real transport surfaces such as the Axum WebSocket listener rather than only in-process helpers.

Open findings grouped into implementation tranches:

- Completed tranche 1: `F03` paste-burst handling and `F05` attach-triggered resize error surfacing.
- Completed tranche 2: `F03` modifier hold/release semantic fidelity and `F04` deterministic wait/assert timing coverage.
- Completed tranche 3: `F02` snapshot region/freshness correctness.
- Remaining tranche 4: `F07`, `F08`, `A03`, `A04`, and `A05` lifecycle, persistence, PTY, and shutdown ownership findings.
- Remaining tranche 5: `F09` and `F10` live-preview security, spec parity, and transport-level tests.
- Remaining tranche 6: `A07`, `A08`, `A09`, `A10`, `A11`, `F11`, lingering protocol/doc drift, and `cargo-deny` duplicate cleanup where dependency graph surgery is required.

Files expected to change in tranche 1:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs`
- `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-findings.md`
- `/Users/pedroproenca/Documents/Projects/agent-tui/docs/exec-plans/active/codex-rs-remediation-program.md`

## Plan of Work

### Milestone 1: Lock a full green baseline

Goal: prove the repository starts from a trustworthy state before any remediation edits.

Work: run `just ready` and the slow ignored core E2E suite, and record that state in this plan.

Result: every later tranche can truthfully say it began from a green base rather than assuming prior state.

Proof: the commands listed in Validation and Acceptance pass before tranche 1 begins.

### Milestone 2: Close the attach-runtime input hardening gaps

Goal: remove the remaining attach behavior mismatches that are both user-facing and directly covered by `codex-rs` TUI guidance.

Work: update `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs` to detect unbracketed paste bursts as buffered input rather than per-character shortcut traffic, preserve input ordering around detach detection, and surface attach-side resize RPC failures instead of silently discarding them.

Result: pasted input will no longer accidentally trigger detach handling on terminals that emit rapid key bursts, and attach operators will receive visible feedback when local and remote terminal sizes diverge because a resize RPC failed.

Proof: new unit coverage exercises the state machine and error-surfacing paths, focused attach tests pass, the slow attach E2E tests remain green, and the corresponding open findings can be removed from the ledger.

### Milestone 3: Continue through the remaining findings in green-base tranches

Goal: reduce the rest of the audit backlog without losing repository stability.

Work: take each grouped tranche in order, start from a green base, implement the smallest coherent batch that materially closes findings, and re-run focused plus full verification before moving on.

Result: the remediation effort becomes resumable and auditable instead of an ad hoc series of local fixes.

Proof: this plan, the findings ledger, and the repo state move forward together after each tranche.

## Concrete Steps

1. Confirm the green base with:
   `env PATH="$HOME/.rustup/toolchains/1.93.0-aarch64-apple-darwin/bin:$PATH" just ready`
   Expected: `All checks passed!`
2. Confirm the ignored slow core E2E suite with:
   `env PATH="$HOME/.rustup/toolchains/1.93.0-aarch64-apple-darwin/bin:$PATH" just test-core-e2e`
   Expected: the three ignored `system_e2e` tests pass.
3. Implement tranche 1 in `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app/src/app/attach.rs`.
4. Run focused verification for the touched area:
   `env PATH="$HOME/.rustup/toolchains/1.93.0-aarch64-apple-darwin/bin:$PATH" cargo test -p agent-tui-app attach`
5. Re-run the full repo gate and the slow E2E gate.
6. Update `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-findings.md` and this plan before moving to tranche 2.

## Validation and Acceptance

Validation commands for tranche 1:

1. `env PATH="$HOME/.rustup/toolchains/1.93.0-aarch64-apple-darwin/bin:$PATH" cargo test -p agent-tui-app attach`
2. `env PATH="$HOME/.rustup/toolchains/1.93.0-aarch64-apple-darwin/bin:$PATH" just ready`
3. `env PATH="$HOME/.rustup/toolchains/1.93.0-aarch64-apple-darwin/bin:$PATH" just test-core-e2e`

Expected results:

- Attach-focused unit tests cover unbracketed paste buffering and resize warning behavior.
- The full repo gate remains green.
- The slow attach E2E tests still pass.

Acceptance for tranche 1:

- Interactive attach no longer interprets likely pasted character bursts as ordinary detach-detectable key-by-key input.
- Attach no longer drops resize RPC failures silently.
- The tranche starts and ends from a green base.

## Idempotence and Recovery

- Re-running the green-base commands is safe and required before each tranche.
- If a tranche introduces instability, revert only that tranche's local edits and restore the last green commit-equivalent worktree state before attempting a narrower batch.
- If a change closes only part of a finding, keep the finding in the ledger and narrow its wording rather than declaring it complete.
- This plan is the persistence layer for the remediation program; every completed tranche must update both this file and the findings ledger before the next tranche begins.
