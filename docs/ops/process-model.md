# Process Model

agent-tui is local-first but can be supervised like a service. These are the main process types:

- `daemon`: Long-running session manager + HTTP/WebSocket API + embedded UI.
- `web-ui` (optional): Standalone Bun server that serves the UI and proxies `/api-state`.
- CLI one-offs: `agent-tui <command>` for admin and interactive tasks.

## Procfile (example)

```
agent-tui-daemon: AGENT_TUI_DAEMON_FOREGROUND=1 agent-tui daemon start
agent-tui-web: bun server.ts
```

## systemd (example)

```
[Unit]
Description=agent-tui daemon
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/agent-tui daemon start
Restart=on-failure
Environment=AGENT_TUI_DAEMON_FOREGROUND=1
Environment=AGENT_TUI_WS_LISTEN=0.0.0.0:8080
Environment=AGENT_TUI_WS_ALLOW_REMOTE=1
Environment=AGENT_TUI_LOG_STREAM=stdout
Environment=AGENT_TUI_LOG_FORMAT=json

[Install]
WantedBy=multi-user.target
```

## Recommended envs for cloud-friendly runs

- `AGENT_TUI_WS_LISTEN=0.0.0.0:8080`: Explicit bind address for daemon WS endpoint.
- `AGENT_TUI_WS_ALLOW_REMOTE=1`: Allows binding non-loopback addresses.
- `AGENT_TUI_LOG_STREAM=stdout`: Sends logs to stdout for aggregation.
- `AGENT_TUI_LOG_FORMAT=json`: Emits structured logs for log processors.
- `PORT`: Optional Bun web preview server port when running standalone `web/server.ts`.

Local defaults remain unchanged; use a supervisor to manage restarts and lifecycle.
