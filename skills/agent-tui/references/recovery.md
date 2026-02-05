# Failure Recovery

Use this file when runs are flaky, stalled, or inconsistent.

## Common Failures and Fixes
- Text not found: re-run `screenshot`, wait for stability, then re-check expected text.
- Target not visible: use `press` navigation keys (for example `PageDown`, arrows), then re-snapshot.
- Wait timeout: increase `--timeout`, use `wait --stable`, then re-snapshot.
- No active session: `sessions` to list; re-run `run` if needed.
- Daemon not running: `daemon start`.
- Version mismatch: `daemon restart`.
- Unresponsive session: `kill`, then re-run.
- Layout missing/overflow: `resize --cols --rows`, re-snapshot.

## Retry Budget
- Allow 3-5 retries for transient UI changes.
- If still failing, stop and ask the user for guidance or updated expectations.

## Escalation
- Last resort: `daemon stop --force` then `daemon start`.
- Warn that `daemon restart` terminates all sessions.
