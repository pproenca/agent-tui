# OpenAI Codex Rust Audit Program

## Purpose / Big Picture

Create and execute a persistent, resumable audit program for `/Users/pedroproenca/Documents/Projects/agent-tui` that evaluates the Rust workspace against all 60 rules in the `openai-codex-rust-patterns` skill. The visible outcome is a repo-local audit system that can be resumed by any later session without re-discovery: a full file-by-rule tracker, a findings ledger, and a progress narrative that records which audit units are complete and what was found.

## Progress

- [x] (2026-04-12 22:03Z) Create the persistent audit artifacts: plan, matrix, and findings ledger.
- [x] (2026-04-12 22:03Z) Complete the workspace and architecture tranche (`A08`) and record findings.
- [x] (2026-04-12 22:06Z) Complete the domain invariants tranche (`A01`) and record findings.
- [x] (2026-04-12 22:07Z) Complete the error-boundary tranche (`A02`) and record findings.
- [x] (2026-04-12 22:12Z) Complete the test harness tranche (`A10`) and record findings.
- [x] (2026-04-12 22:19Z) Complete the session spawn and initial run tranche (`F01`) and record findings.
- [x] (2026-04-12 23:19Z) Complete the resize and terminal reflow tranche (`F05`) and record findings.
- [x] (2026-04-13 00:20Z) Complete the session repository and persistence tranche (`A03`) and record findings.
- [x] (2026-04-13 06:53Z) Complete the PTY and virtual terminal engine tranche (`A04`) and record findings.
- [x] (2026-04-13 06:59Z) Complete the concurrency, shutdown, and thread/task ownership tranche (`A05`) and record findings.
- [x] (2026-04-13 07:33Z) Complete the daemon lifecycle control plane tranche (`F08`) and record findings.
- [x] (2026-04-13 07:48Z) Complete the session lifecycle management tranche (`F07`) and record findings.
- [x] (2026-04-13 08:04Z) Complete the snapshot and screenshot rendering tranche (`F02`) and record findings.
- [x] (2026-04-13 08:13Z) Complete the input injection tranche (`F03`) and record findings.
- [x] (2026-04-13 08:27Z) Complete the wait and assert semantics tranche (`F04`) and record findings.
- [x] (2026-04-13 08:37Z) Complete the interactive attach session tranche (`F06`) and record findings.
- [x] (2026-04-13 08:48Z) Complete the live preview control plane tranche (`F09`) and record findings.
- [x] (2026-04-13 08:54Z) Complete the live preview data plane tranche (`F10`) and record findings.
- [x] (2026-04-13 09:04Z) Complete the Rust-to-web contract boundary tranche (`A11`) and record findings.
- [x] (2026-04-13 09:16Z) Complete the IPC transport and client/server RPC tranche (`F11`) and record findings.
- [x] (2026-04-13 09:24Z) Complete the process control and isolation tranche (`A06`) and record findings.
- [x] (2026-04-13 09:31Z) Complete the CLI admin and operator UX tranche (`F12`) and record findings.
- [x] (2026-04-13 09:40Z) Complete the observability and runtime diagnostics tranche (`A07`) and record findings.
- [x] (2026-04-13 09:47Z) Execute the remaining feature-slice and shared-runtime audits until every audit unit in `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-audit-inventory.md` is marked complete.
- [x] (2026-04-13 09:47Z) Close the plan by filling the retrospective and moving it to `completed/`.

## Surprises & Discoveries

