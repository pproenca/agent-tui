# Bash CLI Tests Final Required Lanes

## Purpose / Big Picture

Complete the required bash suite under `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests` so `just ready` exercises all planned required workflow lanes for the built `agent-tui` binary. The visible outcome is three new explicit scenario scripts that prove lifecycle, live-preview, and failure-path behavior against a real local daemon without hiding the `agent-tui` calls behind helper wrappers.

## Progress

- [x] (2026-04-14 08:48Z) Create the exec plan and confirm the exact CLI/runtime contracts for lifecycle, live-preview, and failure-path scenarios.
- [x] (2026-04-14 08:48Z) Add `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-lifecycle-shell.sh`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-live-preview.sh`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-failures-basic.sh`.
- [x] (2026-04-14 08:48Z) Update `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/README.md` so the required-lane matrix matches the implemented suite.
- [x] (2026-04-14 08:48Z) Validate the suite with shell syntax checks, `just cli-tests`, and `just ready`, then move this plan to completed.

## Surprises & Discoveries

- `2026-04-14 08:48Z` `agent-tui kill --yes` removes the session cleanly enough that `sessions cleanup --yes` has nothing left to remove, so the cleanup assertions need a naturally exited shell rather than a killed one.
- `2026-04-14 08:48Z` `agent-tui live stop --json` returns a JSON explanation string that includes backticks. In bash assertions that text must stay in single quotes or the shell tries to execute the embedded command.
- `2026-04-14 08:48Z` Local probes confirmed stable automation-facing exit codes for the basic failures lane: `69` for missing sessions, `64` for confirmation-required actions without `--yes`, and `75` for `wait --assert` timeouts.

## Decision Log

- `2026-04-14 08:48Z` Keep the three new scenarios fully explicit and accept repeated daemon-start blocks inside each script rather than reintroducing behavior wrappers into `cli-tests/lib.sh`.
- `2026-04-14 08:48Z` Split kill verification from cleanup verification in the lifecycle and failures lanes. Kill proves immediate termination; cleanup proves dead-session record removal after a natural process exit.

## Outcomes & Retrospective

This pass completed the required bash suite.

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-lifecycle-shell.sh` now proves `run`, `restart`, `kill`, and `sessions cleanup` through an explicit shell-based flow.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-live-preview.sh` now proves `daemon status`, `live status`, `live start`, `/ui` reachability, and the current daemon-backed `live stop` semantics.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-failures-basic.sh` now proves missing-session failure, `--no-input` confirmation gating, `wait --assert` timeout behavior, and cleanup idempotence.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/README.md` now marks every required lane as implemented.

The required tier now contains seven scenarios total: smoke, viewer, editor, sessions, lifecycle, live, and failures. No additional runner changes were needed because `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/run.sh` already discovers `req-*.sh`.

## Context and Orientation

Files expected to change in this pass:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-lifecycle-shell.sh`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-live-preview.sh`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-failures-basic.sh`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/README.md`
- `/Users/pedroproenca/Documents/Projects/agent-tui/docs/exec-plans/completed/bash-cli-tests-final-required-lanes.md`

Existing supporting files:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/lib.sh` provides only scenario bootstrap, teardown, and generic assertions. It intentionally does not wrap `agent-tui` behavior.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-smoke-pet.sh`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-viewer-top.sh`, `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-editor-vim.sh`, and `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-sessions-switch.sh` are the current explicit required scenarios.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/run.sh` discovers `req-*.sh` automatically, so new required scripts become part of `just cli-tests` and `just ready` without runner changes.

Terms used in this plan:

- `lifecycle lane`: a workflow that proves session creation, restart, kill, and dead-session cleanup.
- `live lane`: a workflow that proves daemon-backed live preview URLs, UI reachability, and the current `live stop` semantics.
- `failures lane`: a workflow that proves expected non-zero exits and error messages for missing sessions, confirmation gating, and wait timeouts.

Current known runtime contracts from local probes:

