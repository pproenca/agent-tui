# Session Lifecycle and Concurrency

Use this file when managing multiple sessions or debugging stuck runs.

## Lifecycle Overview
1) `run` creates a session and returns `session_id`.
2) Commands without `--session` target the most recent session.
3) Use `--session <id>` when multiple sessions exist.
4) End sessions with `kill` or `sessions cleanup`.

## Recommended Pattern
- Capture `session_id` from `run` JSON output.
- Pass `--session <id>` to every subsequent command in the flow.
- When running multiple apps concurrently, never rely on the default session.

## Inspect and Attach
- `sessions`: list active sessions.
- `sessions show <id>`: show session details.
- `sessions switch <id>`: set the active session.
- `sessions attach`: attach in TTY mode (detach with Ctrl-P Ctrl-Q). Use `-s <id>` to target a specific session.
- `-s <id> sessions attach -T`: stream output only (no TTY).

## Cleanup and Recovery
- `kill`: terminate the current session.
- `sessions cleanup`: remove dead/orphaned sessions.
- `sessions cleanup --all`: remove all sessions (including active).

## Restart Behavior
- `restart`: restart the current session command.
- `daemon restart`: restarts daemon and terminates all sessions.

## Health and Diagnostics
- `sessions status`: daemon health status.
- `daemon status`: daemon PID, uptime, version (exit code 0/3).
