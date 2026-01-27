# JSON Output Contract

Use this file when parsing `--format json` output.

## Spawn / Run
- `agent-tui run ...` returns:
  ```json
  { "session_id": "<id>", "pid": 123 }
  ```

## Screenshot
- `agent-tui screenshot ...` returns:
  ```json
  {
    "session_id": "<id>",
    "screenshot": "<string>",
    "elements": [ ... ],
    "cursor": { "row": 0, "col": 0, "visible": true },
    "rendered": "<optional>"
  }
  ```
- `elements[]` item shape:
  ```json
  {
    "ref": "@e1",
    "type": "button|input|checkbox|radio|select|menu_item|link|...",
    "label": "Submit",
    "value": "<value>",
    "position": { "row": 10, "col": 5, "width": 12, "height": 1 },
    "focused": false,
    "selected": false,
    "checked": true,
    "disabled": false,
    "hint": "<optional>"
  }
  ```

## Find / Count
- `agent-tui find ...` returns:
  ```json
  { "elements": [ ... ], "count": 2 }
  ```
- `agent-tui count ...` returns:
  ```json
  { "count": 2 }
  ```

## Wait
- `agent-tui wait ...` returns:
  ```json
  { "found": true, "elapsed_ms": 1200 }
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

## Action/Mutation Responses
- Most action commands return:
  ```json
  { "success": true, "message": "...", "warning": "<optional>" }
  ```
- `kill` returns:
  ```json
  { "success": true, "session_id": "<id>" }
  ```
