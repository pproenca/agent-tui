# Discovery

Run:
- `skills/tui-explorer/scripts/tui_explorer discover --command "<app command>"`

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
- `acceptance.md`
- `trace.jsonl`
- `discover-report.json`
