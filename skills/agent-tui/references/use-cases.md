# Use Cases -> Command Strategy

Use this file when deciding which commands solve the user's problem.

## 1) Regression test a CLI/TUI you are building
- Goal: verify inputs, rendering, and flow end-to-end.
- Commands: `run` -> `screenshot -e --json` -> `action/press/input` -> `wait --assert` -> `kill`.
- Use `--format json` and explicit waits for deterministic tests.

## 2) Drive an interactive wizard or form
- Goal: fill fields and submit like a user.
- Commands: `run`, `screenshot -e --json`, `action @inp fill`, `action @btn click`, `wait`.
- Re-snapshot after each step to re-map selectors.

## 3) Validate rendering/layout (snapshot auditing)
- Goal: ensure specific strings, labels, or sections are present.
- Commands: `run`, `screenshot` (text), `wait "Expected" --assert`.
- Prefer text snapshots if element selectors are unnecessary.

## 4) Investigate flaky UI or race conditions
- Goal: synchronize with dynamic screens.
- Commands: `wait --stable`, `screenshot -e --json`, `wait -e @spinner --gone`.
- Repeat snapshots after any action.

## 5) Accessibility tree checks
- Goal: inspect focusable/interactive elements.
- Commands: `screenshot -a --interactive-only`.
- Use for verifying focus order and interactive roles.

## 6) Live observation/debugging
- Goal: watch the run in real time or attach to an existing session.
- Commands: `live start --open`, `sessions attach`, `sessions`.
- Always ask the user whether they want live preview.

## 7) TUI exploration / reverse engineering
- Goal: discover controls and available actions.
- Commands: `screenshot -e --json`, then probe with `action`, `press`, `input`.
- Capture another snapshot after each change.

## 8) Element presence/absence checks
- Goal: quickly confirm presence or count without full parsing.
- Commands: `find --role/--name/--text`, `count --text`.
