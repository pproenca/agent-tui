---
name: agent-tui
description: Automate, test, and inspect terminal UI (TUI) apps using agent-tui (CLI/daemon). Use when an agent must drive a TUI program end-to-end, verify interactive behavior, or parse terminal UI state over time (screenshots, elements, waits, inputs, sessions, live preview, find/count, resize, restart).
---

# Agent TUI Skill

## Goals
- Launch a TUI app under agent control.
- Capture snapshots and discover elements.
- Decide actions based on on-screen state.
- Send key presses or element actions.
- Wait for specific conditions.
- Exit cleanly and clean up sessions.

## Quality-First Defaults
- Prefer `--format json` (or `--json`) for deterministic parsing.
- Treat every action as invalidating selectors; re-snapshot after each UI change.
- Convert every requirement into an explicit wait/assertion.
- Capture the `session_id` from `run` and use `--session <id>` when multiple sessions exist.
- Ask whether the user wants live preview before starting a run.

## Prereqs
- Install globally: `npm i -g agent-tui` or `pnpm i -g agent-tui`.
- Verify on PATH: `agent-tui --version`.
- Ensure daemon running. If unsure, run `agent-tui daemon start` or use `agent-tui run ...` which auto-starts.

## Mental Model (Work Backwards from the CLI)
- `agent-tui run`: spawn and control a TUI process; creates a session.
- `agent-tui screenshot`: observe UI state (text, elements, or accessibility tree).
- `agent-tui action` / `agent-tui press` / `agent-tui input`: act like a human at the keyboard/mouse.
- `agent-tui wait`: synchronize with UI changes and verify conditions.
- `agent-tui find` / `agent-tui count`: query element sets by role/name/text.
- `agent-tui sessions`: inspect/attach/clean up sessions for debugging.
- `agent-tui live`: stream the UI for live preview during development.
- `agent-tui kill`: end sessions and clean up.
- `agent-tui daemon`: manage the background process if it is stuck or out of date.

## Command Atlas (Summary)
- Spawn: `run <command> [-- args...]` (use `-d <dir>` and `--cols/--rows`).
- Observe: `screenshot`, `screenshot -e --json`, `screenshot -a --interactive-only`.
- Query: `find --role/--name/--text`, `count --role/--name/--text`.
- Act: `action @ref click|dblclick|fill|select|toggle|focus|clear|selectall|scroll`.
- Keys: `press <keys...>`, `type "text"`, `input <key|text> [--hold|--release]`.
- Sync: `wait <text>` / `wait -e @ref` / `wait --stable` / `wait --assert`.
- Viewport: `scroll-into-view @ref`, `resize --cols --rows`.
- Sessions: `sessions`, `sessions show <id>`, `sessions attach [id]`, `sessions cleanup`.
- Live preview: `live start --open`, `live status`, `live stop`.
- Daemon: `daemon start|stop|status|restart`.
- Debug: `env`, `version`.
- Shorthand selectors: `agent-tui @e1`, `agent-tui :Submit`, `agent-tui '@"Exact Text"'`.

## Global Flags and Defaults
- `--session <id>`: target a specific session (defaults to most recent session).
- `--format <text|json>` / `--json`: output control for automation.
- `--no-color`: disable colored output (also respects `NO_COLOR`).
- `--verbose`: include request timing.
- Defaults: terminal size 120x40; wait timeout 30000ms.
- Limits: terminal size clamps to 10x5 min and 500x200 max.

## Observe -> Decide -> Act (Decision Guide)
- Use `screenshot` (text) when assertions are pure text checks.
- Use `screenshot -e --json` when you need element refs for actions.
- Use `screenshot -a --interactive-only` for focus and accessibility checks.
- Use `find`/`count` when you need targeted lookup (role/name/text) without parsing full screenshots.
- After any action, re-snapshot; do not reuse stale refs.
- If the UI is moving, use `wait --stable` before acting.
- If elements are off-screen, use `scroll-into-view @ref`.

## Session Lifecycle and Hygiene
- `run` returns a new session; capture `session_id` from JSON output.
- Use `--session <id>` for every follow-up command when multiple sessions exist.
- Use `sessions` to list active sessions and `sessions show` for details.
- Use `sessions attach` for interactive debugging (detach with Ctrl-P Ctrl-Q).
- Use `restart` to restart the current session command.
- Use `kill` to terminate the current session; use `sessions cleanup` for orphans.
- Use `daemon restart` only when necessary (terminates all sessions).

## Clarify Before Running (ask if missing)
- App under test: command, args, working dir, and required env vars.
- Expected outcomes: text/element states to assert; success criteria.
- Inputs: keystrokes/text, order, and timing constraints.
- Data safety: confirm whether it is safe to submit, delete, or modify data.
- Auth: request test credentials or fixtures if login is required.
- Live view: ask if the user wants real-time preview (`agent-tui live start --open`).
- Use the prompt templates in `references/prompt-templates.md` to ask concisely.

