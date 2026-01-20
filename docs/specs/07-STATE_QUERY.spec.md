# State Query Commands

Commands for querying element state and properties.

## Command Summary

| Browser Command | TUI Equivalent | Status | Notes |
|-----------------|----------------|--------|-------|
| `getattribute` | N/A | ❌ N/A | No HTML attributes |
| `gettext` | `get_text` | ✅ Exists | Identical |
| `innertext` | `get_text` | ✅ Exists | Same semantics |
| `innerhtml` | N/A | ❌ N/A | No HTML |
| `inputvalue` | `get_value` | ✅ Exists | Identical |
| `setvalue` | `fill` | ✅ Exists | Same effect |
| `isvisible` | `is_visible` | ✅ Exists | Identical |
| `isenabled` | - | ❌ Missing | Could add (disabled prop) |
| `ischecked` | - | ❌ Missing | Could add |
| `count` | - | ❌ Missing | Could add via find |
| `boundingbox` | - | ⚠️ Different | Position struct exists |
| `styles` | N/A | ❌ N/A | No CSS |

---

## get_text (TUI) / gettext (Browser)

**Purpose**: Get text content of an element

### Browser Signature

```typescript
interface GetTextCommand {
  action: 'gettext';
  selector: string;
}

// Response
interface GetTextResponse {
  text: string;
}
```

### TUI Signature

```rust
pub struct GetTextParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub session: Option<String>,
}

pub struct GetTextResult {
    pub success: bool,
    pub text: Option<String>,
    pub message: Option<String>,
}
```

### CLI Usage

```bash
# Browser
agent-browser gettext "#title"

# TUI
agent-tui get-text @e5
```

### Parity

✅ **Identical semantics** - Returns text content of element.

### JSON-RPC Example

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "get_text",
  "params": {
    "ref": "@e5"
  },
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "success": true,
    "text": "Submit Form"
  },
  "id": 1
}
```

---

## get_value (TUI) / inputvalue (Browser)

**Purpose**: Get value of an input element

### Browser Signature

```typescript
interface InputValueCommand {
  action: 'inputvalue';
  selector: string;
}

// Response
interface InputValueResponse {
  value: string;
}
```

### TUI Signature

```rust
pub struct GetValueParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub session: Option<String>,
}

pub struct GetValueResult {
    pub success: bool,
    pub value: Option<String>,
    pub message: Option<String>,
}
```

### CLI Usage

```bash
# Browser
agent-browser inputvalue "#email"

# TUI
agent-tui get-value @e3
```

### Parity

✅ **Identical semantics** - Returns current value of input field.

---

## is_visible (TUI) / isvisible (Browser)

**Purpose**: Check if element is visible on screen

### Browser Signature

```typescript
interface IsVisibleCommand {
  action: 'isvisible';
  selector: string;
}

// Response
interface IsVisibleResponse {
  visible: boolean;
}
```

### TUI Signature

```rust
pub struct IsVisibleParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub session: Option<String>,
}

pub struct IsVisibleResult {
    pub visible: bool,
    #[serde(rename = "ref")]
    pub element_ref: String,
}
```

### CLI Usage

```bash
# Browser
agent-browser isvisible "#modal"

# TUI
agent-tui is-visible @e5
```

### Parity

✅ **Identical semantics**

### JSON-RPC Example

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "is_visible",
  "params": {
    "ref": "@e5"
  },
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "visible": true,
    "ref": "@e5"
  },
  "id": 1
}
```

---

## is_focused (TUI only)

**Purpose**: Check if element currently has focus

### TUI Signature

```rust
pub struct IsFocusedParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub session: Option<String>,
}

pub struct IsFocusedResult {
    pub focused: bool,
    #[serde(rename = "ref")]
    pub element_ref: String,
}
```

### CLI Usage

```bash
agent-tui is-focused @e3
```

### Note

Browser doesn't have a direct `isfocused` command, but can check via:
```typescript
await page.evaluate(() => document.activeElement === element);
```

---

## isenabled (Browser) - MISSING IN TUI

**Purpose**: Check if element is enabled (not disabled)

### Browser Signature

```typescript
interface IsEnabledCommand {
  action: 'isenabled';
  selector: string;
}

// Response
interface IsEnabledResponse {
  enabled: boolean;
}
```

