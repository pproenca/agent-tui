# agent-tui web UI (Bun)

This is the Bun-powered frontend for the daemon live preview WebSocket RPC.
In normal usage, the daemon serves the built UI directly at `/ui`.

## Run (dev)

```bash
bun install
bun run dev
```

The server defaults to `http://127.0.0.1:4173`.

## Run (prod-like)

```bash
bun install
bun run build
bun run serve
```

## External UI integration

If you prefer to run the UI separately, set a UI URL and use `--open`:

```bash
export AGENT_TUI_UI_URL=http://127.0.0.1:4173
agent-tui live start --open
```

The CLI appends `ws`, `session`, and `auto=1` query params so the UI auto-connects.

## Manual URL params

- `ws` (WebSocket stream URL)
- `session` (`active` or a session id)
- `auto=1` (auto-connect on load)
