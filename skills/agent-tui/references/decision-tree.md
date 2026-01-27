# Decision Tree: Observe -> Decide -> Act

Use this file when choosing between commands or output modes.

## Observation Choice
- Need to act on elements? Use `screenshot -e --json` to get refs.
- Need to assert text only? Use `screenshot` (text) and `wait "text" --assert`.
- Need focus/interactive semantics? Use `screenshot -a --interactive-only`.
- Need targeted lookup by role/name/text? Use `find`/`count`.

## Action Choice
- Prefer `action` when you have an element ref.
- Use `press` for special keys or navigation (arrows, function keys).
- Use `input` when you do not know if the token is a key name or text.
- Use `type` when you explicitly want literal text typing only.

## Synchronization Choice
- Use `wait --stable` if UI is animating or rendering is flaky.
- Use `wait -e @ref` when waiting for an element to appear/disappear.
- Use `wait --focused @ref` for focus-driven flows.
- Use `wait --value @ref=VALUE` for form field validation.

## Selector Stability Rules
- Always re-snapshot after any action that could change the UI.
- Treat element refs as session-specific and screen-state-specific.
- Do not cache element refs across major screen transitions.

## Layout/Viewport Rules
- If the UI looks truncated or labels are missing, `resize` then re-snapshot.
- If an element exists but is not visible, `scroll-into-view @ref`.
