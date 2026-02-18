---
name: tui-explorer
description: >
  Discover and replay terminal UI paths using agent-tui.
  Use when you need to launch a TUI app, explore navigation with bounded BFS,
  generate markdown acceptance tests, and verify those tests repeatedly.
  Do not use for web or desktop GUI automation.
---

# tui-explorer

## Purpose
Generate replayable acceptance tests from discovered TUI navigation paths.

## Commands
- Discover:
  - `skills/tui-explorer/scripts/tui_explorer discover --command "<app command>"`
- Verify:
  - `skills/tui-explorer/scripts/tui_explorer verify --spec "<path-to-acceptance.md>"`

## Defaults
- Exploration strategy: bounded BFS.
- Safe action set: `Enter`, `Tab`, `ArrowDown`, `ArrowUp`, `ArrowRight`, `ArrowLeft`, `Esc`, `Space`.
- Risky actions are disabled unless `--allow-risky` is set.
- Output directory defaults to `.agent-tui/discover/<timestamp>/`.
- Replay fails on scenario failure and exits non-zero.

## Workflow
1. Start live preview over the HTTP endpoint with `agent-tui live start --open` before discovery so the user can watch exploration in the built-in web UI.
2. Run `discover` for the target command.
3. Inspect generated artifacts:
   - `acceptance.md` (human-readable OpenSpec-style expectations + machine-executable steps)
   - `trace.jsonl`
   - `discover-report.json`
4. Run `verify` against the generated `acceptance.md`.
5. Stop live preview with `agent-tui live stop` after verification.

Session handling for live preview:
- Treat browser session selection as preview-local context only.
- Do not use browser interactions to switch the daemon active session.
6. On failure, inspect `verify-report.json` and files under `failures/`.

## References
- Schema details: `references/schema-v1.md`
- Discovery mechanics: `references/discovery.md`
- Replay semantics: `references/replay.md`
