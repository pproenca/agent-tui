# Prompt Templates

Use these verbatim when you need to ask the user for missing details.

## Minimum Info Request
- "Please share: (1) command + args to run, (2) expected UI text/state to assert, (3) inputs/steps, (4) any env vars, and (5) whether you want live preview while I run it."

## Live Preview
- "Do you want a live preview while I run the test? If yes, I'll start `agent-tui live start --open` before running the app."

## Safety / Environment
- "Is this a test environment, and is it safe to submit or modify data during this run?"

## Credentials
- "Does the flow require login? If so, please provide test credentials or a fixture account."

## Clarify Assertions
- "What exact text or UI state should I treat as success?"
- "Are there any error states I should explicitly check for?"

## Timeouts and Stability
- "Should I use a longer timeout or wait for stability (`wait --stable`) before actions?"

## Completion Check
- "What should indicate the test is finished (specific screen, message, or exit state)?"

## Results Summary Template
- "I ran the flow, executed these actions: <actions>, and asserted: <assertions>. The last snapshot shows: <evidence>. Want me to extend coverage or add more assertions?"
