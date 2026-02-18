# Discovery

Run:
- `skills/tui-explorer/scripts/tui_explorer discover --command "<app command>"`
- Before running discover, start live preview so users can observe exploration in the web UI:
  - `agent-tui live start --open`
- After verification, stop preview:
  - `agent-tui live stop`

## Strategy
- Bounded BFS over action paths.
- Rebuild each node from a fresh session for deterministic state evaluation.
- Deduplicate with:
  - `sha256(normalized_screenshot + cursor + cols + rows)`

## Normalization
- Strip ANSI sequences.
- Collapse whitespace.

## Stop rules
- `max_depth`
- `max_states`
- `branch_limit`
- `time_budget_sec`

## Artifacts
- `acceptance.md` (includes OpenSpec-style `WHEN/THEN/SHOULD` expectation narrative plus executable step lines)
- `trace.jsonl`
- `discover-report.json`

## Session isolation
- Session changes made in the web UI are preview-local.
- Discovery/verify commands must not rely on browser-driven active session switching.
