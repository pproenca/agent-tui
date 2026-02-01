# JSON Output Contract

Use this file when parsing `--format json` output.

## Spawn / Run
- `agent-tui run ...` returns:
  ```json
  { "session_id": "<id>", "pid": 123 }
  ```

## Screenshot (text)
- `agent-tui screenshot ...` returns:
  ```json
  {
    "session_id": "<id>",
    "screenshot": "<string>",
    "cursor": { "row": 0, "col": 0, "visible": true },
    "rendered": "<optional>"
  }
  ```

## Wait
- `agent-tui wait ...` returns:
  ```json
  { "found": true, "elapsed_ms": 1200 }
  ```

## Scroll
- `agent-tui scroll ...` returns:
  ```json
  { "success": true }
  ```

## Resize
- `agent-tui resize ...` returns:
  ```json
  { "success": true, "session_id": "<id>", "cols": 120, "rows": 40 }
  ```

## Restart / Kill
- `restart` returns:
  ```json
  { "old_session_id": "<id>", "new_session_id": "<id>", "command": "<cmd>", "pid": 123 }
  ```
- `kill` returns:
  ```json
  { "success": true, "session_id": "<id>" }
  ```

## Sessions
- `agent-tui sessions` returns:
  ```json
  {
    "sessions": [
      {
        "id": "<id>",
        "command": "<command>",
        "pid": 123,
        "running": true,
        "created_at": "<timestamp>",
        "size": { "cols": 120, "rows": 40 }
      }
    ],
    "active_session": "<id>"
  }
  ```
