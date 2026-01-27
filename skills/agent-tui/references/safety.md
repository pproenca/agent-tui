# Safety and Confirmation Prompts

Use this file when actions can mutate data or require credentials.

## Always Confirm
- Destructive actions (delete, reset, overwrite, submit).
- Actions against production or real user data.
- Use of real credentials.

## Prompt Examples
- "Is this a test environment, and is it safe to submit or delete data?"
- "Do you want me to use live preview while running the test?"
- "Can you provide test credentials or a fixture account?"

## Safe Defaults
- Prefer dry-run or read-only flows when available.
- Avoid submitting forms unless explicitly requested.
- Avoid sharing secrets in logs or outputs.
