# Session Lifecycle Commands

Commands for starting, stopping, and managing application sessions.

## Command Summary

| Browser Command | TUI Equivalent | Status | Notes |
|-----------------|----------------|--------|-------|
| `launch` | `spawn` | ✅ Exists | Different params (no browser type) |
| `close` | `kill` | ✅ Exists | Identical semantics |
| `tab_list` | `sessions` | ✅ Exists | Different response format |
| - | `health` | TUI-only | Daemon health check |

---

## spawn (TUI) / launch (Browser)

**Purpose**: Start a new application session

### Browser Signature

```typescript
interface LaunchCommand {
  action: 'launch';
  headless?: boolean;
  viewport?: { width: number; height: number };
  browser?: 'chromium' | 'firefox' | 'webkit';
  cdpPort?: number;
  executablePath?: string;
  extensions?: string[];
  headers?: Record<string, string>;
  proxy?: { server: string; bypass?: string; username?: string; password?: string };
}

// Response
interface LaunchResponse {
  launched: boolean;
  browserType: string;
  viewport: { width: number; height: number };
}
```

### TUI Signature

```rust
pub struct SpawnParams {
    pub command: String,              // Required: command to run
    pub args: Option<Vec<String>>,    // Optional: command arguments
    pub cwd: Option<String>,          // Optional: working directory
    pub env: Option<HashMap<String, String>>, // Optional: environment
    pub session: Option<String>,      // Optional: custom session ID
    pub cols: Option<u16>,            // Default: 120
    pub rows: Option<u16>,            // Default: 40
}

pub struct SpawnResult {
    pub session_id: String,
    pub pid: u32,
}
```

### CLI Usage

```bash
# Browser
agent-browser launch --headless --viewport 1920x1080

# TUI
agent-tui spawn htop
agent-tui spawn bash -- -c "npm start"
agent-tui spawn --cols 120 --rows 40 vim file.txt
```

### Key Differences

| Aspect | Browser | TUI |
|--------|---------|-----|
| Target | Browser instance | PTY process |
| Main param | `executablePath` | `command` + `args` |
| Size | `viewport.width/height` (pixels) | `cols/rows` (characters) |
| Browser-specific | `browser`, `headless`, `cdpPort`, `extensions`, `headers`, `proxy` | N/A |

### JSON-RPC Example

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "spawn",
  "params": {
    "command": "htop",
    "cols": 120,
    "rows": 40
  },
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "session_id": "htop-abc123",
    "pid": 12345
  },
  "id": 1
}
```

---

## kill (TUI) / close (Browser)

**Purpose**: Terminate the application session

### Browser Signature

```typescript
interface CloseCommand {
  action: 'close';
}

// Response
interface CloseResponse {
  closed: boolean;
}
```

### TUI Signature

```rust
pub struct KillParams {
    pub session: Option<String>,  // Optional: specific session (defaults to active)
}

pub struct KillResult {
    pub success: bool,
    pub session_id: String,
}
```

### CLI Usage

```bash
# Browser
agent-browser close

# TUI
agent-tui kill
agent-tui kill --session htop-abc123
```

### Parity

✅ **Identical semantics** - Both terminate the active session/browser.

### JSON-RPC Example

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "kill",
  "params": {
    "session": "htop-abc123"
  },
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "success": true,
    "session_id": "htop-abc123"
  },
  "id": 1
}
```

---

## sessions (TUI) / tab_list (Browser)

**Purpose**: List all active sessions/tabs

### Browser Signature

```typescript
interface TabListCommand {
  action: 'tab_list';
}

// Response
interface TabListResponse {
  tabs: Array<{
    index: number;
    url: string;
    title: string;
    active: boolean;
  }>;
  active: number;
}
```

### TUI Signature

```rust
pub struct SessionsParams {
    // No parameters
}

pub struct SessionsResult {
    pub sessions: Vec<SessionInfo>,
    pub active_session: Option<String>,
}

pub struct SessionInfo {
    pub id: String,
    pub command: String,
    pub pid: u32,
    pub running: bool,
    pub created_at: String,
    pub size: TerminalSize,
}
```

### CLI Usage

```bash
# Browser
agent-browser tab_list

# TUI
agent-tui sessions
```

### Key Differences

| Aspect | Browser | TUI |
|--------|---------|-----|
| Identifier | Numeric `index` | String `id` |
| Location | `url` | `command` |
| Metadata | `title` | `pid`, `running`, `created_at`, `size` |

### JSON-RPC Example

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "sessions",
  "params": {},
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "sessions": [
      {
        "id": "htop-abc123",
        "command": "htop",
        "pid": 12345,
        "running": true,
        "created_at": "2024-01-15T10:30:00Z",
        "size": { "cols": 120, "rows": 40 }
      }
    ],
    "active_session": "htop-abc123"
  },
  "id": 1
}
```

---

## health (TUI only)

**Purpose**: Check daemon health and connection status

### TUI Signature

```rust
pub struct HealthParams {
    // No parameters
}

pub struct HealthResult {
    pub status: String,           // "healthy" | "degraded"
    pub pid: u32,
    pub uptime_ms: u64,
    pub session_count: usize,
    pub version: String,
    pub memory_usage_mb: Option<f64>,
    pub memory_details: Option<MemoryDetails>,
    pub request_stats: Option<RequestStats>,
    pub degradation_reasons: Option<Vec<String>>,
}
```

### CLI Usage

```bash
agent-tui health
```

### Notes

- No browser equivalent exists
- TUI-specific for daemon management
- Returns degradation reasons if status is "degraded"

### JSON-RPC Example

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "health",
  "params": {},
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "status": "healthy",
    "pid": 1234,
    "uptime_ms": 3600000,
    "session_count": 2,
    "version": "0.1.0"
  },
  "id": 1
}
```
