# Assertions and Test Oracles

Use this file when translating requirements into checks.

## Turn Requirements into Assertions
- Convert each requirement into observable UI state (text or stability).
- Prefer `wait --assert` for pass/fail semantics.
- Collect evidence with a final `screenshot` before cleanup.

## Assertion Patterns
- Text presence: `agent-tui wait "Expected" --assert`
- Text gone: `agent-tui wait "Expected" --gone --assert`
- Stability: `agent-tui wait --stable --assert`

## Validation Strategy
- For static UI: text snapshot + `wait` is enough.
- For dynamic UI: insert `wait --stable` before each action.

## Reporting
- Report: scenario, actions taken, assertions run, and final evidence (screenshot output).
- If an assertion fails, include the last screenshot and the expected condition.
