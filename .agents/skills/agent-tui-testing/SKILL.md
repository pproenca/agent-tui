---
name: agent-tui-testing
description: Use agent-tui to run, inspect, and drive TUI apps with snapshots, selectors, waits, and cleanup. Designed for agent-driven testing (e.g., "test htop").
---

# Agent TUI Testing Skill

## Goals
- Start a TUI app under agent control.
- Capture snapshots and detect elements.
- Plan interactions based on what is on screen.
- Execute key presses and element actions.
- Wait for specific conditions.
- Cleanly exit and kill the session.

## Prereqs
- Install `agent-tui` globally: `npm i -g agent-tui` or `pnpm i -g agent-tui`.
- Verify on PATH: `agent-tui --version`.
- Ensure daemon running. If unsure, run `agent-tui daemon start` or use `agent-tui run ...` which auto-starts.
- Prefer `--format json` (or `--json`) for automation.

## Core Workflow (Agent Loop)
1) Start: `agent-tui run <command> [-- args...]`
2) Snapshot: `agent-tui screenshot -e --format json`
3) Plan: decide next action based on elements/text.
4) Act: `agent-tui action @e1 click` / `agent-tui press Enter` / `agent-tui input "text"`
5) Wait: `agent-tui wait "text"` or `agent-tui wait --stable` or `agent-tui wait -e @e2`
6) Repeat steps 2-5 until done.
7) Exit: `agent-tui press F10` (or app-specific quit) then `agent-tui kill`.

## Command Cheat Sheet
- Run app: `agent-tui run htop`
- Screenshot: `agent-tui screenshot` (text) or `agent-tui screenshot -e --json` (elements)
- Accessibility tree: `agent-tui screenshot -a --interactive-only`
- Click element: `agent-tui action @e1 click`
- Fill input: `agent-tui action @e1 fill "value"`
- Press keys: `agent-tui press Ctrl+C` or `agent-tui press ArrowDown ArrowDown Enter`
- Unified input: `agent-tui input Enter` or `agent-tui input "hello"`
- Wait: `agent-tui wait "Ready"` or `agent-tui wait -e @e1 --gone`
- Sessions: `agent-tui sessions`, `agent-tui sessions attach`, `agent-tui sessions cleanup`
- Live preview: `agent-tui live start --open`
- Kill session: `agent-tui kill`

## Output and Parsing Guidance
- Use `--format json` when an agent is parsing output.
- In text mode, data is human-friendly; in JSON mode, use machine parsing.
- For waits with `--assert`, timeouts return exit code 1; otherwise waits always exit 0.

## Element Selectors
- Element refs: `@e1`, `@btn2`, `@inp3` (from `screenshot -e`).
- Exact text: `@Submit` or `@"Submit"` (quote to include spaces).
- Partial text: `:Submit` (contains match).

## Example: Test htop
1) Start: `agent-tui run htop`
2) Snapshot: `agent-tui screenshot -e --format json`
3) Verify: look for "F1 Help" or "F10 Quit" text in screenshot.
4) Interact: `agent-tui press F10` to quit.
5) Confirm: `agent-tui wait "Quit" --gone` or `agent-tui wait --stable`.
6) Cleanup: `agent-tui kill`.

## Example: Form Interaction (generic)
1) `agent-tui run <app>`
2) `agent-tui screenshot -e --format json`
3) If input found: `agent-tui action @e2 fill "my-value"`
4) If button found: `agent-tui action @btn1 click`
5) `agent-tui wait "Success" --assert`
6) `agent-tui kill`

## Error Handling
- If daemon not running: start it (`agent-tui daemon start`) or re-run with `agent-tui run ...`.
- If element not found: take a new snapshot and re-select.
- If UI is dynamic: use `agent-tui wait --stable` before acting.

## Live Debugging
- `agent-tui live start --open` to open the live preview UI.
- `agent-tui sessions attach` for interactive live stream (detach with Ctrl-P Ctrl-Q).

## Cleanup
- Always end with `agent-tui kill` or `agent-tui sessions cleanup`.
- If stuck, `agent-tui daemon stop --force` is the last resort.
