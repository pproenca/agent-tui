# Sessions CLI Restructure (Spec/Plan)

Date: 2026-01-25
Owner: Codex (with user approval)
Status: In progress

## Context
The current `sessions` command mixes flags for modes (list/cleanup/attach/status) and advertises
`agent-tui sessions <id>` details without implementing it. The attach flow also requires an explicit
ID, which blocks the desired `agent-tui sessions attach` default behavior. There is also a client/
server mismatch where `pty_read` accepts `timeout_ms` on the client but the daemon ignores it.

## Goals
- Provide a clear UNIX-style `sessions` subcommand interface.
- Make `agent-tui sessions attach` work without an explicit ID by defaulting to `--session` or the
  active session reported by the daemon.
- Implement an explicit “show details” path.
- Ensure `sessions status` respects `-v` verbosity.
- Fix the `pty_read` `timeout_ms` contract so the client parameter is honored.

## Non-goals
- Backwards compatibility with `sessions --attach/--cleanup/--status` flags.
- Adding new daemon RPC methods for sessions; reuse the existing `sessions` response.

## CLI Design
```
agent-tui sessions                 # list (default)
agent-tui sessions list            # list (explicit)
agent-tui sessions show <id>       # show details for a session
agent-tui sessions attach [id]     # interactive attach; defaults to --session or active
agent-tui sessions cleanup [--all] # remove dead/orphaned sessions
agent-tui sessions status          # show daemon health (respects -v)
```

### Attach resolution rules
1. If `attach [id]` is provided, use it.
2. Else if `--session <id>` is set, use it.
3. Else call `sessions` RPC and use `active_session`.
4. If no active session, return a user-facing error suggesting `sessions list` or `--session`.

### JSON output shape
- `sessions` list: unchanged (daemon result pass-through).
- `sessions show <id>`: `{ "session": {..}, "active_session": "<id|null>" }`.
- `sessions status`: unchanged (health result pass-through).

## Implementation Plan
1. Restructure CLI definitions to use subcommands under `Sessions`.
2. Update dispatch routing and implement `sessions show` + attach resolution logic.
3. Respect `-v` for `sessions status`.
4. Add `timeout_ms` to `PtyReadInput`, parse it from RPC, and pass it to the use case.
5. Update tests for new CLI shapes and timeout parsing.

## Test Plan
- CLI parsing unit tests for new `sessions` subcommands.
- Integration tests:
  - `sessions status` still calls `health` and prints status.
  - `sessions attach` without ID fails cleanly when no active session (non-TTY still errors).
  - `sessions show <id>` prints a single session.
- RPC parsing tests for `pty_read` include `timeout_ms` and defaults.

## Risks
- Breaking change for scripts using `sessions --attach/--cleanup/--status`.
- If no active session is reported, `sessions attach` requires explicit `--session` or ID.

