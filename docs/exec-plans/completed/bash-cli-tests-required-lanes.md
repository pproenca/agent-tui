# Bash CLI Tests Required Lanes

## Purpose / Big Picture

Expand `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests` from a single smoke scenario into a small required-tier suite that covers the highest-value real workflows for the built `agent-tui` binary. The visible outcome is that `just ready` exercises focused bash scenarios for viewer behavior (`top`), editor behavior (`vim`), and multi-session switching, while the shell code stays maintainable through a minimal shared helper file instead of copy-pasted setup blocks.

## Progress

- [x] (2026-04-14 07:46Z) Create the exec plan and lock the scope to minimal shared helpers plus three required scenarios.
- [x] (2026-04-14 07:52Z) Add a minimal shared helper library under `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests` and migrate the existing smoke scenario to it.
- [x] (2026-04-14 07:52Z) Implement focused required scenarios for viewer, editor, and sessions lanes.
- [x] (2026-04-14 07:52Z) Update suite documentation/status and verify `just cli-tests` plus `just ready`.

## Surprises & Discoveries

- `2026-04-14 07:46Z` The current suite already validates the required-tier naming scheme, so the next risk is maintainability rather than naming drift.
- `2026-04-14 07:46Z` `agent-tui --json sessions` exposes both `active_session` and the full session list, which is enough to verify active-session switching without introducing `jq` or a heavier JSON parser.
- `2026-04-14 07:52Z` The first editor scenario failed immediately because `agent-tui resize` requires `--cols` and `--rows` flags rather than positional dimensions, so the bash scenarios need to mirror the CLI contract exactly instead of assuming the friendliest shape.
- `2026-04-14 07:52Z` macOS socket path length limits surfaced once multiple scenarios reused long runtime directory names. Shortening the runtime directory and socket filenames in the shared helper solved the issue for every scenario at once.
- `2026-04-14 07:52Z` During the smoke refactor an earlier malformed patch had spliced scenario text into `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-smoke-pet.sh`; the verification pass caught it because the suite executed that file after the new scenarios had already passed.

## Decision Log

- `2026-04-14 07:46Z` Introduce a minimal `cli-tests/lib.sh` now because four required scenarios would otherwise duplicate daemon setup, JSON extraction, cleanup, and session helpers enough to become the new maintenance problem.
- `2026-04-14 07:46Z` Keep the smoke scenario in place instead of replacing it. The focused scenarios prove specific lanes, while the smoke scenario remains a cheap broad sanity check.

## Outcomes & Retrospective

This pass delivered the first genuinely shaped required bash suite.

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/lib.sh` now centralizes isolated daemon setup, cleanup, JSON extraction, and common session helpers.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-smoke-pet.sh` now uses the shared helper instead of carrying its own setup stack.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-viewer-top.sh` proves viewer-style output observation with `top`.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-editor-vim.sh` proves `vim` typing plus resize stability.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-sessions-switch.sh` proves session inventory, `sessions show`, active-session switching, and default-session behavior.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/README.md` now marks the viewer, editor, and sessions lanes as implemented.

The suite now has four required scenarios total: smoke, viewer, editor, and sessions. That is enough structure to keep adding lanes incrementally without each new script reinventing daemon setup or session plumbing.

## Context and Orientation

Files expected to change in this pass:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/lib.sh`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-smoke-pet.sh`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-viewer-top.sh`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-editor-vim.sh`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-sessions-switch.sh`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/README.md`
- `/Users/pedroproenca/Documents/Projects/agent-tui/docs/exec-plans/active/bash-cli-tests-required-lanes.md`

Terms used in this plan:

- `viewer lane`: a read-only workload where `agent-tui` must capture and wait on terminal output correctly.
- `editor lane`: an interactive full-screen workload where `agent-tui` must type text, send keys, and preserve screen state through resize.
- `sessions lane`: a workflow that depends on more than one live session and verifies inventory plus active-session switching.
- `helper library`: a sourced shell file with only the common scenario primitives, not a full test framework.

Current state:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-smoke-pet.sh` already proves a broad happy path with `top` and `vim`.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/run.sh` already discovers `req-*.sh` and is wired into `xtask ci`.
- The suite does not yet have separate required scenarios for viewer-only, editor-only, or multi-session behavior.

## Plan of Work

### Milestone 1: Extract just enough shell reuse

Goal: remove the repeated setup/cleanup and session primitives before adding more scenarios.

Work: add a small `cli-tests/lib.sh` with isolated daemon setup, common argument parsing, JSON field extraction, session helpers, and cleanup behavior; migrate the existing smoke scenario to it.

Result: new scenarios can stay short and focused on behavior instead of shell plumbing.

Proof: `req-smoke-pet.sh` still passes after sourcing the helper library.

### Milestone 2: Add the three highest-value required lanes

Goal: cover the viewer, editor, and sessions lanes with real tools and narrow assertions.

Work: add:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-viewer-top.sh`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-editor-vim.sh`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-sessions-switch.sh`

Result: the bash suite now covers focused scenarios instead of relying only on a single smoke pass.

Proof: each scenario passes directly under `just cli-tests`.

### Milestone 3: Refresh the suite docs and verify the integrated path

Goal: keep the documentation aligned with the actual suite.

Work: update `cli-tests/README.md` status lines and planned-scenario inventory, then run the narrow and full verification commands.

Result: the docs describe the real required-tier suite and CI proves it.

Proof: `just cli-tests` and `just ready` both pass with the new scenarios included.

## Concrete Steps

1. Add `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/lib.sh` and migrate `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-smoke-pet.sh`.
2. Add the viewer, editor, and sessions required scenarios.
3. Update `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/README.md` to mark the implemented lanes.
4. Run `bash -n` over the suite scripts, `just cli-tests`, and `just ready`.
5. Update this plan with completion timestamps and validation results.

## Validation and Acceptance

Validation commands:

1. `bash -n /Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/lib.sh`
2. `bash -n /Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-*.sh`
3. `cd /Users/pedroproenca/Documents/Projects/agent-tui && just cli-tests`
4. `cd /Users/pedroproenca/Documents/Projects/agent-tui && just ready`

Expected results:

- The smoke scenario still passes after the helper extraction.
- The new `req-viewer-top.sh`, `req-editor-vim.sh`, and `req-sessions-switch.sh` scenarios are discovered and executed by the suite runner.
- The README marks those three lanes as implemented.
- The full CI path remains green.

## Idempotence and Recovery

- Re-running any scenario should be safe because the shared helper creates isolated runtime paths and stops the daemon in cleanup.
- If the helper extraction causes broad regressions, revert only the scenario migrations first and keep the new focused scenarios blocked until the shared setup is stable.
- If the sessions scenario proves flaky, keep the viewer and editor scenarios and narrow the sessions assertions to list/show/switch output until the cause is understood.