### Recommendation

Add `is_enabled` command using Element's `disabled` property:

```rust
// Proposed TUI signature
pub struct IsEnabledParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub session: Option<String>,
}

pub struct IsEnabledResult {
    pub enabled: bool,
    #[serde(rename = "ref")]
    pub element_ref: String,
}
```

### Implementation

Check the `disabled` field on the Element struct from snapshot.

---

## ischecked (Browser) - MISSING IN TUI

**Purpose**: Check if checkbox/radio is checked

### Browser Signature

```typescript
interface IsCheckedCommand {
  action: 'ischecked';
  selector: string;
}

// Response
interface IsCheckedResponse {
  checked: boolean;
}
```

### Recommendation

Add `is_checked` command using Element's `checked` property:

```rust
// Proposed TUI signature
pub struct IsCheckedParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub session: Option<String>,
}

pub struct IsCheckedResult {
    pub checked: bool,
    #[serde(rename = "ref")]
    pub element_ref: String,
}
```

### Implementation

Check the `checked` field on the Element struct from snapshot.

---

## count (Browser) - MISSING IN TUI

**Purpose**: Count elements matching a selector

### Browser Signature

```typescript
interface CountCommand {
  action: 'count';
  selector: string;
}

// Response
interface CountResponse {
  count: number;
}
```

### Recommendation

Add `count` command or extend `find` with count option:

```rust
// Option 1: Dedicated command
pub struct CountParams {
    pub role: Option<String>,
    pub name: Option<String>,
    pub text: Option<String>,
    pub session: Option<String>,
}

pub struct CountResult {
    pub count: usize,
}

// Option 2: Extend find result (already includes count)
pub struct FindResult {
    pub elements: Vec<Element>,
    pub count: usize,  // Already exists!
}
```

### Note

The current `find` command already returns `count` in its result. A dedicated `count` command would just be a convenience.

---

## boundingbox (Browser) → position (TUI)

**Purpose**: Get element position and size

### Browser Signature

```typescript
interface BoundingBoxCommand {
  action: 'boundingbox';
  selector: string;
}

// Response
interface BoundingBoxResponse {
  x: number;
  y: number;
  width: number;
  height: number;
}
```

### TUI Equivalent

Element position is included in snapshot:

```rust
pub struct Position {
    pub row: u16,
    pub col: u16,
    pub width: u16,
    pub height: Option<u16>,
}
```

### Key Differences

| Aspect | Browser | TUI |
|--------|---------|-----|
| Units | Pixels | Characters (row/col) |
| Coordinates | x, y (from top-left) | row, col |
| Size | width, height (pixels) | width, height (chars) |

### Access Pattern

```bash
# TUI - position is in snapshot result
agent-tui snapshot --format json | jq '.elements[] | select(.ref == "@e5") | .position'
```

---

## Browser-Only State Commands

### getattribute

```typescript
interface GetAttributeCommand {
  action: 'getattribute';
  selector: string;
  name: string;
}
```

❌ **Not applicable** - No HTML attributes in TUI.

### innerhtml

```typescript
interface InnerHtmlCommand {
  action: 'innerhtml';
  selector: string;
}
```

❌ **Not applicable** - No HTML in TUI.

### styles

```typescript
interface StylesCommand {
  action: 'styles';
  selector: string;
  properties: string[];
}
```

❌ **Not applicable** - No CSS in TUI.

**Alternative**: ANSI color information could be captured but is not commonly needed.

---

## Summary

| Query | Browser | TUI | Status |
|-------|---------|-----|--------|
| Get text content | `gettext` | `get_text` | ✅ |
| Get input value | `inputvalue` | `get_value` | ✅ |
| Is visible | `isvisible` | `is_visible` | ✅ |
| Is focused | (via evaluate) | `is_focused` | ✅ |
| Is enabled | `isenabled` | - | ❌ Missing |
| Is checked | `ischecked` | - | ❌ Missing |
| Count elements | `count` | (via `find`) | ⚠️ Partial |
| Get position | `boundingbox` | (via `snapshot`) | ⚠️ Different |
| Get attribute | `getattribute` | N/A | ❌ N/A |
| Get styles | `styles` | N/A | ❌ N/A |
