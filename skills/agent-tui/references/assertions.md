# Assertions and Test Oracles

Use this file when translating requirements into checks.

## Turn Requirements into Assertions
- Convert each requirement into observable UI state (text, element presence, focus, or value).
- Prefer `wait --assert` for pass/fail semantics.
- Collect evidence with a final `screenshot` before cleanup.

## Assertion Patterns
- Text presence: `agent-tui wait "Expected" --assert`
- Element present: `agent-tui wait -e @ref --assert`
- Element gone: `agent-tui wait -e @ref --gone --assert`
- Focused element: `agent-tui wait --focused @ref --assert`
- Input value: `agent-tui wait --value @inp1=VALUE --assert`

## Validation Strategy
- For static UI: text snapshot + `wait` is enough.
- For interactive UI: element snapshot + action + wait, repeating after each step.
- For dynamic UI: insert `wait --stable` before each action.

## Reporting
- Report: scenario, actions taken, assertions run, and final evidence (screenshot output).
- If an assertion fails, include the last screenshot and the expected condition.
