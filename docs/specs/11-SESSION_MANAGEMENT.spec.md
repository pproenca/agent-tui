# Session Management Commands

Commands for managing terminal size, switching sessions, and related operations.

## Command Summary

| Browser Command | TUI Equivalent | Status | Notes |
|-----------------|----------------|--------|-------|
| `tab_new` | - | ❌ Missing | Multi-session |
| `tab_list` | `sessions` | ✅ Exists | List sessions |
| `tab_switch` | `attach` | ✅ Exists | Switch session |
| `tab_close` | `kill` | ✅ Exists | Close session |
| `viewport` | `resize` | ✅ Exists | Identical |
| `bringtofront` | `attach` | ✅ Exists | Same effect |
| `frame` | N/A | ❌ N/A | No iframes |
| `mainframe` | N/A | ❌ N/A | No iframes |

---

## resize (TUI) / viewport (Browser)

**Purpose**: Change the terminal/viewport size

### Browser Signature

```typescript
interface ViewportCommand {
  action: 'viewport';
  width: number;   // Pixels
  height: number;  // Pixels
}

// Response
interface ViewportResponse {
  width: number;
  height: number;
}
```

### TUI Signature

```rust
pub struct ResizeParams {
    pub cols: u16,              // Character columns
    pub rows: u16,              // Character rows
    pub session: Option<String>,
}

pub struct ResizeResult {
    pub success: bool,
    pub session_id: String,
    pub size: TerminalSize,
}

pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
}
```

### CLI Usage

```bash
# Browser
agent-browser viewport --width 1920 --height 1080

# TUI
agent-tui resize --cols 120 --rows 40
agent-tui resize --cols 80 --rows 24 --session htop-abc123
```

### Key Differences

| Aspect | Browser | TUI |
|--------|---------|-----|
| Units | Pixels | Characters |
| Terminology | width, height | cols, rows |
| Effect | Browser window size | PTY dimensions |

### JSON-RPC Example

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "resize",
  "params": {
    "cols": 120,
    "rows": 40
  },
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "success": true,
    "session_id": "htop-abc123",
    "size": {
      "cols": 120,
      "rows": 40
    }
  },
  "id": 1
}
```

### Common Terminal Sizes

| Name | Cols | Rows | Use Case |
|------|------|------|----------|
| Standard | 80 | 24 | Classic terminal |
| Wide | 120 | 40 | Modern default |
| Large | 160 | 50 | High resolution |
| HD | 200 | 60 | Very large screens |

---

## attach (TUI) / bringtofront + tab_switch (Browser)

**Purpose**: Switch to a specific session and make it active

### Browser Signature

Browser has separate commands:

```typescript
interface BringToFrontCommand {
  action: 'bringtofront';
}

interface TabSwitchCommand {
  action: 'tab_switch';
  index: number;
}

// Response
interface TabSwitchResponse {
  index: number;
  url: string;
  title: string;
}
```

### TUI Signature

TUI has unified attach:

```rust
pub struct AttachParams {
    pub session: String,    // Session ID to attach to
}

pub struct AttachResult {
    pub success: bool,
    pub session_id: String,
    pub message: Option<String>,
}
```

### CLI Usage

```bash
# Browser
agent-browser tab_switch --index 2
agent-browser bringtofront

# TUI
agent-tui attach htop-abc123
```

### Key Differences

| Aspect | Browser | TUI |
|--------|---------|-----|
| Identifier | Numeric index | String session ID |
| Commands | `tab_switch` + `bringtofront` | Single `attach` |

### JSON-RPC Example

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "attach",
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
    "session_id": "htop-abc123",
    "message": "Now attached to session htop-abc123"
  },
  "id": 1
}
```

---

## sessions (TUI) / tab_list (Browser)

**Purpose**: List all active sessions/tabs