- `agent-tui --session <id> restart --yes --json` returns JSON with `old_session_id` and `new_session_id`, and the new session becomes active.
- `agent-tui sessions cleanup --yes --json` removes dead/orphaned sessions and reports `sessions_cleaned`.
- `agent-tui live status --json` exposes `ui_url` and the daemon-backed `/ui` endpoint is fetchable with `curl`.
- `agent-tui live stop --json` does not stop the daemon-backed preview; it returns a JSON explanation that the daemon must be stopped instead.
- Failure probes showed exit code `69` for missing sessions, `64` for confirmation-required actions run with `--no-input` but without `--yes`, and `75` for `wait --assert` timeouts.

## Plan of Work

### Milestone 1: Encode the lifecycle lane explicitly

Goal: prove session restart and cleanup with a real long-lived shell process.

Work: add a scenario that starts an isolated daemon, runs `sh -lc 'printf ...; sleep ...'`, waits for text, restarts the session, verifies the new session is active, kills it, and removes the dead session with `sessions cleanup --yes`.

Result: the required suite directly covers the restart/cleanup behavior that a real agent flow depends on.

Proof: the scenario exits zero and its saved JSON/text artifacts show the old session id, new session id, and cleanup count.

### Milestone 2: Encode the live-preview lane explicitly

Goal: prove that daemon-backed live preview metadata and the embedded `/ui` route are usable.

Work: add a scenario that starts the daemon, inspects `daemon status --json`, `live status --json`, and `live start --json`, fetches `ui_url` with `curl`, and asserts the current `live stop --json` explanation without expecting the daemon to stop.

Result: the suite verifies the actual live-preview contract instead of assuming a separate UI process exists.

Proof: the scenario saves the JSON outputs and fetched HTML, and assertions confirm the UI endpoint and stop explanation.

### Milestone 3: Encode the failures lane explicitly

Goal: prove expected non-happy-path exits and messages without interactive fallback.

Work: add a scenario that asserts missing-session screenshot failure, `--no-input` confirmation failure on `kill`, a wait timeout exit from `wait --assert`, and idempotent cleanup reporting.

Result: the suite now covers the main automation-facing failure contracts with explicit exit-code assertions.

Proof: the scenario exits zero only if the expected exit codes and messages are observed.

### Milestone 4: Refresh docs and verify the full path

Goal: keep the README and CI path aligned with the new suite shape.

Work: update the lane matrix and planned-scenario list in `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/README.md`, then run syntax checks, `just cli-tests`, and `just ready`.

Result: the suite documentation matches reality and the new scenarios are proven through the same top-level command CI uses.

Proof: the README marks the remaining required lanes as implemented and the verification commands pass.

## Concrete Steps

1. Add the three required scenario scripts with inline `agent-tui` commands and local assertions.
2. Update `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/README.md` to mark `lifecycle`, `live`, and `failures` as implemented.
3. Run `bash -n /Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/*.sh`.
4. Run `cd /Users/pedroproenca/Documents/Projects/agent-tui && just cli-tests`.
5. Run `cd /Users/pedroproenca/Documents/Projects/agent-tui && just ready`.
6. Update this plan with timestamps, discoveries, and outcomes, then move it to `/Users/pedroproenca/Documents/Projects/agent-tui/docs/exec-plans/completed/`.

## Validation and Acceptance

Validation commands:

1. `bash -n /Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/*.sh`
2. `cd /Users/pedroproenca/Documents/Projects/agent-tui && just cli-tests`
3. `cd /Users/pedroproenca/Documents/Projects/agent-tui && just ready`

Expected results:

- The new `req-lifecycle-shell.sh`, `req-live-preview.sh`, and `req-failures-basic.sh` scripts are discovered automatically by `cli-tests/run.sh`.
- The README marks all required workflow lanes as implemented.
- The full `just ready` path stays green with the larger required suite.

## Idempotence and Recovery

- Each scenario must create its own isolated runtime directory via `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/lib.sh`, so reruns should not collide on sockets or live-preview state files.
- If `req-live-preview.sh` proves flaky due to environment-specific `curl` behavior, keep the JSON assertions and temporarily narrow the HTML fetch to the first successful bytes of `/ui`.
- If `req-failures-basic.sh` shows platform-specific exit-code drift, keep the stderr message assertions and update only the exact numeric codes after re-probing locally rather than weakening the whole scenario.
