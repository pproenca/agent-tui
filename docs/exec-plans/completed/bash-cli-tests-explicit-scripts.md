# Bash CLI Tests Explicit Scripts

## Purpose / Big Picture

Refactor the bash suite under `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests` so the real `agent-tui` command sequences are explicit inside each scenario script instead of being hidden behind helper wrappers. The visible outcome is that contributors can open a scenario file and immediately see the exact `agent-tui` calls being tested, while a tiny shared helper remains only for setup, cleanup, and generic assertions.

## Progress

- [x] (2026-04-14 07:55Z) Create the exec plan and lock the scope to explicit scenario scripts with a lean shared helper.
- [x] (2026-04-14 07:57Z) Reduce the shared helper to setup/cleanup/assertion utilities only.
- [x] (2026-04-14 07:57Z) Inline `agent-tui` command sequences in each required scenario.
- [x] (2026-04-14 07:57Z) Refresh docs and verify `just cli-tests` plus `just ready`.

## Surprises & Discoveries

- `2026-04-14 07:55Z` The current helper file centralizes enough `agent-tui` behavior that the scenario files now read more like a thin DSL than shell scripts.
- `2026-04-14 07:55Z` The current suite already passes, so this refactor must preserve behavior while shifting readability back into the scenario files.
- `2026-04-14 07:57Z` The scripts remained readable after inlining because most scenarios only need a small number of direct CLI calls; the extra repetition was acceptable once the helper stopped hiding the core behavior.
- `2026-04-14 07:57Z` Keeping the cleanup/bootstrap path in the helper still felt right because it deals with test harness state rather than the actual `agent-tui` workflow under test.

## Decision Log

- `2026-04-14 07:55Z` Keep only setup, cleanup, JSON extraction, and generic assertions in the shared helper; move all behavioral `agent-tui` calls back into the scenario scripts.
- `2026-04-14 07:55Z` Prefer a few repeated shell lines over a helper API when those lines reveal the tested workflow more clearly.

## Outcomes & Retrospective

This pass made the bash suite easier to inspect without weakening coverage.

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/lib.sh` now contains only bootstrap, cleanup, JSON extraction, command checks, and generic assertions.
- The required scenarios now show their `agent-tui` calls inline, including daemon startup, `run`, `wait`, `screenshot`, `resize`, `sessions switch`, and `kill`.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/README.md` now explicitly says helpers should not hide behavioral `agent-tui` steps.

The suite remains green with 4 required scenarios, but the scripts now read like concrete test transcripts instead of a helper-driven DSL. That should make future maintenance and review simpler.

## Context and Orientation

Files expected to change in this pass:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/lib.sh`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-smoke-pet.sh`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-viewer-top.sh`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-editor-vim.sh`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-sessions-switch.sh`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/README.md`
- `/Users/pedroproenca/Documents/Projects/agent-tui/docs/exec-plans/active/bash-cli-tests-explicit-scripts.md`

Terms used in this plan:

- `explicit scenario`: a bash test file where the `agent-tui` invocations appear inline in the order they execute.
- `lean helper`: a sourced shell file that handles only environment setup, cleanup, and generic parsing/assertion utilities.
- `behavioral wrapper`: a helper function that hides a specific `agent-tui` action such as spawn, wait, screenshot, or kill.

Current state:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/lib.sh` currently wraps many direct `agent-tui` actions such as `spawn_vim_session`, `assert_text`, `capture_screen`, and `kill_session`.
- The required scenarios are correct, but their agent behavior is partly obscured by those wrappers.

## Plan of Work

### Milestone 1: Shrink the helper surface

Goal: leave only the utilities that are genuinely cross-cutting and not specific to one `agent-tui` workflow.

Work: remove behavioral wrappers from `cli-tests/lib.sh` and keep only bootstrap, cleanup, JSON parsing, command-existence checks, and generic assertions.

Result: the helper stops acting like a test DSL.

Proof: no scenario-specific `agent-tui` verbs remain wrapped in `lib.sh`.

### Milestone 2: Make the scenarios explicit

Goal: make each script readable as a linear transcript of what `agent-tui` is doing.

Work: update the required scenario files to call `"$BIN"` and `"$BIN" --json` directly for daemon start, run, wait, screenshot, resize, switch, and kill operations.

Result: each script shows the exact CLI contract it is exercising.

Proof: the scripts still pass and the command sequence is visible directly in each file.

### Milestone 3: Align docs and re-verify

Goal: make the suite docs match the new explicit-script style.

Work: update the README authoring guidance, then run the narrow suite and full CI.

Result: the suite style is documented and validated.

Proof: `just cli-tests` and `just ready` pass.

## Concrete Steps

1. Trim `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/lib.sh` down to bootstrap and assertion primitives.
2. Inline the `agent-tui` run/wait/screenshot/resize/switch/kill calls in each required scenario.
3. Update `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/README.md` to state that helpers must not hide behavioral `agent-tui` steps.
4. Run `bash -n` over the suite, then `just cli-tests`, then `just ready`.
5. Update this plan with timestamps and validation results.

## Validation and Acceptance

Validation commands:

1. `bash -n /Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/lib.sh`
2. `bash -n /Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-*.sh`
3. `cd /Users/pedroproenca/Documents/Projects/agent-tui && just cli-tests`
4. `cd /Users/pedroproenca/Documents/Projects/agent-tui && just ready`

Expected results:

- `lib.sh` contains only setup/cleanup/assertion utilities.
- The required scenario files show direct `agent-tui` command invocations inline.
- The suite and full CI remain green.

## Idempotence and Recovery

- Re-running the suite is safe because the bootstrap still creates isolated runtime directories and teardown still stops the daemon.
- If the inlining makes a scenario too repetitive, prefer a small local variable over reintroducing a wrapper function.
- If any scenario becomes flaky during the refactor, restore only the minimum wrapper needed temporarily, record it, and keep the rest of the scripts explicit.
