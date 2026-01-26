# agent-tui web UI (Bun)

This is the Bun-powered frontend for the daemon live preview API.

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

## Live command integration

The CLI can auto-start this web UI when you run `agent-tui live`.
If you prefer to run the UI separately, set a UI URL and use `--open`:

```bash
export AGENT_TUI_UI_URL=http://127.0.0.1:4173
agent-tui live start --open
```

The CLI appends `api`, `ws`, `token`, `session`, `encoding`, and `auto=1` query params so the UI auto-connects.

### Auto-start configuration (optional)

- `AGENT_TUI_UI_MODE=dev|serve` (default: `dev`)
- `AGENT_TUI_UI_PORT=4173` (default: `4173`)
- `AGENT_TUI_UI_ROOT=/path/to/web` (default: auto-detect `./web`)
- `AGENT_TUI_UI_STATE=/path/to/ui.json` (default: `~/.agent-tui/ui.json`)

## Manual URL params

- `api` (HTTP API base URL)
- `ws` (WebSocket stream URL)
- `token` (auth token)
- `session` (`active` or a session id)
- `encoding` (`binary` or `base64`)
- `auto=1` (auto-connect on load)
