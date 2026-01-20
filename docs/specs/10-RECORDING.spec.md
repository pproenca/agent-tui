# Recording Commands

Commands for recording sessions, traces, and logs.

## Command Summary

| Browser Command | TUI Equivalent | Status | Notes |
|-----------------|----------------|--------|-------|
| `video_start` | N/A | ❌ N/A | No video |
| `video_stop` | N/A | ❌ N/A | No video |
| `recording_start` | `record_start` | ✅ Exists | Different format |
| `recording_stop` | `record_stop` | ✅ Exists | JSON/asciicast |
| `trace_start` | `trace --start` | ✅ Exists | Identical |
| `trace_stop` | `trace --stop` | ✅ Exists | Identical |
| `har_start` | N/A | ❌ N/A | No HTTP |
| `har_stop` | N/A | ❌ N/A | No HTTP |
| `console` | `console` | ✅ Exists | Identical |
| `errors` | - | ❌ Missing | Could add |

---

## record_start / record_stop (TUI) / recording_* (Browser)

**Purpose**: Record session for playback

### Browser Signature

```typescript
interface RecordingStartCommand {
  action: 'recording_start';
  path: string;
  url?: string;
}

interface RecordingStopCommand {
  action: 'recording_stop';
}

// Response
interface RecordingStartResponse {
  started: boolean;
  path: string;
}

interface RecordingStopResponse {
  path: string;
  frames: number;
  error?: string;
}
```

### TUI Signature

```rust
pub struct RecordStartParams {
    pub session: Option<String>,
}

pub struct RecordStopParams {
    pub session: Option<String>,
    pub output: Option<String>,       // Output file path
    pub format: Option<RecordFormat>, // json | asciicast
}

pub enum RecordFormat {
    Json,       // Custom JSON format
    Asciicast,  // asciinema compatible
}

pub struct RecordStartResult {
    pub success: bool,
    pub session_id: String,
    pub message: Option<String>,
}

pub struct RecordStopResult {
    pub success: bool,
    pub session_id: String,
    pub frame_count: Option<u64>,
    pub duration_ms: Option<u64>,
    pub output_file: Option<String>,
}
```

### CLI Usage

```bash
# Browser
agent-browser recording_start --path ./recording.webm
# ... do actions ...
agent-browser recording_stop

# TUI
agent-tui record-start
# ... do actions ...
agent-tui record-stop --output ./session.cast --format asciicast
```

### Key Differences

| Aspect | Browser | TUI |
|--------|---------|-----|
| Output format | Video (webm/mp4) | JSON or asciicast |
| Path timing | On start | On stop |
| Playback | Video player | asciinema play |

### JSON-RPC Example

```json
// Start recording
{
  "jsonrpc": "2.0",
  "method": "record_start",
  "params": {},
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

// Stop recording
{
  "jsonrpc": "2.0",
  "method": "record_stop",
  "params": {
    "output": "./session.cast",
    "format": "asciicast"
  },
  "id": 2
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "success": true,
    "session_id": "htop-abc123",
    "frame_count": 150,
    "duration_ms": 30000,
    "output_file": "./session.cast"
  },
  "id": 2
}
```

### Output Formats

**JSON format** - Custom agent-tui format:
```json
{
  "version": 1,
  "session_id": "htop-abc123",
  "command": "htop",
  "start_time": "2024-01-15T10:30:00Z",
  "frames": [
    { "time_ms": 0, "type": "output", "data": "..." },
    { "time_ms": 100, "type": "input", "data": "\x1b[B" },
    { "time_ms": 150, "type": "output", "data": "..." }
  ]
}
```

**Asciicast format** - asciinema compatible:
```json
{"version": 2, "width": 120, "height": 40, "timestamp": 1705316200}
[0.0, "o", "...initial output..."]
[0.1, "i", "\x1b[B"]
[0.15, "o", "...updated output..."]
```

### Playback

```bash
# Using asciinema
asciinema play session.cast

# Using custom player (JSON format)
agent-tui playback session.json
```

---

## trace (Both)

**Purpose**: Capture detailed execution trace

### Browser Signature

```typescript
interface TraceStartCommand {
  action: 'trace_start';
  screenshots?: boolean;
  snapshots?: boolean;
}

interface TraceStopCommand {
  action: 'trace_stop';
  path: string;
}

// Response
interface TraceStartResponse {
  started: boolean;
}

interface TraceStopResponse {
  path: string;
  size: number;
}
```

### TUI Signature

```rust
pub struct TraceParams {
    pub session: Option<String>,
    pub count: Option<usize>,   // Get last N entries
    pub start: Option<bool>,    // Start tracing
    pub stop: Option<bool>,     // Stop tracing
}

pub struct TraceResult {
    pub session_id: String,
    pub is_tracing: bool,
    pub entries: Vec<TraceEntry>,
    pub formatted: Option<String>,
}

pub struct TraceEntry {
    pub timestamp: String,
    pub event_type: String,     // input, output, snapshot, command
    pub data: serde_json::Value,
}
```

### CLI Usage

```bash
# Browser
agent-browser trace_start --screenshots --snapshots
# ... do actions ...
agent-browser trace_stop --path ./trace.zip

# TUI
agent-tui trace --start
# ... do actions ...
agent-tui trace --stop
agent-tui trace --count 10  # View last 10 entries
```

### JSON-RPC Example