- `2026-04-12 22:00Z` The repository does not currently contain `/Users/pedroproenca/Documents/Projects/agent-tui/docs/PLANS.md` or `/Users/pedroproenca/Documents/Projects/agent-tui/docs/exec-plans/`, so this plan uses the `exec-plan` skill's required structure directly instead of inheriting a local template.
- `2026-04-12 22:03Z` The first matrix draft undercounted the perimeter because it omitted non-`src` audit targets such as `build.rs`, workspace manifests, API specs, and Rust-to-web boundary files. The corrected baseline is `130` targets and `7,800` audit cells.
- `2026-04-12 22:06Z` The domain already contains a strong `TerminalSize` invariant type, but spawn and resize request DTOs still move raw `u16` dimensions through the boundary, which means the invariant is not actually enforced at all entry points.
- `2026-04-12 22:12Z` The transport-test surface is stronger than expected because `MockDaemon` drives the real Unix-socket JSON-RPC protocol and can inject malformed frames, but the broader suite still has `50` inline `#[cfg(test)]` modules, zero sibling `#[path]` test stubs, and no snapshot-testing dependency.
- `2026-04-12 22:19Z` The public OpenAPI and AsyncAPI specs do not describe the JSON-RPC `spawn` contract, so the `F01` env-propagation audit had to treat the Rust adapter boundary as the authoritative interface rather than checking a versioned external spec.
- `2026-04-12 23:19Z` Spawn input clamps terminal sizes at the RPC boundary, but resize does not: `resize` DTOs, use cases, PTY/vterm engines, and session state all accept raw `u16`, and `SessionManager::list` later hides invalid live sizes with `TerminalSize::try_new(cols, rows).unwrap_or_default()`.
- `2026-04-13 00:20Z` The JSONL session store already has a workable forward-compatibility strategy despite the strict `SessionEvent` enum: unknown records are preserved by append-only cleanup and compaction refusal, so future log entries survive older runtimes without requiring an explicit `Unknown` variant.
- `2026-04-13 06:53Z` The session runtime already caps retained stream bytes in `StreamBuffer`, but the lower-level PTY hop still blocks on a bounded `ReadEvent` channel and never joins or times out the detached `pty-reader` thread, so the remaining risk sits below the existing ring buffer rather than inside it.
- `2026-04-13 06:59Z` The concurrency layer splits cleanly into two shutdown qualities: per-connection WebSocket stream tasks already do timeout-plus-abort correctly, but the daemon's owner threads still stop at "warn and detach", and the shutdown use case treats wakeup delivery as infallible even though the accept loop depends on a byte written into `ShutdownWaker`.
- `2026-04-13 07:33Z` The graceful daemon-stop helper is less safe than it first looks: `stop_daemon_via_rpc()` only waits for socket disappearance after an acknowledged shutdown, and if the socket never goes away it deletes the socket and lock files anyway without proving the daemon process exited.
- `2026-04-13 07:48Z` The session lifecycle surface relies on two lossy selector/state shorthands that are easy to miss in review: malformed `session` parameters are silently treated as "active", and lock-timed-out sessions are surfaced to cleanup as synthetic stopped `"(locked)"` entries even though the underlying process may still be running.
- `2026-04-13 08:04Z` The screenshot boundary has two optimistic-success holes that sit on top of otherwise solid rendering helpers: `--region` and the RPC `region` field are wired through the CLI, DTO, and adapter layers but ignored by `SnapshotUseCaseImpl`, and `SessionHandleImpl::update()` treats flush acknowledgement as best-effort even though the screenshot path depends on that barrier for freshness.
- `2026-04-13 08:13Z` The input slice hides two different semantic traps: the advertised modifier hold/release path only toggles unused booleans in `Session`, and attach correctly batches explicit `Event::Paste` frames but has no fallback burst detector for terminals that emit pasted text as rapid key events instead.
- `2026-04-13 08:27Z` The wait/assert slice currently relies on two hidden shortcuts at once: each wait iteration issues two refreshes but discards the second result, and the existing tests stop at routing plus timeout exit code even though the wait use case and mock-daemon harness already have deterministic timing seams for convergence and timeout behavior.
- `2026-04-13 08:37Z` The attach runtime already has the right raw ingredients for safer teardown, but they are not composed: `TerminalGuard` restores on ordinary drop yet ignores initial terminal-setup failure and lacks a chained panic hook, while `RpcStream` exposes an abort handle that the attach loops only use on the explicit detach path before falling back to warn-and-detach helper-thread shutdown.
- `2026-04-13 08:48Z` The live-preview control plane is less local-only than it first appears: the listener itself stays on loopback by default, but `AGENT_TUI_UI_URL` can hand the full authenticated localhost WS URL to an arbitrary browser target, and the published OpenAPI contract has drifted far enough that it now describes unauthenticated routes and parameters the runtime does not actually serve.
- `2026-04-13 08:54Z` The data-plane spec drift is even wider than the control-plane doc drift suggested: the published AsyncAPI still describes `hello`, `command`, `error`, and binary `output` messages with query-selected encoding, while the runtime only ships token-authenticated JSON-RPC text responses with base64 `output` events and no binary mode.
- `2026-04-13 09:04Z` The first-party web UI has now drifted enough from the published docs to expose two contract layers at once: the browser is coded for token-authenticated JSON-RPC envelopes with `active_session`, while direct `/ui` fallback still silently tries unauthenticated same-origin `/ws` and surfaces the resulting auth failure only as generic disconnected/load errors.
- `2026-04-13 09:16Z` The transport/RPC layer contains two easy-to-miss mismatches: the client advertises retry knobs that no call path ever consumes, and the daemon's Unix-socket request-size limit is cumulative across the whole connection rather than per request, which means long-lived clients can fail later small RPCs for the wrong reason.
- `2026-04-13 09:24Z` The process-control boundary still lacks a stable notion of process identity: daemon and UI control paths trust bare PIDs from lock or state files, treat `kill(pid, 0)` as sufficient validation, and can therefore report or signal an unrelated process after PID reuse even though the lower-level signal translation itself is typed and explicit.
- `2026-04-13 09:31Z` The operator-facing CLI still has one whole-surface contract drift that the lower layers do not reveal: global help promises `--format json` for machine-readable output, but the standalone `completions` flow bypasses the presenter layer entirely, while `live stop` still prints and returns the same UI-stop failure through two separate text paths.
- `2026-04-13 09:40Z` The observability slice has a sharper credential-leak edge than expected: the live-preview server logs the full tokenized `ws_url`, transport debug logs can repeat the same address, and the telemetry bootstrap both suppresses targets and installs only one sink, so there is no built-in log-only versus trace-safe path to contain sensitive diagnostics.
- `2026-04-13 09:47Z` The build and release slice turned out to be half-finished in two different directions: version synchronization is stricter than expected across Cargo, npm, and optional platform packages, but the actual `xtask release` path bypasses those validation helpers entirely, and all three build scripts silently stamp `unknown` metadata when version or git discovery fails.