## Live Preview Policy
- Ask: "Do you want a live preview while I run the test?"
- If yes, start preview before the run: `agent-tui live start --open`.
- Stop preview when done: `agent-tui live stop`.

## Core Workflow (Agent Loop)
1) Start: `agent-tui run <command> [-- args...]`
2) Snapshot: `agent-tui screenshot -e --format json`
3) Plan: decide next action based on elements/text.
4) Act: `agent-tui action @e1 click` / `agent-tui press Enter` / `agent-tui input "text"`
5) Wait: `agent-tui wait "text"` or `agent-tui wait --stable` or `agent-tui wait -e @e2 --assert`
6) Repeat steps 2-5 until done.
7) Exit: quit the app (e.g., `agent-tui press F10`) then `agent-tui kill`.

## Problem -> Command Mapping
- "Start the app under test": `agent-tui run <command> [-- args...]`.
- "See what's on screen": `agent-tui screenshot` (text) or `agent-tui screenshot -e --json` (elements).
- "Find and click/fill something": `agent-tui action @e1 click` / `agent-tui action @e1 fill "value"`.
- "Send keys like a human": `agent-tui press Enter` / `agent-tui press Ctrl+C` / `agent-tui input "text"`.
- "Wait for a UI state": `agent-tui wait "Ready" --assert` / `agent-tui wait -e @e1 --gone` / `agent-tui wait --stable`.
- "Count or locate elements": `agent-tui find --role button` / `agent-tui count --text "Error"`.
- "Element is off-screen": `agent-tui scroll-into-view @e1`.
- "Layout depends on size": `agent-tui resize --cols 120 --rows 40`.
- "Inspect environment or version": `agent-tui env` / `agent-tui version`.
- "Debug daemon": `agent-tui daemon status` / `agent-tui daemon restart`.
- "Finish and clean up": `agent-tui kill` or `agent-tui sessions cleanup`.

## Command Cheat Sheet
- Run app: `agent-tui run htop`
- Screenshot: `agent-tui screenshot` (text) or `agent-tui screenshot -e --json` (elements)
- Accessibility tree: `agent-tui screenshot -a --interactive-only`
- Find elements: `agent-tui find --role button --name "OK"`
- Count elements: `agent-tui count --text "Error"`
- Click element: `agent-tui action @e1 click`
- Fill input: `agent-tui action @e1 fill "value"`
- Press keys: `agent-tui press Ctrl+C` or `agent-tui press ArrowDown ArrowDown Enter`
- Unified input: `agent-tui input Enter` or `agent-tui input "hello"`
- Wait: `agent-tui wait "Ready" --assert` or `agent-tui wait -e @e1 --gone`
- Sessions: `agent-tui sessions`, `agent-tui sessions attach`, `agent-tui sessions cleanup`
- Live preview: `agent-tui live start --open`
- Kill session: `agent-tui kill`

## Output and Parsing Guidance
- Use `--format json` for automation.
- Re-snapshot after actions; element refs are not stable across UI changes.
- Use `wait --assert` for pass/fail semantics; timeouts return exit code 1.
- Use `wait --stable` before acting on dynamic UI.

## Output Contract (JSON)
- `run` returns: `{ "session_id": "...", "pid": 123 }`.
- `screenshot` returns: `{ "session_id": "...", "screenshot": "...", "elements": [...], "cursor": {"row":0,"col":0,"visible":true} }` (elements/cursor optional).
- `elements[]` items include: `ref`, `type`, `label`, `value`, `position {row,col,width,height}`, `focused`, `selected`, `checked`, `disabled`, `hint`.
- `find` returns: `{ "elements": [...], "count": N }`.
- `count` returns: `{ "count": N }`.
- `wait` returns: `{ "found": true|false, "elapsed_ms": N }`.
- `sessions` returns: `{ "sessions": [{"id":"...","command":"...","pid":123,"running":true,"created_at":"...","size":{"cols":120,"rows":40}}], "active_session":"..." }`.

## Element Selectors
- Element refs: `@e1`, `@btn2`, `@inp3` (from `screenshot -e`).
- Exact text: `@Submit` or `@"Submit"` (quote to include spaces).
- Partial text: `:Submit` (contains match).
- Shorthand actions: `agent-tui @e1` (click), `agent-tui :Submit` (click), `agent-tui @e1 fill "value"`.

## Full Flows (reference quality)
### CLI Regression Test (human-like)
Use this to validate your CLI's inputs, rendering, and flow end-to-end.
1) Start the app: `agent-tui run <your-cli> -- <args>`
2) Snapshot elements: `agent-tui screenshot -e --format json`
3) Act: `agent-tui action @e1 click` / `agent-tui action @e2 fill "value"` / `agent-tui press Enter`
4) Wait for expectations: `agent-tui wait "Expected text" --assert`
5) Re-snapshot and continue until done.
6) Cleanup: `agent-tui kill`