```json
// Start tracing
{
  "jsonrpc": "2.0",
  "method": "trace",
  "params": {
    "start": true
  },
  "id": 1
}

// Get trace entries
{
  "jsonrpc": "2.0",
  "method": "trace",
  "params": {
    "count": 5
  },
  "id": 2
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "session_id": "htop-abc123",
    "is_tracing": true,
    "entries": [
      {
        "timestamp": "2024-01-15T10:30:01.123Z",
        "event_type": "command",
        "data": { "method": "keystroke", "params": { "key": "ArrowDown" } }
      },
      {
        "timestamp": "2024-01-15T10:30:01.150Z",
        "event_type": "output",
        "data": { "bytes": 256 }
      }
    ]
  },
  "id": 2
}
```

---

## console (Both)

**Purpose**: Capture console/terminal output

### Browser Signature

```typescript
interface ConsoleCommand {
  action: 'console';
  clear?: boolean;
}

// Response
interface ConsoleResponse {
  messages: ConsoleMessage[];
  cleared?: boolean;
}

interface ConsoleMessage {
  type: 'log' | 'warn' | 'error' | 'info' | 'debug';
  text: string;
  timestamp: number;
  location?: { url: string; line: number; column: number };
}
```

### TUI Signature

```rust
pub struct ConsoleParams {
    pub session: Option<String>,
    pub count: Option<usize>,     // Limit number of lines
    pub clear: Option<bool>,      // Clear console buffer
}

pub struct ConsoleResult {
    pub session_id: String,
    pub lines: Vec<String>,       // Output lines
    pub total_lines: usize,       // Total available
    pub cleared: Option<bool>,
}
```

### CLI Usage

```bash
# Browser
agent-browser console
agent-browser console --clear

# TUI
agent-tui console
agent-tui console --count 50
agent-tui console --clear
```

### Key Differences

| Aspect | Browser | TUI |
|--------|---------|-----|
| Content | console.log messages | Terminal output lines |
| Types | log, warn, error, info, debug | Plain text |
| Metadata | location (file, line) | N/A |

### JSON-RPC Example

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "console",
  "params": {
    "count": 20
  },
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "session_id": "htop-abc123",
    "lines": [
      "  PID USER      PRI  NI  VIRT   RES...",
      "  1234 root      20   0  168M  12.3M...",
      "  ..."
    ],
    "total_lines": 1523
  },
  "id": 1
}
```

---

## errors (Browser) - MISSING IN TUI

**Purpose**: Capture error messages separately

### Browser Signature

```typescript
interface ErrorsCommand {
  action: 'errors';
  clear?: boolean;
}

// Response
interface ErrorsResponse {
  errors: ErrorMessage[];
}

interface ErrorMessage {
  message: string;
  stack?: string;
  timestamp: number;
}
```

### Recommendation

Add `errors` command for TUI to capture stderr:

```rust
// Proposed TUI signature
pub struct ErrorsParams {
    pub session: Option<String>,
    pub count: Option<usize>,
    pub clear: Option<bool>,
}

pub struct ErrorsResult {
    pub session_id: String,
    pub errors: Vec<ErrorEntry>,
    pub total_count: usize,
}

pub struct ErrorEntry {
    pub timestamp: String,
    pub message: String,
    pub source: String,  // "stderr" | "exit_code" | "signal"
}
```

### Implementation Notes

TUI should separately capture:
- stderr output from the spawned process
- Non-zero exit codes
- Signals received (SIGSEGV, etc.)

---

## Browser-Only Recording Commands

### video_start / video_stop

```typescript
interface VideoStartCommand {
  action: 'video_start';
  path: string;
}

interface VideoStopCommand {
  action: 'video_stop';
}
```

❌ **Not applicable** - No video recording for terminal.

**Alternative**: Use `record_start`/`record_stop` with asciicast format for similar effect.

### har_start / har_stop

```typescript
interface HarStartCommand {
  action: 'har_start';
  path?: string;
}

interface HarStopCommand {
  action: 'har_stop';
}
```

❌ **Not applicable** - No HTTP traffic to capture in TUI.

---

## Summary

| Recording Type | Browser | TUI | Status |
|----------------|---------|-----|--------|
| Session recording | `recording_*` (video) | `record_*` (asciicast) | ✅ Different format |
| Execution trace | `trace_*` | `trace` | ✅ |
| Console/output | `console` | `console` | ✅ |
| Error capture | `errors` | - | ❌ Missing |
| Video | `video_*` | N/A | ❌ N/A |
| Network (HAR) | `har_*` | N/A | ❌ N/A |

---

## Use Cases

### Debugging

```bash
# Start trace before problematic operation
agent-tui trace --start

# Perform operation
agent-tui click @e5

# Check what happened
agent-tui trace --count 10
```

### Automated Testing

```bash
# Record test session
agent-tui spawn ./my-app
agent-tui record-start

# Run test steps
agent-tui fill @e1 "test@example.com"
agent-tui click @e2
agent-tui wait "Success"

# Save recording
agent-tui record-stop --output ./test-session.cast
```

### Documentation

```bash
# Create demo recording
agent-tui spawn htop
agent-tui record-start

# Demonstrate features
agent-tui keystroke F6
agent-tui keystroke ArrowDown
agent-tui keystroke Enter

# Save for documentation
agent-tui record-stop --output ./htop-demo.cast --format asciicast
```
