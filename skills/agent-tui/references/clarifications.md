# Clarification Checklist

Use this file when requirements are incomplete or ambiguous.

## Ask Before Running
- App command, args, and working directory (what to run, where).
- Required environment variables or config files.
- Expected UI outcomes (texts, states, or tree hints to assert).
- Inputs and sequencing (keys, text, timing constraints).
- Stop condition (what indicates the test is done).
- Terminal size requirements (cols/rows) if layout-sensitive.
- Output format preference: JSON for automation or text for readability.

## Live Preview (always ask)
- "Do you want a live preview while I run the test?"
- If yes: run `agent-tui live start --open` before `agent-tui run ...` and stop it afterward.

## Safety and Data Handling
- Ask for credentials or test accounts if login is required; never guess secrets.
- Confirm if it is safe to execute actions that modify data (delete, submit, confirm).
- If the app can perform destructive actions, request a dry-run or a test environment.

## Session Scope
- Ask whether multiple sessions will be running; if yes, plan to use `--session <id>`.