## Decision Log

- `2026-04-12 22:00Z` Use a generated TSV matrix for the file-by-rule audit cells instead of a Markdown table because the state space is too large for manual editing and needs deterministic regeneration.
- `2026-04-12 22:00Z` Treat the existing audit inventory at `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-audit-inventory.md` as the coverage perimeter and audit-unit registry rather than duplicating that taxonomy in this plan.
- `2026-04-12 22:03Z` Expand the matrix perimeter beyond Rust `src/**/*.rs` to include manifests, `build.rs`, API specs, and Rust-to-web contract files because those artifacts materially affect the skill's `workspace`, `proto`, and `tui` audit categories.
- `2026-04-12 22:12Z` Treat the skill's `wiremock` and SSE testing rule as a transport-agnostic requirement for real wire-level fakes, because this repository's runtime boundary is Unix socket JSON-RPC and WebSocket rather than outbound HTTP SSE.
- `2026-04-12 22:19Z` Record the missing request-scoped env propagation in `F01` under `sandbox-env-clear-pre-exec` even though `agent-tui run` is not a codex-style sandbox, because the audit inventory explicitly requires cwd/env propagation review for the spawn path.
- `2026-04-12 22:19Z` Queue `F05` before `A03` after `F01` because the still-open terminal-size invariant gap feeds directly into resize and reflow behavior, while `A03` can then revisit persistence semantics shared by both flows.
- `2026-04-12 23:19Z` Treat silent attach resize RPC failures as an `errors-boundary-error-translator` finding because `attach.rs` drops structured `resize` errors instead of surfacing or logging them for the operator.
- `2026-04-12 23:19Z` Queue `A03` ahead of `A04` after `F05` because the resize audit exposed size masking inside `SessionManager::list`, and then revisit the deeper PTY/vterm engine guarantees before returning to broader lifecycle flows.
- `2026-04-13 00:20Z` Treat warn-only session-store writes and startup cleanup failures as an `errors-boundary-error-translator` finding because `SessionManager` mutates live state first and then suppresses persistence errors that materially affect next-start recovery.
- `2026-04-13 00:20Z` Queue `A04` before `F07` after `A03` because the persistence audit confirmed the next unresolved shared guarantee sits lower in the stack, inside PTY/vterm buffering and rendering, before revisiting the higher-level lifecycle commands that sit on top of it.
- `2026-04-13 06:53Z` Queue `A05` ahead of `F07` after `A04` because the bounded PTY event channel and missing reader-drain timeout are shared ownership and shutdown guarantees, then return to lifecycle behavior and screenshot rendering once those runtime semantics are better mapped.
- `2026-04-13 06:59Z` Queue `F08` ahead of `F07` after `A05` because the concurrency audit exposed control-plane shutdown risks in the daemon and WS runtime owners themselves (silent wakeup failure and detached threads after timeout), so the daemon lifecycle surface is now the highest-yield next feature slice.
- `2026-04-13 07:33Z` Queue `F07` ahead of `F02` after `F08` because the daemon control-plane audit completed the start/stop/restart boundary and exposed no new session-model prerequisites, so the next highest-yield slice is the still-unaudited session lifecycle surface that sits directly on the same persistence and stop paths.
- `2026-04-13 07:48Z` Queue `F02` ahead of `F03` after `F07` because the session lifecycle audit revalidated the need for trustworthy rendered-session state, and the next highest-yield slice is the screenshot/rendering boundary that sits directly on the `render`/`vterm` guarantees already mapped in `A04`.
- `2026-04-13 08:04Z` Queue `F03` ahead of `F04` after `F02` because the snapshot audit exposed optimistic refresh semantics at the same session boundary used by input injection, so the next highest-yield slice is the write path before validating wait/assert behavior that depends on post-input state convergence.
- `2026-04-13 08:13Z` Queue `F04` ahead of `F06` after `F03` because the input audit exposed semantic gaps in what terminal writes actually do, so the next highest-yield slice is the wait/assert boundary that decides whether those writes are observed correctly before returning to the more UI-heavy attach path.
- `2026-04-13 08:27Z` Queue `F06` ahead of `F09` after `F04` because the wait/assert audit completed the remaining state-convergence slice around the core CLI loop, so the next highest-yield review is the interactive attach runtime that consumes those same input, render, and session-state guarantees before moving on to live-preview control-plane behavior.
- `2026-04-13 08:37Z` Queue `F09` ahead of `F10` and `A06` after `F06` because the attach audit closed the last unaudited operator-facing interactive CLI path, so the next highest-yield work is the live-preview browser/control-plane surface and then the shared process/isolation boundary underneath it.
- `2026-04-13 08:48Z` Queue `F10` ahead of `A11` and `A06` after `F09` because the control-plane audit exposed both wire-contract drift and credential-sharing behavior on the same live-preview subsystem, so the next highest-yield step is the live-preview data plane itself before lifting to the broader Rust-to-web contract boundary and remaining process/isolation review.
- `2026-04-13 08:54Z` Queue `A11` ahead of `F11` and `A06` after `F10` because the data-plane audit confirmed the highest-yield remaining risk is now the Rust-to-web contract boundary that consumes these stream payloads, then the underlying transport and RPC surface, and only after that the broader process and isolation boundary.
- `2026-04-13 09:04Z` Queue `F11` ahead of `A06` and `F12` after `A11` because the web-boundary audit confirmed the next highest-yield unresolved work is the transport and RPC surface that feeds those browser contracts, then the remaining process/isolation boundary, before finishing the operator-facing admin UX.
- `2026-04-13 09:16Z` Queue `A06` ahead of `F12` and `A07` after `F11` because the transport audit finished the client/server framing surface and left the deeper process-spawn, environment-inheritance, and local-bind safety guarantees as the next unresolved boundary underneath it; operator UX and runtime diagnostics come after that substrate is mapped.
- `2026-04-13 09:24Z` Queue `F12` ahead of `A07` and `A09` after `A06` because the process/isolation audit closed the remaining OS-boundary substrate, so the next highest-yield work is the operator-facing CLI UX that reports those states, then runtime diagnostics, and finally build or release tooling.
- `2026-04-13 09:31Z` Queue `A07` ahead of `A09` after `F12` because the CLI admin audit closed the remaining operator-facing command surface, so the next highest-yield unresolved work is the observability and diagnostics substrate those commands rely on before finishing the build/release tooling tranche.
- `2026-04-13 09:40Z` Queue `A09` as the final remaining tranche after `A07` because the observability audit closed the last shared-runtime surface outside build and release tooling, leaving versioning, packaging, and distribution as the only unfinished inventory unit before the retrospective.
- `2026-04-13 09:47Z` Mark the audit program complete after `A09` because the inventory is exhausted, the findings ledger and matrix are synchronized, and the remaining work is remediation rather than coverage.

