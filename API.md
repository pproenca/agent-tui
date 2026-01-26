# agent-tui API (v1)

This API is exposed by the daemon over HTTP and WebSocket. It is intended for
external frontends and automation clients.

Defaults
- HTTP bind: 127.0.0.1:0 (ephemeral)
- Token: generated on daemon start (unless disabled)
- State file: ~/.agent-tui/api.json

Configuration
- AGENT_TUI_API_LISTEN=127.0.0.1:0
- AGENT_TUI_API_ALLOW_REMOTE=1
- AGENT_TUI_API_TOKEN=<token> (or "none" to disable)
- AGENT_TUI_API_STATE=/path/to/api.json
- AGENT_TUI_API_DISABLED=1 (disable HTTP/WS server)
- AGENT_TUI_API_MAX_CONNECTIONS=32
- AGENT_TUI_API_WS_QUEUE=128

Authentication
- Provide the token using one of:
  - Authorization: Bearer <token>
  - x-agent-tui-token: <token>
  - token=<token> query parameter

HTTP Endpoints
GET /api/v1/version
Response:
  {
    "api_version": "1",
    "daemon_version": "...",
    "daemon_commit": "..."
  }

GET /api/v1/health
Response:
  {
    "status": "healthy",
    "pid": 1234,
    "uptime_ms": 123456,
    "session_count": 2,
    "api_version": "1",
    "daemon_version": "...",
    "daemon_commit": "..."
  }

GET /api/v1/sessions
Response:
  {
    "active": "session-id" | null,
    "sessions": [
      {
        "id": "...",
        "command": "...",
        "pid": 1234,
        "running": true,
        "created_at": "...",
        "size": { "cols": 80, "rows": 24 }
      }
    ]
  }

GET /api/v1/sessions/:id/snapshot
Use :id = "active" for the active session.
Response:
  {
    "cols": 80,
    "rows": 24,
    "init": "<initial-render-seq>"
  }

WebSocket
WS /api/v1/stream?session=<id>
Use session=active to follow the active session.

First message (server -> client):
  {
    "event": "hello",
    "api_version": "1",
    "daemon_version": "...",
    "daemon_commit": "...",
    "session_id": "..."
  }

Stream events (server -> client):
- ready:
  { "event": "ready", "session_id": "...", "cols": 80, "rows": 24 }
- init:
  { "event": "init", "time": 0.0, "cols": 80, "rows": 24, "init": "<seq>" }
- output:
  { "event": "output", "time": 1.23, "data_b64": "<base64>" }
- resize:
  { "event": "resize", "time": 3.21, "cols": 120, "rows": 30 }
- dropped:
  { "event": "dropped", "time": 4.56, "dropped_bytes": 8192 }
- heartbeat:
  { "event": "heartbeat", "time": 5.00 }
- closed:
  { "event": "closed", "time": 9.87 }
- error:
  { "event": "error", "message": "..." }

Example (curl)
  curl -H "Authorization: Bearer $TOKEN" http://127.0.0.1:PORT/api/v1/version

State file (default: ~/.agent-tui/api.json)
  {
    "pid": 1234,
    "http_url": "http://127.0.0.1:PORT/",
    "ws_url": "ws://127.0.0.1:PORT/api/v1/stream",
    "listen": "127.0.0.1:PORT",
    "token": "....",
    "api_version": "1",
    "started_at": 1710000000
  }
