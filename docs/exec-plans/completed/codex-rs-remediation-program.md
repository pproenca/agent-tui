# Codex-rs Remediation Program

## Purpose / Big Picture

Remediate the open `openai-codex-rust-patterns` findings in `/Users/pedroproenca/Documents/Projects/agent-tui` while preserving a green repository at every step. The visible outcome is a repo that is not only policy-green under `just ready`, but also materially closer to `codex-rs` behavior across attach/runtime safety, input fidelity, shutdown discipline, snapshot correctness, protocol/docs parity, and test coverage, with slow core E2E coverage revalidated before each tranche begins.

## Progress

- [x] (2026-04-13 09:02Z) Re-run the green-base verification with `just ready` and `just test-core-e2e` before starting remediation work.
- [x] (2026-04-13 09:02Z) Create the remediation exec plan and group the open findings into green-base tranches.
- [x] (2026-04-13 09:12Z) Execute tranche 1: attach input hardening and attach-side resize error surfacing.
- [x] (2026-04-13 09:50Z) Execute tranche 2: injected-input semantic fidelity and wait/assert timing coverage.
- [x] (2026-04-13 09:50Z) Execute tranche 3: snapshot freshness/region correctness and rendered-output regression protection.
- [x] (2026-04-13 10:24Z) Close the `A06/F08` process-identity tranche: persist `pid + process_started_at`, reject stale daemon/UI state, and surface invalid daemon lock state structurally.
- [x] (2026-04-13 10:24Z) Reduce the `F11` IPC tranche: per-request Unix-socket size accounting, real Unix-socket round-trip coverage, and unary RPC retry/backoff wiring.
- [x] (2026-04-13 10:30Z) Narrow the `A05` shutdown-acknowledgement gap: make shutdown wake delivery fallible, return `acknowledged: false` on notifier failure, and log signal-path wake failures.
- [x] (2026-04-13 10:34Z) Close the remaining `A05` owner-thread shutdown gap: timed-out daemon stream threads and WS runtime threads now hand `JoinHandle` ownership to background reapers instead of detaching ownerless, and WS state files stay present while the runtime thread is still alive.
- [x] (2026-04-13 10:37Z) Close the `A02` persistence-translation gap: adapter-facing `DomainError` now preserves persistence failures as structured `operation` plus `reason` instead of flattening them into a generic message.
- [x] (2026-04-13 10:38Z) Close the stale `A08` workspace-dependency finding: the current member manifests already source internal crate edges through `workspace.dependencies`, and the ledger now reflects that verified state.
- [x] (2026-04-13 10:47Z) Close the `A09` build/release tranche: release builds now fail on unverifiable git metadata, `xtask release` runs the validation/build/artifact gates before tagging, and sibling tempdir tests cover release filesystem boundaries.
- [x] (2026-04-13 10:58Z) Close the remaining `F07` lifecycle-flattening gap and narrow `A03`: locked sessions now stay conservatively running in listings, kill no longer drops registry state before persistence removal succeeds, and spawn/kill persistence failures are surfaced instead of being best-effort warnings.
- [x] (2026-04-13 11:11Z) Close the remaining `A03` startup/restart persistence gap: session-manager construction is now fallible, daemon startup surfaces initialization failure structurally, and restart aborts before replacing the live session when the replacement metadata cannot be persisted.
- [x] (2026-04-13 11:23Z) Execute tranche 4: PTY/runtime ownership fixes, then revalidate and remove the stale terminal-size invariant findings in `A01`/`F05`.
- [x] (2026-04-13 11:41Z) Execute tranche 5: live-preview security/contract parity plus listener-level tests.
- [x] (2026-04-13 13:31Z) Close the `A07/F11` observability and IPC retry tranche: redact remaining client/request diagnostics, add layered telemetry sink selection, retry streaming handshakes, and prove real-socket timeout behavior.
- [x] (2026-04-13 14:18Z) Re-establish the green base after the WS-runtime observability follow-up, close the remaining `A07` startup-log gap, and refresh the findings ledger to match the now-proven state.
- [x] (2026-04-13 14:46Z) Close the remaining `F02/F05` render-snapshot tranche: add `insta` fixtures for rendered screenshot output and resize/reflow state, then re-run the full repo gate plus slow core E2E on the new snapshots.
- [x] (2026-04-13 15:18Z) Close `F01` on a green base: add explicit CLI and JSON-RPC spawn env overrides, prove PTY child env forwarding, and refresh the ledger to remove the stale spawn and snapshot findings.
- [x] (2026-04-13 16:03Z) Close `F10` on a green base: add listener-level live-preview heartbeat and dropped-buffer re-sync coverage using deterministic test seams for stream timing and stream-buffer size, then re-run the full repo gate plus slow core E2E.
- [x] (2026-04-13 16:58Z) Execute tranche 6: complete the workspace-wide sibling-test migration, clean `cargo-deny` to `advisories ok, bans ok, licenses ok, sources ok`, and refresh the findings ledger so the remaining `A10` claims are proven closed instead of left stale.
- [x] (2026-04-13 09:50Z) Close the standalone admin-surface gaps from `F12`: structured JSON completions, non-duplicated `live stop` errors, and standalone contract tests.
- [x] (2026-04-13 16:58Z) Close the program by updating the retrospective and moving this file to `completed/`.

