# Replay

Run:
- `skills/tui-explorer/scripts/tui_explorer verify --spec "<acceptance.md>"`

## Execution model
- Spawn a fresh session per scenario.
- Execute each step in order.
- Use `wait --assert` for `expect` steps.
- Stop on first scenario failure (fail-fast).

## Result contract
- Exit `0`: all scenarios pass.
- Exit `1`: scenario/assertion failure.
- Exit `2`: spec validation error.
- Exit `69`: agent-tui/daemon unavailable.

## Failure artifacts
- `verify-report.json`
- `failures/<scenario>-step-<n>.txt`
