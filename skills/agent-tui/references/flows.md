# Full Flows and Command Sequences

Use this file when you need a complete, end-to-end command sequence.

## Standard CLI Regression Test (human-like flow)
1) Start the app under test:
   - `agent-tui run <your-cli> -- <args>`
2) Capture session id from JSON output (use `--session <id>` for the rest).
3) First snapshot (elements):
   - `agent-tui --session <id> screenshot -e --format json`
4) Act based on elements/text:
   - `agent-tui --session <id> action @e1 click`
   - `agent-tui --session <id> action @e2 fill "value"`
   - `agent-tui --session <id> press Enter`
5) Wait for expected state:
   - `agent-tui --session <id> wait "Expected text" --assert`
6) Repeat steps 3-5 until the flow finishes.
7) Cleanup:
   - `agent-tui --session <id> kill`

## Form Interaction Flow
1) `agent-tui run <app>`
2) `agent-tui --session <id> screenshot -e --format json`
3) Fill input: `agent-tui --session <id> action @inp1 fill "my-value"`
4) Click submit: `agent-tui --session <id> action @btn1 click`
5) Wait for success: `agent-tui --session <id> wait "Success" --assert`
6) Cleanup: `agent-tui --session <id> kill`

## Dynamic UI / Flaky Rendering Flow
1) Start: `agent-tui run <app>`
2) Stabilize: `agent-tui --session <id> wait --stable`
3) Snapshot: `agent-tui --session <id> screenshot -e --format json`
4) Act: `agent-tui --session <id> action @e1 click` or `agent-tui --session <id> press Enter`
5) Re-stabilize: `agent-tui --session <id> wait --stable`
6) Re-snapshot and continue.
7) Cleanup: `agent-tui --session <id> kill`

## Live Preview Flow (optional)
1) Ask user if they want live preview.
2) If yes: `agent-tui live start --open`
3) Run the app: `agent-tui run <app>`
4) Continue normal flow (snapshot/act/wait).
5) Stop preview when done: `agent-tui live stop`
6) Cleanup: `agent-tui kill`

## Debug/Attach Flow (existing session)
1) List sessions: `agent-tui sessions`
2) Attach to active session: `agent-tui sessions attach`
3) Interact/observe, then detach (Ctrl-P Ctrl-Q).
4) Cleanup: `agent-tui kill` or `agent-tui sessions cleanup`

## Smoke Test Example (htop)
1) `agent-tui run htop`
2) `agent-tui --session <id> screenshot -e --format json`
3) Verify UI text (e.g., "F1 Help").
4) Quit: `agent-tui --session <id> press F10`
5) Confirm quit: `agent-tui --session <id> wait "Quit" --gone`
6) Cleanup: `agent-tui --session <id> kill`