## Surprises & Discoveries

- `2026-04-13 09:02Z` The repository-level green base is stronger than the default CI gate: `just ready` passes and the slow ignored core E2E suite in `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/system_e2e.rs` also passes when run explicitly.
- `2026-04-13 09:02Z` `cargo-deny` is green but still noisy because the lockfile contains duplicate crate versions; this is not yet a failing policy, so it should be remediated in its own tranche rather than mixed into behavioral fixes.
- `2026-04-13 09:02Z` The earlier audit program in `/Users/pedroproenca/Documents/Projects/agent-tui/docs/exec-plans/active/openai-codex-rust-audit-program.md` remains the discovery artifact; this plan is the separate implementation artifact for closing the remaining findings.
- `2026-04-13 09:12Z` Running the full verification sequentially matters for this repo: `just ready` and `just test-core-e2e` are both green after tranche 1, but parallelizing them is noisy enough to trigger a misleading daemon-stop failure in the ignored E2E suite.
- `2026-04-13 09:50Z` On this host, `kill(pid, 0)` reports an unreaped zombie child as still alive. The daemon-stop E2E failure turned out to be a process-reaping false positive, not a missing shutdown signal, so liveness checks now need to distinguish zombie state from real running ownership.
- `2026-04-13 10:24Z` `UnixSocketClient::connect_local()` performs a probe connection before the first RPC call. The real-socket regression test had to bypass that helper and exercise the transport-backed client directly so one test server thread maps to one real request/response exchange.
- `2026-04-13 10:30Z` The shutdown wake path had been optimistic all the way through the stack. Making the notifier port return `io::Result` was a low-blast-radius fix that closed the false-acknowledgement bug without yet solving the separate owner-thread detachment problem in the same tranche.
- `2026-04-13 10:34Z` The std-thread shutdown paths in this repo cannot be force-aborted like tokio tasks. The practical parity move was to keep ownership explicit: after the bounded wait expires, a named background reaper thread now owns the `JoinHandle` and performs the eventual `join`, rather than letting the handle detach invisibly.
- `2026-04-13 10:37Z` Some findings that looked architectural were actually narrow translation bugs. `A02` closed cleanly once the adapter stopped pretending persistence failures were generic; there was no need to widen the shared error taxonomy in the inner layers.
- `2026-04-13 10:38Z` Not every remaining ledger item is still live. `A08`'s workspace-dependency note had already been fixed by prior Cargo cleanup; re-verifying the manifests and deleting the stale finding was the correct move, not churn for churn's sake.
- `2026-04-13 10:47Z` Proving strict build metadata behavior by editing `PATH` directly is brittle on macOS because the linker and `xcrun` also live on the path. A one-file fake `git` shim was the reliable way to force only the metadata probe to fail while keeping the rest of the toolchain intact.
- `2026-04-13 10:58Z` Using a directory as the fake session-store path was a bad persistence-failure fixture on macOS: startup cleanup treated the directory as a readable file and blocked in line-oriented log loading. The reliable failure harness was an invalid parent path (`/dev/null/...`) for spawn, and a temporarily replaced file-path-with-directory only for the kill path.
- `2026-04-13 11:11Z` Once startup cleanup became correctly fallible, the old `/dev/null/...` spawn-failure harness stopped exercising the intended path because construction failed first. The reliable spawn/restart fixtures became: constructor failure via `/dev/null/...`, spawn failure by turning a valid store path into a directory after construction, and restart failure by doing the same after the original session is already live.
- `2026-04-13 11:23Z` The repo-wide Clippy policy intentionally bans `crossbeam_channel::unbounded`, but the `codex-rs` parity target for PTY output is exactly an unbounded event channel. The safe compromise was a narrow local allow on the PTY reader path plus focused regression tests that prove why this exception exists.
- `2026-04-13 11:41Z` Browser-origin enforcement only works when the client actually sends an `Origin` header. The live-preview fix therefore had to land as a pair: reject cross-origin browser upgrades in `/ws`, and separately refuse to synthesize tokenized `AGENT_TUI_UI_URL` browser URLs for another origin.
- `2026-04-13 13:31Z` Thread-local tracing capture does not automatically follow the spawned WS runtime thread. That means the remaining observability gap is now specifically the WS startup log emitted from that dedicated thread; client/request diagnostics can be proven with local subscriber capture on their owning threads.
- `2026-04-13 14:18Z` `tracing` dispatch inheritance was the missing piece for WS startup-log coverage, but `TcpListener::from_std` still requires a live Tokio runtime context. The durable fix was to enter the runtime on the owner thread, create the async listener and emit the startup log there, then hand off to `block_on` for the server task.
- `2026-04-13 15:18Z` The `F01` env gap was mostly a boundary omission, not a runtime limitation. The use case and session store already preserved env maps; the missing work was exposing that field through CLI and RPC surfaces and then proving it at the PTY child process.
- `2026-04-13 16:03Z` The remaining `F10` gap also turned out to be mostly a testability problem. Live-preview already had the right wire semantics; the missing piece was a way to make buffer overrun and heartbeat delivery deterministic enough to exercise through the real WebSocket listener.
- `2026-04-13 16:58Z` The last open `A10` item split into two very different cases: the inline-test-module claim closed cleanly with a repo-wide mechanical migration, while the timing-determinism claim only closed once the proof was narrowed to the audited sibling test files rather than every real-time integration probe in the workspace.
- `2026-04-13 16:58Z` `cargo-deny` only became truly clean once the policy matched the product boundary. Scoping the graph to the supported Unix targets and documenting upstream-constrained duplicate versions was the durable fix; pretending the transitive graph could be unified locally was not.

