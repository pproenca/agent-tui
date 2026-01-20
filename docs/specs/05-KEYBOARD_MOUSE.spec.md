# Keyboard and Mouse Commands

Commands for keyboard input and scrolling.

## Command Summary

| Browser Command | TUI Equivalent | Status | Notes |
|-----------------|----------------|--------|-------|
| `keyboard` | `keystroke` | ✅ Exists | Supports combos |
| `keydown` | - | ❌ Missing | Hold key down |
| `keyup` | - | ❌ Missing | Release key |
| `inserttext` | `type` | ✅ Exists | Same semantics |
| `mousemove` | N/A | ❌ N/A | No mouse in PTY |
| `mousedown` | N/A | ❌ N/A | No mouse in PTY |
| `mouseup` | N/A | ❌ N/A | No mouse in PTY |
| `wheel` | `scroll` | ✅ Exists | Different impl |
| `tap` | `click` | ✅ Exists | Same as click |
| `scroll` | `scroll` | ✅ Exists | Identical |
| `scrollintoview` | `scrollintoview` | ✅ Exists | Identical |

---

## keystroke (TUI) / keyboard (Browser)

**Purpose**: Send a keystroke or key combination

### Browser Signature

```typescript
interface KeyboardCommand {
  action: 'keyboard';
  keys: string;           // "Control+a", "Shift+Tab"
  delay?: number;         // Delay between keys
}

// Response
interface KeyboardResponse {
  pressed: string;
}
```

### TUI Signature

```rust
pub struct KeystrokeParams {
    pub key: String,          // "Ctrl+C", "Enter", "ArrowDown"
    pub session: Option<String>,
}

pub struct KeystrokeResult {
    pub success: bool,
}
```

### CLI Usage

```bash
# Browser
agent-browser keyboard "Control+a"
agent-browser keyboard "Shift+Tab"

# TUI
agent-tui keystroke "Ctrl+A"
agent-tui keystroke Enter
agent-tui keystroke "Shift+Tab"
```

### Key Names Mapping

| Browser | TUI | Description |
|---------|-----|-------------|
| `Control+a` | `Ctrl+A` | Select all |
| `Shift+Tab` | `Shift+Tab` | Reverse tab |
| `ArrowDown` | `ArrowDown` | Down arrow |
| `ArrowUp` | `ArrowUp` | Up arrow |
| `ArrowLeft` | `ArrowLeft` | Left arrow |
| `ArrowRight` | `ArrowRight` | Right arrow |
| `Enter` | `Enter` | Enter/Return |
| `Escape` | `Escape` | Escape |
| `Tab` | `Tab` | Tab |
| `Backspace` | `Backspace` | Backspace |
| `Delete` | `Delete` | Delete |
| `Home` | `Home` | Home |
| `End` | `End` | End |
| `PageUp` | `PageUp` | Page up |
| `PageDown` | `PageDown` | Page down |
| `F1` - `F12` | `F1` - `F12` | Function keys |

### Modifier Combinations

| Combination | TUI Format |
|-------------|------------|
| Control + key | `Ctrl+X` |
| Alt + key | `Alt+X` |
| Shift + key | `Shift+X` |
| Control + Shift + key | `Ctrl+Shift+X` |
| Control + Alt + key | `Ctrl+Alt+X` |

### JSON-RPC Example

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "keystroke",
  "params": {
    "key": "Ctrl+C"
  },
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "success": true
  },
  "id": 1
}
```

---

## keydown / keyup (Browser) - MISSING IN TUI

**Purpose**: Hold or release a key

### Browser Signature

```typescript
interface KeyDownCommand {
  action: 'keydown';
  key: string;
}

interface KeyUpCommand {
  action: 'keyup';
  key: string;
}
```

### Use Cases

- Hold Shift while clicking multiple items
- Hold Control for multi-select
- Modifier key sequences

### Recommendation

Add `keydown`/`keyup` to TUI for modifier key sequences:

```rust
// Proposed TUI signature
pub struct KeyDownParams {
    pub key: String,
    pub session: Option<String>,
}

pub struct KeyUpParams {
    pub key: String,
    pub session: Option<String>,
}
```

---

## type (TUI) / inserttext (Browser)

**Purpose**: Type literal text

### Browser Signature

```typescript
interface InsertTextCommand {
  action: 'inserttext';
  text: string;
}
```

### TUI Signature

```rust
pub struct TypeParams {
    pub text: String,
    pub session: Option<String>,
}