Covered in [01-SESSION_LIFECYCLE.spec.md](01-SESSION_LIFECYCLE.spec.md#sessions-tui--tab_list-browser)

---

## kill (TUI) / tab_close (Browser)

**Purpose**: Close a session/tab

Covered in [01-SESSION_LIFECYCLE.spec.md](01-SESSION_LIFECYCLE.spec.md#kill-tui--close-browser)

---

## tab_new (Browser) - MISSING AS EXPLICIT COMMAND

**Purpose**: Create a new tab/session

### Browser Signature

```typescript
interface TabNewCommand {
  action: 'tab_new';
  url?: string;
}

// Response
interface TabNewResponse {
  index: number;
  url: string;
}
```

### TUI Equivalent

TUI uses `spawn` to create new sessions:

```bash
# Browser
agent-browser tab_new --url "https://example.com"

# TUI - spawn is equivalent to tab_new
agent-tui spawn htop
agent-tui spawn bash
```

### Note

While TUI doesn't have explicit `tab_new`, the `spawn` command creates new sessions. Multiple sessions can run simultaneously.

---

## Browser-Only Commands

### frame / mainframe

```typescript
interface FrameCommand {
  action: 'frame';
  selector: string;  // iframe selector
}

interface MainFrameCommand {
  action: 'mainframe';
}
```

❌ **Not applicable** - No iframes in TUI.

---

## Multi-Session Workflow

### Example: Running Multiple Apps

```bash
# Spawn first app
agent-tui spawn htop
# Returns session_id: htop-abc123

# Spawn second app (htop-abc123 keeps running)
agent-tui spawn vim file.txt
# Returns session_id: vim-def456

# List sessions
agent-tui sessions
# Shows both sessions

# Switch between them
agent-tui attach htop-abc123
agent-tui snapshot  # View htop

agent-tui attach vim-def456
agent-tui snapshot  # View vim

# Kill specific session
agent-tui kill --session htop-abc123
```

### Session ID Format

Session IDs are auto-generated in format: `{command}-{random}`

```
htop-abc123
vim-def456
bash-ghi789
```

You can also provide custom session IDs:

```bash
agent-tui spawn htop --session my-monitor
# Creates session with ID: my-monitor
```

---

## JSON-RPC Example: Full Session Workflow

```json
// 1. Spawn first session
{
  "jsonrpc": "2.0",
  "method": "spawn",
  "params": { "command": "htop" },
  "id": 1
}
// Response: { "session_id": "htop-abc123", "pid": 1234 }

// 2. Spawn second session
{
  "jsonrpc": "2.0",
  "method": "spawn",
  "params": { "command": "vim", "args": ["file.txt"] },
  "id": 2
}
// Response: { "session_id": "vim-def456", "pid": 5678 }

// 3. List sessions
{
  "jsonrpc": "2.0",
  "method": "sessions",
  "params": {},
  "id": 3
}
// Response: { "sessions": [...], "active_session": "vim-def456" }

// 4. Attach to htop
{
  "jsonrpc": "2.0",
  "method": "attach",
  "params": { "session": "htop-abc123" },
  "id": 4
}
// Response: { "success": true, "session_id": "htop-abc123" }

// 5. Resize htop
{
  "jsonrpc": "2.0",
  "method": "resize",
  "params": { "cols": 160, "rows": 50 },
  "id": 5
}
// Response: { "success": true, "size": { "cols": 160, "rows": 50 } }

// 6. Kill htop
{
  "jsonrpc": "2.0",
  "method": "kill",
  "params": { "session": "htop-abc123" },
  "id": 6
}
// Response: { "success": true, "session_id": "htop-abc123" }
```

---

## Summary

| Operation | Browser | TUI | Status |
|-----------|---------|-----|--------|
| List sessions | `tab_list` | `sessions` | ✅ |
| New session | `tab_new` | `spawn` | ✅ (different name) |
| Switch session | `tab_switch` | `attach` | ✅ |
| Close session | `tab_close` | `kill` | ✅ |
| Resize | `viewport` | `resize` | ✅ |
| Bring to front | `bringtofront` | `attach` | ✅ |
| iframes | `frame`/`mainframe` | N/A | ❌ N/A |