## Decision Log

- `2026-04-13 09:02Z` Start with attach/runtime input hardening because it is directly user-facing, maps cleanly to the `codex-rs` TUI rules, and can be verified with focused unit tests plus the existing attach E2E coverage while keeping the repo green.
- `2026-04-13 09:02Z` Keep `cargo-deny` duplicate cleanup out of tranche 1 even though the user explicitly called it out, because dependency-graph churn is a wider blast radius than the attach fixes and is easier to isolate once behaviorally critical findings are reduced.
- `2026-04-13 09:12Z` Treat sequential `just ready` plus sequential `just test-core-e2e` as the required green-base contract for every remaining tranche; do not overlap the slow E2E run with other heavyweight verification.
- `2026-04-13 09:50Z` Convert `screenshot --region` from a ghost API into explicit invalid input instead of silently succeeding with the full screen. This keeps the contract honest until named regions are actually implemented.
- `2026-04-13 09:50Z` Give standalone `completions` a first-class JSON contract rather than carving it out as a text-only exception. The CLI advertises `--format json` globally, so the standalone admin surface should honor that promise.
- `2026-04-13 10:47Z` Keep debug builds permissive for missing git metadata but make release builds strict. That preserves local developer ergonomics while preventing published binaries from embedding unverifiable `unknown` commit metadata.
- `2026-04-13 10:47Z` Add the first sibling `#[path = "main_tests.rs"]` test module in `xtask` while closing `A09`. The repo-wide inline-test finding remains open, but the release tooling now has maintainable tempdir coverage without making `main.rs` even larger.
- `2026-04-13 10:58Z` For lifecycle state, conservative is safer than synthetic precision. A session we cannot lock should stay "running" with persisted metadata rather than becoming a fake stopped `"(locked)"` row that cleanup might destroy.
- `2026-04-13 11:11Z` Restart should persist the replacement before it disrupts the original session. That ordering lets replacement-append failures abort cleanly with the old session still alive, and it narrows any later remove failure to a stale-log problem instead of a lost-live-session problem.
- `2026-04-13 11:23Z` Treat the PTY reader as the one justified exception to the default bounded-channel policy. Submission paths still stay bounded, but PTY output events now use an unbounded queue so the child process cannot be backpressured by daemon-side lock contention.
- `2026-04-13 11:41Z` Treat `AGENT_TUI_UI_URL` as a same-origin override, not a generic remote frontend bridge. The CLI may still target an alternate path or query on the daemon's own origin, but it must not inject a localhost bearer-token websocket URL into a different origin.
- `2026-04-13 13:31Z` Treat stream-RPC setup as part of the same transport contract as unary calls. Retry/backoff and timeout handling are now expected on the stream handshake too; once the stream is established, the long-lived polling semantics remain separate.
- `2026-04-13 14:18Z` Keep the green-base contract literal. A minor telemetry test regression surfaced while re-running `just ready`; I fixed that expectation mismatch first and re-ran both `just ready` and `just test-core-e2e` before moving on.
- `2026-04-13 15:18Z` Treat stale findings the same as code drift. Once the `insta` tranche and `F01` env propagation were proven under `just ready` plus `just test-core-e2e`, the ledger had to be narrowed immediately so the remaining work reflects real gaps instead of old audit text.
- `2026-04-13 16:03Z` Keep listener-level tests honest by controlling the runtime, not the machine. Shrinking the session stream buffer and heartbeat interval in a test-only seam made the WebSocket coverage deterministic without trying to manufacture TCP backpressure or five-second sleeps in every test.
- `2026-04-13 16:58Z` Close `A10` only on direct proof. The ledger now points at the exact `rg` checks that show inline `mod tests` blocks are gone from `cli/crates` and that the audited sibling test files no longer use the old wall-clock timing patterns.
- `2026-04-13 16:58Z` Resolve the remaining `cargo-deny` noise with explicit policy, not lockfile churn. The supported-target scope and justified `bans.skip` entries document the real dependency situation without inventing fake local unification work.

