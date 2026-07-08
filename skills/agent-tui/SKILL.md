---
name: agent-tui
description: >
  Automate terminal UI (TUI) apps with agent-tui for testing, inspection, demos, and scripted interactions.
  Use when automating CLI/TUI flows, regression testing terminal apps, verifying interactive behavior, or extracting structured text from terminal UIs.
  Also use when asked what agent-tui is, how it works, or to demo it.
  Do not use for web browsers, GUI apps, or non-terminal interfaces.
---

# agent-tui

## Quick start
- Verify install: `agent-tui --version`
- Install with `npm i -g agent-tui` (or `pnpm add -g agent-tui`, `bun add -g agent-tui`).
- Alternate install: `curl -fsSL https://raw.githubusercontent.com/pproenca/agent-tui/master/install.sh -o /tmp/agent-tui-install.sh && sh /tmp/agent-tui-install.sh` or `cargo install --git https://github.com/pproenca/agent-tui.git --path cli/crates/agent-tui`.
- If you used the install script, ensure `~/.local/bin` is on your PATH.
- Start a session: `agent-tui run --format json <command> -- <args...>`
- Observe: `agent-tui screenshot --format json`
- Act: `agent-tui press Enter`, `agent-tui type "text"`, or `agent-tui scroll down`
- Wait or verify: `agent-tui wait "Expected text" --assert` or `agent-tui wait --stable`
- Cleanup: `agent-tui kill --yes`

## Core workflow
Current automation loop: `run -> screenshot -> press/type/scroll -> wait -> kill --yes`.

1. Run the app with `agent-tui run` and capture `session_id` from JSON output.
2. Take a fresh snapshot with `agent-tui screenshot` or `agent-tui screenshot --format json`.
3. Decide the next action based on the latest snapshot.
4. Act with `press`, `type`, or `scroll`.
5. Synchronize with `wait --assert` or `wait --stable`.
6. Repeat from step 2 until the task finishes.
7. Clean up with `agent-tui kill --yes`.

## Reliability rules
- Re-snapshot after every action that could change the UI.
- Never act on a changing screen; wait for stability first.
- Verify outcomes with `wait --assert` instead of assuming success.
- Always end automation runs with `kill --yes` or `sessions cleanup --yes`.

## Legacy migration
Use current commands for new scripts. Legacy compatibility forms still parse for old scripts, but each one writes its deprecation notice to stderr, keeps JSON stdout parseable, and is covered by the next-major removal window.

- `agent-tui input "text"` -> `agent-tui type "text"`.
- `agent-tui action <selector> click` -> `agent-tui press Enter`.
- `agent-tui action <selector> fill <text>` -> `agent-tui type "<text>"`.
- `agent-tui screenshot -e` -> `agent-tui screenshot`.
- `agent-tui screenshot -a` -> `agent-tui screenshot`.
- `agent-tui wait -e <ref>` -> `agent-tui wait "<text>"`.
- `agent-tui scroll-into-view <selector>` -> `agent-tui scroll <direction> [amount]` or `agent-tui press ...`; the compatibility command does not send terminal input.

## Session handling
- Use `--session <id>` for every command if more than one session exists.
- If you lose the session id, run `agent-tui sessions` and `agent-tui sessions show <id>`.

## Live preview (optional)
- Ask whether a live preview is desired.
- Start preview: `agent-tui live start --open`
- Stop standalone preview state with `agent-tui live stop`; daemon-backed preview remains available until `agent-tui daemon stop --yes`.

## Deep-dive references
- Full CLI coverage and options: `references/command-atlas.md`
- JSON output contract: `references/output-contract.md`
- End-to-end command sequences: `references/flows.md`
- Quick command selection: `references/decision-tree.md`
- Session lifecycle and concurrency: `references/session-lifecycle.md`
- Assertions and test oracles: `references/assertions.md`
- Failure recovery playbook: `references/recovery.md`
- Safety and confirmation prompts: `references/safety.md`
- Clarification checklist: `references/clarifications.md`
- Test plan template: `references/test-plan.md`
- Demo script: `references/demo.md`
- User prompt templates: `references/prompt-templates.md`
- Minimal command sets by use case: `references/use-cases.md`
- Explorer/replay automation skill: `../tui-explorer/SKILL.md`