### Form Interaction Flow
1) `agent-tui run <app>`
2) `agent-tui screenshot -e --format json`
3) Fill: `agent-tui action @inp1 fill "my-value"`
4) Submit: `agent-tui action @btn1 click`
5) Wait: `agent-tui wait "Success" --assert`
6) Cleanup: `agent-tui kill`

### Dynamic/Flaky UI Flow
1) `agent-tui run <app>`
2) Stabilize: `agent-tui wait --stable`
3) Snapshot: `agent-tui screenshot -e --format json`
4) Act: `agent-tui action @e1 click` or `agent-tui press Enter`
5) Re-stabilize: `agent-tui wait --stable`
6) Re-snapshot and continue.
7) Cleanup: `agent-tui kill`

### Live Preview Flow (optional)
1) Ask whether live preview is desired.
2) If yes: `agent-tui live start --open`
3) Run: `agent-tui run <app>`
4) Continue normal flow (snapshot/act/wait).
5) Stop preview: `agent-tui live stop`
6) Cleanup: `agent-tui kill`

## Use Cases (when to use what)
1) Regression test a CLI/TUI you are building:
   - `run` -> `screenshot -e --json` -> `action/press/input` -> `wait --assert` -> `kill`.
2) Drive an interactive wizard or form:
   - `run` -> `screenshot -e --json` -> `action @inp fill` -> `action @btn click` -> `wait`.
3) Validate rendering/layout (snapshot audit):
   - `run` -> `screenshot` (text) -> `wait "Expected" --assert`.
4) Investigate flaky UI or race conditions:
   - `wait --stable` -> `screenshot -e --json` -> `wait -e @spinner --gone`.
5) Accessibility tree checks:
   - `screenshot -a --interactive-only` to inspect focusable/interactive elements.
6) Live observation/debugging:
   - `live start --open` or `sessions attach` to watch a run in real time.
7) TUI exploration / reverse engineering:
   - `screenshot -e --json`, probe with `action/press/input`, re-snapshot each step.

## Failure Recovery Playbook
- If an element is not found: take a new `screenshot -e`, then re-select.
- If an element exists but is off-screen: `scroll-into-view @ref` and re-snapshot.
- If waits time out: increase timeout, use `wait --stable`, then re-snapshot.
- If the daemon is not running: `agent-tui daemon start`.
- If CLI/daemon versions mismatch: `agent-tui daemon restart`.
- If the session is stuck: `agent-tui kill`, then re-run.
- If layout is wrong: `agent-tui resize --cols --rows` and re-snapshot.
- After 3-5 failed loops, stop and ask the user for guidance.

## Agent Prompt Templates (ready to use)
### Quick Clarify (first response)
Please share: (1) command + args to run, (2) expected UI text/state to assert, (3) inputs/steps, (4) any env vars, and (5) whether you want live preview while I run it.

### Live Preview Check
Do you want a live preview while I run the test? If yes, I'll start `agent-tui live start --open` before running the app.

### Safety Confirmation
Is this a test environment, and is it safe to submit or modify data during this run?

### Auth Request
Does the flow require login? If so, please provide test credentials or a fixture account.

### Results Summary (after run)
I ran the flow, executed these actions: <actions>, and asserted: <assertions>. The last snapshot shows: <evidence>. Want me to extend coverage or add more assertions?

## Default Test Plan Generator (use when requirements are vague)
1) Restate the goal in one sentence.
2) Identify entry command and environment.
3) Define assertions (text/element/value/focus).
4) Define inputs and navigation steps.
5) Choose observation mode (text vs elements vs accessibility).
6) Define synchronization points (`wait --assert`, `wait --stable`).
7) Set cleanup and stop conditions (`kill`, `sessions cleanup`).
8) Ask for missing details, then produce an executable command plan.

Use `references/test-plan.md` for a fill-in template.
## Progressive Disclosure References
- Full command atlas, options, and env vars: `references/command-atlas.md`.
- Decision tree (observe vs act) and selector stability: `references/decision-tree.md`.
- Session lifecycle and concurrency: `references/session-lifecycle.md`.
- Assertions and test oracles: `references/assertions.md`.
- Failure recovery and timeouts: `references/recovery.md`.
- JSON output contract and parsing tips: `references/output-contract.md`.
- Safety and confirmation prompts: `references/safety.md`.
- Full flows and command sequences: `references/flows.md`.
- Clarification prompts and checklists: `references/clarifications.md`.
- Problem-driven use cases: `references/use-cases.md`.
- Prompt templates for user communication: `references/prompt-templates.md`.
- Test plan template: `references/test-plan.md`.

## Cleanup
- Always end with `agent-tui kill` or `agent-tui sessions cleanup`.
- Use `agent-tui daemon stop --force` only as a last resort.
