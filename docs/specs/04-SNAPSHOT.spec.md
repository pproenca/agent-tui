# Snapshot Commands

Commands for capturing screen state and content.

## Command Summary

| Browser Command | TUI Equivalent | Status | Notes |
|-----------------|----------------|--------|-------|
| `snapshot` | `snapshot` | ✅ Exists | Accessibility tree |
| `screenshot` | `snapshot --strip-ansi` | ✅ Exists | Plain text capture |
| `content` | `snapshot` | ✅ Exists | Raw content = screen |

---

## snapshot (Both)

**Purpose**: Capture current screen state with optional element detection

### Browser Signature

```typescript
interface SnapshotCommand {
  action: 'snapshot';
  interactive?: boolean;  // Only interactive elements
  maxDepth?: number;      // A11y tree depth
  compact?: boolean;      // Compact output
  selector?: string;      // Scope to element
}

interface SnapshotResponse {
  snapshot: string;       // A11y tree text representation
  refs?: Record<string, {
    role: string;
    name?: string;
    boundingBox?: { x: number; y: number; width: number; height: number };
  }>;
}
```

### TUI Signature

```rust
pub struct SnapshotParams {
    pub session: Option<String>,
    pub include_elements: Option<bool>,   // Include detected elements
    pub format: Option<SnapshotFormat>,   // text | json | tree
    pub region: Option<String>,           // Scope to region
    pub strip_ansi: Option<bool>,         // Remove ANSI escape codes
    pub include_cursor: Option<bool>,     // Include cursor position
}

pub enum SnapshotFormat {
    Text,
    Json,
    Tree,
}

pub struct SnapshotResult {
    pub session_id: String,
    pub screen: String,             // Raw screen content
    pub elements: Option<Vec<Element>>,
    pub cursor: Option<CursorPosition>,  // Included with -i or --include-cursor
    pub size: TerminalSize,
}

pub struct Element {
    #[serde(rename = "ref")]
    pub element_ref: String,        // @e1, @e2, etc.
    pub role: String,               // button, textbox, menu, etc.
    pub name: Option<String>,       // Accessible name
    pub value: Option<String>,      // Current value
    pub focused: bool,
    pub disabled: bool,
    pub checked: Option<bool>,
    pub position: Position,
}
```

### CLI Usage

```bash
# Browser
agent-browser snapshot --interactive --compact

# TUI
agent-tui snapshot              # Screen only (most common)
agent-tui snapshot -i           # Screen + detected elements
agent-tui snapshot --strip-ansi # Plain text without colors
agent-tui snapshot -i -f json   # JSON with elements
agent-tui snapshot --include-cursor  # Include cursor position
```

### Key Differences

| Aspect | Browser | TUI |
|--------|---------|-----|
| Element refs | `refs` map | `elements` array |
| Position | `boundingBox` (pixels) | `position` (row/col) |
| Cursor | N/A | Included with `-i` or `--include-cursor` |
| Size | N/A | Included (cols/rows) |
| Format | Text only | text, json, tree |
| Depth control | `maxDepth` | Detection algorithm |

### JSON-RPC Example

```json
// Request - screen only
{
  "jsonrpc": "2.0",
  "method": "snapshot",
  "params": {
    "session": "htop-abc123"
  },
  "id": 1
}

// Request - with elements
{
  "jsonrpc": "2.0",
  "method": "snapshot",
  "params": {
    "include_elements": true
  },
  "id": 2
}

// Request - plain text (stripped ANSI)
{
  "jsonrpc": "2.0",
  "method": "snapshot",
  "params": {
    "strip_ansi": true
  },
  "id": 3
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "session_id": "htop-abc123",
    "screen": "  PID USER      PRI  NI  VIRT   RES...",
    "elements": [
      {
        "ref": "@e1",
        "role": "button",
        "name": "Help",
        "focused": false,
        "disabled": false,
        "position": { "row": 24, "col": 1, "width": 6 }
      },
      {
        "ref": "@e2",
        "role": "button",
        "name": "Setup",
        "focused": true,
        "disabled": false,
        "position": { "row": 24, "col": 8, "width": 7 }
      }
    ],
    "cursor": { "row": 5, "col": 10, "visible": true },
    "size": { "cols": 120, "rows": 40 }
  },
  "id": 1
}
```

### Output Formats

**Text format** (default):
```
Screen:
  PID USER      PRI  NI  VIRT   RES...
  ...

Elements:
  @e1 [button] "Help" at row 24
  @e2 [button] "Setup" at row 24 (focused)
```

**JSON format**:
```json
{
  "screen": "...",
  "elements": [...],
  "cursor": {...},
  "size": {...}
}
```

**Tree format**:
```
Screen (120x40)
├── Header
│   └── CPU/Memory bars
├── Process List
│   ├── @e1 [listitem] "1234 root htop"
│   └── @e2 [listitem] "5678 user bash" (focused)
└── Footer
    ├── @e3 [button] "Help"
    └── @e4 [button] "Setup"
```

---

## Comparison: When to Use Each

| Use Case | Browser | TUI |
|----------|---------|-----|
| Get element references for interaction | `snapshot` | `snapshot -i` |
| Get raw visual content | `screenshot` | `snapshot` |
| Get plain text (no formatting) | `content` | `snapshot --strip-ansi` |
| Get structured content | `content` (HTML) | `snapshot -f json` |
| Debug UI state | `screenshot` | `snapshot` (with ANSI colors) |
| Automated testing assertions | `snapshot` | `snapshot` |

---

## Detection Capabilities

TUI snapshot detects these element types:

| Role | Description | Detection Method |
|------|-------------|------------------|
| `button` | Clickable buttons | Bracketed text, F-key labels |
| `textbox` | Text input fields | Cursor position, borders |
| `checkbox` | Toggle checkboxes | `[x]`, `[ ]`, `[*]` patterns |
| `radiobutton` | Radio options | `(*)`, `( )` patterns |
| `menuitem` | Menu entries | Arrow indicators, highlighting |
| `listitem` | List entries | Consistent indentation |
| `progressbar` | Progress indicators | Bar characters |
| `tab` | Tab controls | Tab-like UI patterns |
| `dialog` | Modal dialogs | Border detection |
| `text` | Static text | Non-interactive content |