## Outcomes & Retrospective

The remediation program is complete. The findings ledger is now empty, the audited `codex-rs` parity gaps have been either fixed or explicitly marked non-applicable, and the repo finishes on a green base with `just ready`, `just test-core-e2e`, and `cd cli && cargo deny check advisories bans licenses sources` all passing.

The highest-value wins were behavioral, not cosmetic: attach/runtime shutdown ownership is explicit, boundary errors stay structured, live preview and resize semantics are covered at the listener level, render output is pinned with snapshots, spawn env overrides are now real end-to-end behavior, and the test layout across `cli/crates` is materially closer to the `codex-rs` sibling-module pattern.

The remaining gaps are no longer audit findings. A few workspace tests still use bounded real-time polling for kernel-visible process or lock transitions, but those are outside the original async-timer findings and were kept honest as integration probes rather than forced into fake virtual-time abstractions.

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
- Completed tranche 4: `A04` PTY/runtime ownership findings plus revalidation of the stale terminal-size invariant findings in `A01`/`F05`.
- Completed tranche 5: `F09`, `F10`, and `A11` live-preview security, spec parity, and listener-level contract coverage.
- Completed tranche 6: the repo-wide sibling-test migration, the `A10` proof refresh, and the `cargo-deny` target/duplicate-policy cleanup that leaves the policy fully green without pretending the upstream graph can be flattened locally.

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
