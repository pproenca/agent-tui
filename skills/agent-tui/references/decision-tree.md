# Decision Tree

Use this file when you need a quick command choice based on intent.

## Snapshot Strategy
- Need raw text? Use `screenshot`.
- Need machine-readable output? Use `screenshot --json`.

## Waiting Strategy
- Waiting for text to appear: `wait "text" --assert`.
- Waiting for text to disappear: `wait "text" --gone --assert`.
- Waiting for UI to settle: `wait --stable`.

## Action Strategy
- Navigate/confirm: `press` (keys or sequences).
- Enter text: `type "text"`.
- Move viewport: `scroll <dir> [amount]`.

## Reliability
- Re-snapshot after any action that could change the UI.
- Prefer `wait --stable` before acting on dynamic screens.
- Verify outcomes with `wait ... --assert`.
