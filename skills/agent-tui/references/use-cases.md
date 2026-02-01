# Use Cases

Use this file when selecting a minimal command set for a task.

## Basic Automation Loop
- Commands: `run` -> `screenshot` -> `press/input/scroll` -> `wait --assert` -> `kill`.
- Re-snapshot after each action.

## Form Entry
- Commands: `run`, `screenshot --json`, `press Tab`, `input "value"`, `press Enter`, `wait "Success" --assert`.

## Stabilization Before Acting
- Commands: `wait --stable`, `screenshot`, then `press/input`.

## Live Preview Support
- Commands: `live start --open` -> `run` -> normal flow -> `live stop`.
