# Test Plan Template

Use this file when writing step-by-step tests.

## Inputs to Collect
- Command and args to run.
- Terminal size (cols/rows) if layout-sensitive.
- Expected text or stability checkpoints.
- Credentials or fixtures (if needed).

## Step Structure
- Step 1: `screenshot` (or `screenshot --json`)
- Step 2: Action (`press`, `input`, `scroll`)
- Step 3: `wait --assert` for expected text or `wait --stable`
- Step 4: Repeat until done
- Step 5: Cleanup (`kill`)

## Example
1) `agent-tui run <app>`
2) `agent-tui --session <id> screenshot --format json`
3) `agent-tui --session <id> press Tab`
4) `agent-tui --session <id> input "value"`
5) `agent-tui --session <id> press Enter`
6) `agent-tui --session <id> wait "Success" --assert`
7) `agent-tui --session <id> kill`
