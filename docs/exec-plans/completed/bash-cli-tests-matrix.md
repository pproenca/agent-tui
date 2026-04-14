# Bash CLI Tests Matrix

## Purpose / Big Picture

Turn `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests` into a predictable bash test suite for the built `agent-tui` binary instead of a loose collection of shell scripts. The visible outcome is a documented matrix that explains which real-world workflows belong in bash tests, a naming convention that keeps scenarios organized as the command surface grows, and a suite runner that distinguishes the required CI tier from optional local-only scenarios.

## Progress

- [x] (2026-04-14 06:55Z) Create the exec plan and lock the scope to matrix, naming, and runner conventions.
- [x] (2026-04-14 06:58Z) Document the concrete bash-suite matrix and naming rules in `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests`.
- [x] (2026-04-14 06:58Z) Align the current suite runner and scenario file names with the required/optional tier convention.
- [x] (2026-04-14 06:58Z) Validate the bash suite directly and through `just ready`, then update this plan with results.

## Surprises & Discoveries

- `2026-04-14 06:55Z` The repository still does not contain `/Users/pedroproenca/Documents/Projects/agent-tui/docs/PLANS.md`, so this plan follows the established `docs/exec-plans/` structure already present in the repo.
- `2026-04-14 06:55Z` The current bash suite is intentionally tiny: `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/run.sh` plus a single scenario at `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/pet.sh`.
- `2026-04-14 06:58Z` Renaming the scenario with `apply_patch` preserved the file contents but dropped the executable bit, so the validation pass needed an explicit `chmod +x /Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-smoke-pet.sh`.
- `2026-04-14 06:58Z` A tier-aware runner did not require any heavy metadata format; a simple filename prefix plus `find ... -name 'req-*.sh'` kept the suite selection readable and stable.

## Decision Log

- `2026-04-14 06:55Z` Organize bash tests by workflow lane rather than by individual CLI command because the same commands recur across many real PTY use cases.
- `2026-04-14 06:55Z` Use filename prefixes for test tiers (`req-` and `opt-`) so CI selection stays simple and visible in the filesystem.

## Outcomes & Retrospective

This pass turned the bash suite from an implicit convention into an explicit one.

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/README.md` now explains the bash suite's role, the workflow-lane matrix, the required-vs-optional tier split, and the filename convention.
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/run.sh` now defaults to the required CI tier and supports `--tier all` for local expansion without changing CI behavior.
- The existing pet scenario was renamed to `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-smoke-pet.sh`, which makes the current suite match the documented naming rule.

The important result is not just the one existing scenario. The repository now has a stable place to add future bash coverage without debating structure every time. New scenarios can be planned by workflow lane and dropped into the required or optional tier immediately.

## Context and Orientation

Files expected to change in this pass:

- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/run.sh`
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/pet.sh` or its renamed replacement
- `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/README.md`
- `/Users/pedroproenca/Documents/Projects/agent-tui/docs/exec-plans/active/bash-cli-tests-matrix.md`

Terms used in this plan:

- `bash suite`: shell scripts under `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests` that invoke the built `agent-tui` binary and verify real Unix behavior.
- `workflow lane`: a user-visible behavior class such as lifecycle, viewer, editor, sessions, live preview, or failure handling.
- `required tier`: scenarios that must pass in CI and rely only on tools the CI image promises to provide.
- `optional tier`: scenarios that are useful locally but depend on extra tools that CI does not guarantee.

Current state:

- `just ready` already provides the single top-level CI command through `xtask ci`.
- `xtask ci` already invokes the bash suite via `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/run.sh`.
- The suite currently has one implemented required-tier smoke scenario plus a documented roadmap for additional lanes.

## Plan of Work

### Milestone 1: Define the matrix and conventions

Goal: explain exactly how bash scenarios should be added as the command surface expands.

Work: write a `cli-tests/README.md` that separates bash vs Rust responsibilities, enumerates workflow lanes, records the required/optional tier policy, and defines the filename convention.

Result: future contributors know where a new scenario belongs and whether it should run in CI.

Proof: the README contains a concrete matrix table, tier rules, and filename examples tied to the existing suite.

### Milestone 2: Make the runner enforce the convention

Goal: keep CI deterministic by having the suite runner select only required scenarios by default.

Work: update `cli-tests/run.sh` to understand the tier naming convention and rename the current pet scenario to match it.

Result: `xtask ci` runs only required scenarios unless explicitly asked for a wider tier.

Proof: `cli-tests/run.sh` reports the scenario count from the required tier and still supports local filtering.

### Milestone 3: Validate the integrated flow

Goal: prove the matrix and runner changes work in both the narrow and full repo paths.

Work: run the bash suite directly and then run `just ready`.

Result: the documented structure matches real executable behavior.

Proof: both commands pass and the plan is updated with exact validation results.

## Concrete Steps

1. Add `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/README.md` with the matrix, ownership split, and naming convention.
2. Rename the existing pet scenario to the chosen required-tier filename and update `/Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/run.sh` to select `req-*.sh` by default and optionally include `opt-*.sh`.
3. Run `bash -n` against the suite scripts, then `just cli-tests`, then `just ready`.
4. Update this plan with completed timestamps, discoveries, and outcomes.

## Validation and Acceptance

Validation commands:

1. `bash -n /Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/run.sh`
2. `bash -n /Users/pedroproenca/Documents/Projects/agent-tui/cli-tests/req-*.sh`
3. `cd /Users/pedroproenca/Documents/Projects/agent-tui && just cli-tests`
4. `cd /Users/pedroproenca/Documents/Projects/agent-tui && just ready`

Expected results:

- The README explains the bash suite in terms of workflow lanes rather than command-by-command enumeration.
- The suite runner defaults to the required tier.
- The current pet scenario follows the documented filename convention.
- The bash suite and full repo CI pass unchanged.

## Idempotence and Recovery

- Re-running the suite runner is safe; each scenario should create isolated daemon/runtime state and clean it up on exit.
- If renaming the current scenario causes any path drift, update only the runner and docs first, then re-run the narrow `just cli-tests` command before touching `xtask` or wider CI.
- If future optional scenarios need different tool dependencies, keep them behind the `opt-` prefix rather than weakening the required CI tier.
