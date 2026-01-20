# Element Interaction Commands

Commands for interacting with UI elements.

## Command Summary

| Browser Command | TUI Equivalent | Status | Notes |
|-----------------|----------------|--------|-------|
| `click` | `click` | ✅ Exists | Identical |
| `dblclick` | - | ❌ Missing | Could add |
| `focus` | `focus` | ✅ Exists | Tab navigation |
| `hover` | N/A | ❌ N/A | Mouse-only in browser |
| `type` | `type` | ✅ Exists | Identical |
| `fill` | `fill` | ✅ Exists | Identical |
| `press` | `keystroke` | ✅ Exists | Different name |
| `clear` | `clear` | ✅ Exists | Identical |
| `selectall` | - | ❌ Missing | Could add via Ctrl+A |
| `drag` | N/A | ❌ N/A | Mouse-only |
| `upload` | N/A | ❌ N/A | No file input in TUI |

---

## click (Both)

**Purpose**: Click/activate an element

### Browser Signature

```typescript
interface ClickCommand {
  action: 'click';
  selector: string;           // CSS selector or @ref
  button?: 'left' | 'right' | 'middle';
  clickCount?: number;
  delay?: number;
  position?: { x: number; y: number };
  modifiers?: Array<'Alt' | 'Control' | 'Meta' | 'Shift'>;
}

// Response
interface ClickResponse {
  clicked: boolean;
}
```

### TUI Signature

```rust
pub struct ClickParams {
    #[serde(rename = "ref")]
    pub element_ref: String,      // @e1, @e2, etc.
    pub session: Option<String>,
}

pub struct ClickResult {
    pub success: bool,
    pub message: Option<String>,
}
```

### CLI Usage

```bash
# Browser
agent-browser click "@submit-btn"
agent-browser click "#login" --button right

# TUI
agent-tui click @e3
agent-tui click @e3 --session htop-abc123
```

### Key Differences

| Aspect | Browser | TUI |
|--------|---------|-----|
| Selector | CSS selector or @ref | Element ref only (@e1) |
| Button | left, right, middle | N/A (Enter key) |
| Click count | Configurable | Single |
| Modifiers | Alt, Control, Meta, Shift | N/A |

### Implementation Notes

Click in TUI typically means:
1. Focus the element (Tab navigation)
2. Send Enter keystroke

### JSON-RPC Example

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "click",
  "params": {
    "ref": "@e3"
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

## dblclick (Browser) - MISSING IN TUI

**Purpose**: Double-click an element

### Browser Signature

```typescript
interface DoubleClickCommand {
  action: 'dblclick';
  selector: string;
}

// Response
interface DoubleClickResponse {
  clicked: boolean;
}
```

### Recommendation

Add `dblclick` command to TUI that:
1. Focuses element
2. Sends Enter twice rapidly

```rust
// Proposed TUI signature
pub struct DblClickParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub session: Option<String>,
}
```

---

## focus (Both)

**Purpose**: Move focus to an element

### Browser Signature

```typescript
interface FocusCommand {
  action: 'focus';
  selector: string;
}

// Response
interface FocusResponse {
  focused: boolean;
}
```

### TUI Signature

```rust
pub struct FocusParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub session: Option<String>,
}

pub struct FocusResult {
    pub success: bool,
    pub message: Option<String>,
}
```

### CLI Usage

```bash
# Browser
agent-browser focus "#username"

# TUI
agent-tui focus @e2
```

### Implementation Notes

TUI focus uses Tab/Shift+Tab navigation to reach the target element.

---

## hover (Browser only)

**Purpose**: Hover mouse over an element

### Browser Signature

```typescript
interface HoverCommand {
  action: 'hover';
  selector: string;
}

// Response
interface HoverResponse {
  hovered: boolean;
}
```

### TUI Equivalent

❌ **Not applicable** - PTY terminals don't have mouse hover state.

**Alternative**: Some TUI apps respond to:
- `focus` - Moving focus may trigger similar UI updates
- Arrow keys - Moving selection in lists

---

## type (Both)

**Purpose**: Type literal text character by character

### Browser Signature

```typescript
interface TypeCommand {
  action: 'type';
  selector: string;
  text: string;
  delay?: number;      // Delay between keystrokes
  clear?: boolean;     // Clear first
}

// Response
interface TypeResponse {
  typed: boolean;
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

### CLI Usage

```bash
# Browser
agent-browser type "#search" "hello world" --delay 50

# TUI
agent-tui type "hello world"
```

### Key Differences

| Aspect | Browser | TUI |
|--------|---------|-----|
| Selector | Required | N/A (types to active terminal) |
| Delay | Configurable | Immediate |
| Clear | Optional | N/A (use `clear` command) |

### Future Enhancement

Consider adding `delay` and `clear` options to TUI:

```rust
pub struct TypeParams {
    pub text: String,
    pub session: Option<String>,
    pub delay_ms: Option<u32>,    // Delay between chars
    pub clear_first: Option<bool>, // Clear input first
}
```

---

## fill (Both)

**Purpose**: Fill an input field with a value (replaces content)

### Browser Signature

```typescript
interface FillCommand {
  action: 'fill';
  selector: string;
  value: string;
}

// Response
interface FillResponse {
  filled: boolean;
}
```

### TUI Signature

```rust
pub struct FillParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub value: String,
    pub session: Option<String>,
}