pub struct TypeResult {
    pub success: bool,
}
```

### Parity

✅ **Identical semantics** - Both type text literally to active input.

---

## scroll (Both)

**Purpose**: Scroll the viewport

### Browser Signature

```typescript
interface ScrollCommand {
  action: 'scroll';
  selector?: string;
  x?: number;
  y?: number;
  direction?: 'up' | 'down' | 'left' | 'right';
  amount?: number;
}

// Response
interface ScrollResponse {
  scrolled: boolean;
}
```

### TUI Signature

```rust
pub struct ScrollParams {
    #[serde(rename = "ref")]
    pub element_ref: Option<String>,
    pub direction: String,    // "up" | "down" | "left" | "right"
    pub amount: Option<u16>,  // Default: 5 lines
    pub session: Option<String>,
}

pub struct ScrollResult {
    pub success: bool,
    pub scrolled_amount: Option<u16>,
}
```

### CLI Usage

```bash
# Browser
agent-browser scroll --direction down --amount 100

# TUI
agent-tui scroll down
agent-tui scroll down --amount 10
agent-tui scroll up --ref @e5
```

### Key Differences

| Aspect | Browser | TUI |
|--------|---------|-----|
| Position | `x`, `y` coordinates (pixels) | Direction only |
| Amount | Pixels | Lines/rows |
| Implementation | Mouse wheel events | Arrow keys or Page Up/Down |

### Implementation Details

TUI scroll implementation:
- `up`/`down` - Sends arrow keys or Page Up/Down
- `amount` - Number of times to send the key
- Large amounts use Page Up/Down for efficiency

---

## scrollintoview (Both)

**Purpose**: Scroll until element is visible

### Browser Signature

```typescript
interface ScrollIntoViewCommand {
  action: 'scrollintoview';
  selector: string;
  behavior?: 'auto' | 'smooth';
  block?: 'start' | 'center' | 'end' | 'nearest';
}

// Response
interface ScrollIntoViewResponse {
  scrolled: boolean;
}
```

### TUI Signature

```rust
pub struct ScrollIntoViewParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub session: Option<String>,
}

pub struct ScrollIntoViewResult {
    pub success: bool,
    pub message: Option<String>,
}
```

### CLI Usage

```bash
# Browser
agent-browser scrollintoview "#footer"

# TUI
agent-tui scrollintoview @e15
```

### Implementation

TUI implementation:
1. Take snapshot to check if element is visible
2. If not visible, scroll in predicted direction
3. Repeat until element appears in snapshot
4. Return success when element is in view

---

## Mouse Commands (Browser only)

These commands have no TUI equivalent:

### mousemove

```typescript
interface MouseMoveCommand {
  action: 'mousemove';
  x: number;
  y: number;
}
```

### mousedown / mouseup

```typescript
interface MouseDownCommand {
  action: 'mousedown';
  button?: 'left' | 'right' | 'middle';
}

interface MouseUpCommand {
  action: 'mouseup';
  button?: 'left' | 'right' | 'middle';
}
```

### Why Not Applicable

- PTY terminals typically don't support mouse events
- Some terminals support mouse protocols (xterm, etc.) but most TUI apps don't use them
- Focus-based navigation is the standard for TUI

### Alternative Patterns

For actions that would use mouse in browser:

| Browser Action | TUI Alternative |
|----------------|-----------------|
| Click element | `focus` + `keystroke Enter` or `click` |
| Hover for tooltip | Not available (tooltips rare in TUI) |
| Drag and drop | Not available |
| Right-click context menu | App-specific key (often menu key or specific keystroke) |
| Mouse selection | `keystroke Shift+Arrow` for selection |

---

## Summary

| Category | Browser | TUI | Notes |
|----------|---------|-----|-------|
| Key press | `keyboard` | `keystroke` | Identical function |
| Text input | `inserttext`, `type` | `type` | Identical function |
| Scrolling | `scroll`, `wheel` | `scroll` | Direction-based |
| Mouse | Full support | N/A | Use keyboard navigation |
| Key hold | `keydown`/`keyup` | Missing | Consider adding |