## Next Queue

- `(none - audit inventory exhausted)`

## Outcomes & Retrospective

The audit program completed all inventory units defined in `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-audit-inventory.md` and left the repository with four durable artifacts: this completed exec plan, the findings ledger, the file-by-rule matrix, and the automation memory checkpoint. The main outcome is not code changes but an auditable map of where the repository matches Codex Rust patterns and where it diverges.

The highest-risk gaps clustered around optimistic-success boundaries and contract drift instead of obvious panic sites: ignored region and modifier APIs, duplicated wait refreshes, detached shutdown paths, stale protocol docs, PID-only process identity, token-bearing live-preview URLs, and release tooling that can tag or stamp incomplete metadata. The audit also found several places where the codebase is stronger than it first appears, including the real socket-based mock daemon, the version synchronization between Cargo and npm packaging, and the explicit connection or stream timing spans.

Remaining work is implementation remediation, not audit coverage. The open findings in `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-findings.md` are the prioritized follow-up queue; the audit inventory itself is exhausted, so this plan moves to `completed/` and future sessions should reference the findings ledger or start separate remediation plans instead of reopening audit discovery.

## Context and Orientation

The codebase is a Unix-only Rust workspace rooted at `/Users/pedroproenca/Documents/Projects/agent-tui/cli`. It contains 8 crates plus workspace tests and tooling:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-common`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-adapters`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-infra`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-app`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/xtask`

Definitions used by this audit:

- `rule`: one concrete guideline from `openai-codex-rust-patterns`, for example `defensive-deny-unwrap-workspace-wide`.
- `audit category`: one of the 10 top-level buckets such as `defensive`, `errors`, or `workspace`.
- `audit unit`: one feature slice or shared runtime area defined in `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-audit-inventory.md`.
- `audit cell`: one `file_path × rule_id` review record.
- `matrix`: the generated baseline table of all audit cells.
- `findings ledger`: the narrative log of concrete findings, decisions, and tranche completion status.

Known counts at plan creation:

- Audit targets in scope: `130`
- Rules in scope: `60`
- Baseline audit cells: `7,800`

Authoritative source files for the audit:

- Skill entry point: `/Users/pedroproenca/Documents/Projects/agent-tui/.agents/skills/openai-codex-rust-patterns/SKILL.md`
- Full skill index: `/Users/pedroproenca/Documents/Projects/agent-tui/.agents/skills/openai-codex-rust-patterns/AGENTS.md`
- Audit inventory: `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-audit-inventory.md`

Persistent artifacts created by this plan:

- Plan: `/Users/pedroproenca/Documents/Projects/agent-tui/docs/exec-plans/active/openai-codex-rust-audit-program.md`
- Matrix: `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-rule-matrix.tsv`
- Findings ledger: `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-findings.md`

## Plan of Work

### Milestone 1: Create durable audit state

Goal: establish files that let later sessions continue without rebuilding scope.

Work: generate the file-by-rule matrix from the current Rust file list and the 60 skill rule identifiers; create a findings ledger that records tranche completion, findings, and next targets.

Result: any later session can answer "what is left?" and "what has been reviewed?" from repo files instead of conversation history.

Proof: the matrix row count matches `130 × 60 + 1 header`, and the findings ledger contains at least one tranche section.

### Milestone 2: Complete the workspace/architecture tranche

Goal: validate the compiler-level and workspace-level structure before deeper feature audits.

Work: inspect `/Users/pedroproenca/Documents/Projects/agent-tui/cli/Cargo.toml`, all crate `Cargo.toml` files, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/clippy.toml`, architecture docs, and architecture tests against the applicable `workspace-*`, `defensive-*`, and `testing-*` rules.