pub struct FillResult {
    pub success: bool,
    pub message: Option<String>,
}
```

### CLI Usage

```bash
# Browser
agent-browser fill "#email" "user@example.com"

# TUI
agent-tui fill @e2 "user@example.com"
```

### Parity

✅ **Identical semantics** - Both clear existing content and set new value.

---

## press (Browser) / keystroke (TUI)

**Purpose**: Send a keystroke to an element

### Browser Signature

```typescript
interface PressCommand {
  action: 'press';
  key: string;
  selector?: string;      // Optional: press on element
}

// Response
interface PressResponse {
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
agent-browser press "Enter" --selector "#submit"

# TUI
agent-tui keystroke Enter
agent-tui keystroke "Ctrl+C"
```

### Note

TUI `keystroke` always goes to active terminal, not element-scoped.

---

## clear (Both)

**Purpose**: Clear an input field

### Browser Signature

```typescript
interface ClearCommand {
  action: 'clear';
  selector: string;
}

// Response
interface ClearResponse {
  cleared: boolean;
}
```

### TUI Signature

```rust
pub struct ClearParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub session: Option<String>,
}

pub struct ClearResult {
    pub success: bool,
    pub message: Option<String>,
}
```

### Parity

✅ **Identical semantics**

---

## selectall (Browser) - MISSING IN TUI

**Purpose**: Select all text in an element

### Browser Signature

```typescript
interface SelectAllCommand {
  action: 'selectall';
  selector: string;
}

// Response
interface SelectAllResponse {
  selected: boolean;
}
```

### Recommendation

Add `selectall` command to TUI:

```rust
// Proposed TUI signature
pub struct SelectAllParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub session: Option<String>,
}
```

Implementation: Focus element, send Ctrl+A

---

## drag (Browser only)

**Purpose**: Drag and drop an element

### Browser Signature

```typescript
interface DragCommand {
  action: 'drag';
  source: string;
  target: string;
}
```

### TUI Equivalent

❌ **Not applicable** - Mouse-only operation not supported in PTY.

---

## upload (Browser only)

**Purpose**: Upload a file through file input

### Browser Signature

```typescript
interface UploadCommand {
  action: 'upload';
  selector: string;
  files: string[];
}
```

### TUI Equivalent

❌ **Not applicable** - No file picker dialogs in TUI.

**Alternative**: For TUI apps that accept file paths:
- Use `fill` to enter file path in a text field
- Use `type` to enter path at prompt
