# Snapshot Commands

Commands for capturing screen state and content.

## Command Summary

| Browser Command | TUI Equivalent | Status | Notes |
|-----------------|----------------|--------|-------|
| `snapshot` | `snapshot` | ✅ Exists | Accessibility tree |
| `screenshot` | `screen` | ✅ Exists | ANSI text capture |
| `content` | `screen` | ✅ Exists | Raw content = screen |

---

## snapshot (Both)

**Purpose**: Capture current screen state with element detection

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
    pub interactive_only: Option<bool>,   // Only interactive elements
    pub compact: Option<bool>,            // Compact output
    pub region: Option<String>,           // Scope to region
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
    pub cursor: Option<CursorPosition>,
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
agent-tui snapshot
agent-tui snapshot --interactive-only --format json
agent-tui snapshot --compact
```

### Key Differences

| Aspect | Browser | TUI |
|--------|---------|-----|
| Element refs | `refs` map | `elements` array |
| Position | `boundingBox` (pixels) | `position` (row/col) |
| Cursor | N/A | Included |
| Size | N/A | Included (cols/rows) |
| Format | Text only | text, json, tree |
| Depth control | `maxDepth` | Detection algorithm |

### JSON-RPC Example

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "snapshot",
  "params": {
    "include_elements": true,
    "interactive_only": true
  },
  "id": 1
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

## screen (TUI) / screenshot + content (Browser)

**Purpose**: Get raw screen content without element detection

### Browser Signature (screenshot)

```typescript
interface ScreenshotCommand {
  action: 'screenshot';
  path?: string;          // Save to file
  fullPage?: boolean;     // Capture entire page
  selector?: string;      // Scope to element
  type?: 'png' | 'jpeg';
  quality?: number;       // 0-100 for jpeg
}

// Response
interface ScreenshotResponse {
  path?: string;
  base64?: string;
}
```

### Browser Signature (content)

```typescript
interface ContentCommand {
  action: 'content';
  selector?: string;      // Optional: scope to element
}

// Response
interface ContentResponse {
  html: string;
}
```

### TUI Signature

```rust
pub struct ScreenParams {
    pub session: Option<String>,
    pub strip_ansi: Option<bool>,     // Remove ANSI codes
    pub include_cursor: Option<bool>, // Show cursor position
}

pub struct ScreenResult {
    pub session_id: String,
    pub screen: String,       // Raw ANSI text (or plain if stripped)
    pub size: TerminalSize,
    pub cursor: Option<CursorPosition>,
}
```

### CLI Usage

```bash
# Browser
agent-browser screenshot --path ./screenshot.png
agent-browser content

# TUI
agent-tui screen
agent-tui screen --strip-ansi
```

### Key Differences

| Aspect | Browser (screenshot) | Browser (content) | TUI (screen) |
|--------|---------------------|-------------------|--------------|
| Output | Image (PNG/JPEG) | HTML | ANSI text |
| Scope | Full page or element | Element | Full terminal |
| Format | Binary/base64 | Text | Text |

### JSON-RPC Example

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "screen",
  "params": {
    "strip_ansi": true
  },
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "session_id": "htop-abc123",
    "screen": "  PID USER      PRI  NI  VIRT   RES   SHR S  CPU%  MEM%   TIME+  Command\n...",
    "size": { "cols": 120, "rows": 40 }
  },
  "id": 1
}
```

---

## Comparison: When to Use Each

| Use Case | Browser | TUI |
|----------|---------|-----|
| Get element references for interaction | `snapshot` | `snapshot` |
| Get raw visual content | `screenshot` | `screen` |
| Get structured content | `content` (HTML) | `snapshot --format json` |
| Debug UI state | `screenshot` | `screen` (with ANSI colors) |
| Automated testing assertions | `snapshot` | `snapshot` or `screen` |

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