Result: a completed `A08` tranche with recorded passes/findings and matrix cells updated for the directly reviewed workspace files.

Proof: the findings ledger has an `A08` section with verdicts, and the matrix includes non-`pending` rows for the audited files/rules.

### Milestone 3: Complete the domain invariants tranche

Goal: validate the core semantic invariants that affect every feature slice.

Work: audit `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-domain/src/**/*.rs` and any boundary parser that can weaken those invariants before values reach the use cases.

Result: completed `A01` section with concrete findings or explicit pass notes.

Proof: the ledger documents the completed tranche and the matrix rows for the reviewed domain files show non-`pending` statuses.

### Milestone 4: Complete the error-boundary tranche

Goal: validate the error-boundary discipline that affects every feature slice.

Work: audit `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-usecases/src/usecases/ports/errors.rs`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui-common/src/common/daemon_error.rs`, and the surrounding boundary translators.

Result: completed `A02` section with concrete findings or explicit pass notes.

Proof: the ledger documents the completed tranche and the matrix rows for the reviewed error files show non-`pending` statuses.

### Milestone 5: Execute remaining tranches until the inventory is exhausted

Goal: finish every audit unit listed in `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-audit-inventory.md`.

Work: iterate through the remaining feature slices and shared runtime areas, update the ledger after each tranche, and keep the matrix synchronized.

Result: all audit units are complete and all findings are recorded as accepted, deferred, or actionable.

Proof: no unfinished audit unit remains in the ledger and the plan can be moved to `completed/`.

## Concrete Steps

1. Create `/Users/pedroproenca/Documents/Projects/agent-tui/docs/exec-plans/active/` and `/Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/` if missing.
2. Generate the Rust file list with:
   `find /Users/pedroproenca/Documents/Projects/agent-tui/cli/crates -path '*/src/*.rs' -o -path '*/src/**/*.rs' -o -path '/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/*.rs' -o -path '/Users/pedroproenca/Documents/Projects/agent-tui/cli/crates/agent-tui/tests/**/*.rs' | sort`
3. Generate the rule list with:
   `find /Users/pedroproenca/Documents/Projects/agent-tui/.agents/skills/openai-codex-rust-patterns/references -maxdepth 1 -name '*.md' ! -name '_sections.md' | sort`
4. Build the TSV matrix with header columns:
   `file_path`, `rule_id`, `category`, `status`, `applicability`, `audit_units`, `reviewed_at`, `notes`
5. Create the findings ledger with sections for `Open Findings`, `Completed Tranches`, and `Next Queue`.
6. Audit tranche `A08` and immediately update both the ledger and matrix.
7. Continue with `A01` and `A02` after `A08` is written down.

Expected outputs:

- Matrix exists and has `7801` lines including the header.
- Findings ledger exists and names the next queued tranches.
- The plan file contains timestamped progress updates as milestones are completed.

## Validation and Acceptance

Validation commands:

1. `wc -l /Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-rule-matrix.tsv`
   Expected: `7801`
2. `rg -n '^## Completed Tranches|^## Next Queue' /Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-findings.md`
   Expected: both headings present
3. `rg -n 'workspace-single-source-dependencies|workspace-lint-config-package|testing-path-attribute-sibling-tests' /Users/pedroproenca/Documents/Projects/agent-tui/docs/audits/openai-codex-rust-patterns-findings.md`
   Expected: at least one `A08` review note referencing reviewed rules

Acceptance:

- The repo contains durable audit artifacts, not just a chat summary.
- The first tranche is genuinely complete and recorded.
- The plan can be resumed by a fresh session with no hidden context.

## Idempotence and Recovery

- Re-running the matrix generator is safe because it deterministically regenerates the full baseline from the current file list and the current skill reference list.
- If a later session needs to rebuild the matrix after files are added or removed, it should regenerate the file and record the reason in the `Decision Log`.
- If a tranche is partially reviewed but not finished, the findings ledger must note the partial state and the next exact files to continue from.
- No destructive repository operations are required. Recovery is document-driven: regenerate the matrix, keep the plan, and append findings rather than rewriting history.
