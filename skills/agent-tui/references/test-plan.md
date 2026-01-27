# Default Test Plan Template

Use this template to turn vague requirements into an executable plan.

## 1) Goal
- One sentence goal:
  - Example: "Verify the CLI wizard accepts input, renders the summary, and exits cleanly."

## 2) Entry Command
- Command: `<command>`
- Args: `<args>`
- Working dir: `<dir>`
- Env vars: `<KEY=VALUE>`

## 3) Expected Outcomes (Assertions)
- Text assertions:
  - "<expected text>"
- Element assertions:
  - `@ref` or `find --role/--name/--text`
- Value assertions:
  - `wait --value @ref=VALUE`
- Focus assertions:
  - `wait --focused @ref`

## 4) Inputs / Steps
- Step 1: `<action>` (press/input/action)
- Step 2: `<action>`
- Step 3: `<action>`

## 5) Observation Mode
- Choose one:
  - `screenshot` (text)
  - `screenshot -e --json` (elements)
  - `screenshot -a --interactive-only` (accessibility)

## 6) Synchronization
- `wait --assert` conditions
- `wait --stable` before/after actions if UI is dynamic
- Timeouts (ms): `<timeout>`

## 7) Live Preview
- Live preview? yes/no
- If yes: `live start --open`, stop with `live stop`

## 8) Cleanup
- `kill` or `sessions cleanup`

## 9) Command Plan (fill-in)
1) `agent-tui run <command> -- <args>`
2) `agent-tui --session <id> screenshot -e --format json`
3) `agent-tui --session <id> <action>`
4) `agent-tui --session <id> wait "<expected>" --assert`
5) Repeat steps 2-4 as needed.
6) `agent-tui --session <id> kill`
