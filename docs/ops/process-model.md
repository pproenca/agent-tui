# Process Model

agent-tui is local-first but can be supervised like a service. These are the main process types:

- `daemon`: Long-running session manager + HTTP/WebSocket API + embedded UI.
- `web-ui` (optional): Standalone Bun server that serves the UI and proxies `/api-state`.
- CLI one-offs: `agent-tui <command>` for admin and interactive tasks.

## Procfile (example)

```
agent-tui-daemon: agent-tui daemon start --foreground
agent-tui-web: bun server.ts
```

## systemd (example)

```
[Unit]
Description=agent-tui daemon
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/agent-tui daemon start --foreground
Restart=on-failure
Environment=AGENT_TUI_API_ALLOW_REMOTE=1
Environment=AGENT_TUI_LOG_STREAM=stdout
Environment=AGENT_TUI_LOG_FORMAT=json

[Install]
WantedBy=multi-user.target
```

## Recommended envs for cloud-friendly runs

- `PORT`: Sets the API listen port when `AGENT_TUI_API_LISTEN` is unset.
- `AGENT_TUI_API_ALLOW_REMOTE=1`: Allows binding non-loopback addresses.
- `AGENT_TUI_LOG_STREAM=stdout`: Sends logs to stdout for aggregation.
- `AGENT_TUI_LOG_FORMAT=json`: Emits structured logs for log processors.

Local defaults remain unchanged; use a supervisor to manage restarts and lifecycle.
