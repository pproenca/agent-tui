# Bash CLI Test Suite

`cli-tests/` is the real-world bash test suite for the built `agent-tui` binary.

The suite exists for behavior that Rust unit and contract tests cannot prove well:

- PTY lifecycle with real Unix processes
- screen capture and wait behavior against real terminal output
- daemon and live-preview state on real sockets
- multi-session coordination with actual running programs

## One CI Command

The single top-level CI command remains:

- `just ready`

That command runs `xtask ci`, and `xtask ci` runs:

- Rust tests for parser/help/output contracts
- the bash suite in `cli-tests/` for real Unix workflows

For local iteration on just the shell suite:

- `just cli-tests`
- `cli-tests/run.sh`

## What Belongs Here

Use Rust tests when the change is about:

- command parsing
- help text
- JSON/text output shape
- invalid flag handling
- pure business logic that does not need a real PTY or daemon

Use bash tests when the change is about:

- spawning real programs
- driving terminal input/output
- screenshot fidelity against a live screen buffer
- daemon lifecycle and socket-backed state
- session switching across real running processes

## Naming Convention

Every scenario file uses:

- `{tier}-{lane}-{scenario}.sh`

Rules:

- `tier` is `req` or `opt`
- `lane` is the primary workflow class being proven
- `scenario` is a short concrete name
- use lowercase kebab-case only

Examples:

- `req-smoke-pet.sh`
- `req-viewer-top.sh`
- `req-editor-vim.sh`
- `opt-viewer-htop.sh`

## Tiers

- `req-*`
  CI-required scenarios. These must pass in the standard repo environment and rely only on tools the project is willing to guarantee for CI.
- `opt-*`
  Local-only or extended scenarios. These are useful for richer manual coverage but depend on extra tools that CI does not promise.

`cli-tests/run.sh` runs `req-*` by default. Use `--tier all` to include `opt-*`.

## Workflow Lanes

Plan bash scenarios by user-visible workflow lane, not by subcommand count.

| Lane | What it proves | Typical `agent-tui` commands | Example real tool | Default tier | Status |
| --- | --- | --- | --- | --- | --- |
| `smoke` | One short happy path across multiple core capabilities | `run`, `wait`, `screenshot`, `type`, `press`, `kill` | `top` + `vim` | `req` | Implemented as `req-smoke-pet.sh` |
| `lifecycle` | Spawn, inspect, restart, kill, cleanup | `run`, `restart`, `kill`, `sessions cleanup` | `sh`, `tail -f` | `req` | Planned |
| `viewer` | Read-only terminal observation and waiting for terminal output | `run`, `screenshot`, `wait` | `top` | `req` | Implemented as `req-viewer-top.sh` |
| `editor` | Interactive text editing, key semantics, and resize stability | `run`, `type`, `press`, `wait`, `resize` | `vim` | `req` | Implemented as `req-editor-vim.sh` |
| `sessions` | Two or more sessions with switching and inspection | `sessions list`, `show`, `switch`, `screenshot` | `sh` | `req` | Implemented as `req-sessions-switch.sh` |
| `live` | Local daemon-backed UI and WS preview state | `daemon *`, `live *` | built-in `/ui` | `req` | Planned |
| `failures` | Non-happy-path behavior and recovery | `wait`, `kill`, `sessions cleanup`, `--no-input` | `sh` | `req` | Planned |
| `viewer` extras | Richer TUI coverage with non-guaranteed tools | same as `viewer` | `htop`, `lazygit` | `opt` | Planned |

## Scenario Intake Rule

Add or extend a bash scenario only when the feature changes real runtime behavior.

Questions to ask:

1. Does this need a real PTY, daemon, or live process to be credible?
2. Is the behavior already covered by an existing lane?
3. Can the new case fit one scenario without turning that scenario into a kitchen sink?

If the answer to `1` is no, keep it in Rust tests.

If the answer to `2` is yes, extend the existing scenario rather than adding a near-duplicate.

If the answer to `3` is no, add a new scenario in the right lane.

## Current and Planned Scenario Set

- `req-smoke-pet.sh`
  Short end-to-end happy path. Starts the daemon, samples `top`, enters text in `vim`, asserts screen state, and cleans up.
- `req-lifecycle-shell.sh`
  Planned. Focus on shell process creation, restart, kill, and cleanup.
- `req-viewer-top.sh`
  Implemented. Focus on `top` output visibility through `wait` and `screenshot`.
- `req-editor-vim.sh`
  Implemented. Focus on typing, keys, waits, and resize behavior in `vim`.
- `req-sessions-switch.sh`
  Implemented. Focus on multi-session inventory, `sessions show`, and active-session switching.
- `req-live-preview.sh`
  Planned. Focus on daemon status and live-preview discovery.
- `req-failures-basic.sh`
  Planned. Focus on missing session, timeout, and no-input behavior.
- `opt-viewer-htop.sh`
  Planned. Local-only richer viewer coverage if `htop` is installed.

## Authoring Rules

- Each scenario must be self-cleaning and safe to re-run.
- Each scenario must use isolated runtime paths.
- Each scenario must target the built binary through `AGENT_TUI_BIN`.
- Keep the actual `agent-tui` command sequence inline in the scenario script; helpers should not hide behavioral steps like `run`, `wait`, `screenshot`, `resize`, `switch`, or `kill`.
- Each scenario should prove one lane cleanly; only `smoke` scenarios should intentionally span multiple lanes.
- Each scenario should leave behind artifacts only when explicitly asked.
