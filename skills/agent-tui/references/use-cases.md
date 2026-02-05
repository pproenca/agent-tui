# Use Cases

Use this file when selecting a minimal command set for a task.

## Basic Automation Loop
- Commands: `run` -> `screenshot` -> `press/type` -> `wait --assert` -> `kill`.
- Re-snapshot after each action.

## Form Entry
- Commands: `run`, `screenshot --json`, `press Tab`, `type "value"`, `press Enter`, `wait "Success" --assert`.

## Stabilization Before Acting
- Commands: `wait --stable`, `screenshot`, then `press/type`.

## Live Preview Support
- Commands: `live start --open` -> `run` -> normal flow -> `live stop`.
