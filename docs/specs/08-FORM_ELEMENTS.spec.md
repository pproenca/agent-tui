# Form Element Commands

Commands for interacting with form controls.

## Command Summary

| Browser Command | TUI Equivalent | Status | Notes |
|-----------------|----------------|--------|-------|
| `check` | `toggle` | ✅ Exists | Set to checked |
| `uncheck` | `toggle` | ✅ Exists | Set to unchecked |
| `select` | `select` | ✅ Exists | Identical |
| `multiselect` | - | ❌ Missing | Could add |

---

## toggle (TUI) / check + uncheck (Browser)

**Purpose**: Toggle or set checkbox/radio state

### Browser Signature

Browser has separate commands:

```typescript
interface CheckCommand {
  action: 'check';
  selector: string;
}

interface UncheckCommand {
  action: 'uncheck';
  selector: string;
}

// Response
interface CheckResponse {
  checked: boolean;
}
```

### TUI Signature

TUI combines into a single toggle command:

```rust
pub struct ToggleParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub session: Option<String>,
    pub state: Option<bool>,  // Optional: force specific state
}

pub struct ToggleResult {
    pub success: bool,
    pub message: Option<String>,
    pub checked: Option<bool>,     // New state after toggle
}
```

### CLI Usage

```bash
# Browser - separate commands
agent-browser check "#agree-terms"
agent-browser uncheck "#newsletter"

# TUI - unified toggle
agent-tui toggle @e5           # Toggle current state
agent-tui toggle @e5 --state true   # Force checked
agent-tui toggle @e5 --state false  # Force unchecked
```

### Key Differences

| Aspect | Browser | TUI |
|--------|---------|-----|
| Commands | Separate `check`/`uncheck` | Unified `toggle` |
| Force state | Implicit in command | Explicit `--state` option |
| Return | `checked: boolean` | `checked: Option<bool>` |

### JSON-RPC Example

```json
// Request - toggle
{
  "jsonrpc": "2.0",
  "method": "toggle",
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
    "checked": true
  },
  "id": 1
}
```

### Recommendation

Consider adding explicit `check`/`uncheck` aliases for browser parity:

```rust
// Aliases for browser compatibility
pub type CheckParams = ToggleParams;  // with state: Some(true)
pub type UncheckParams = ToggleParams;  // with state: Some(false)
```

---

## select (Both)

**Purpose**: Select an option from a dropdown/list

### Browser Signature

```typescript
interface SelectCommand {
  action: 'select';
  selector: string;
  values: string | string[];  // Option value(s)
}

// Response
interface SelectResponse {
  selected: string[];
}
```

### TUI Signature

```rust
pub struct SelectParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub option: String,           // Single option to select
    pub session: Option<String>,
}

pub struct SelectResult {
    pub success: bool,
    pub message: Option<String>,
    pub selected_option: Option<String>,
}
```

### CLI Usage

```bash
# Browser
agent-browser select "#country" "United States"
agent-browser select "#colors" '["red", "blue"]'  # Multi-select

# TUI
agent-tui select @e3 "United States"
```

### Key Differences

| Aspect | Browser | TUI |
|--------|---------|-----|
| Multi-select | `values: string[]` | Single option only |
| Selection | By value or label | By visible text |

### JSON-RPC Example

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "select",
  "params": {
    "ref": "@e3",
    "option": "Option 2"
  },
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "success": true,
    "selected_option": "Option 2"
  },
  "id": 1
}
```

---

## multiselect (Browser) - MISSING IN TUI

**Purpose**: Select multiple options from a list

### Browser Signature

```typescript
interface SelectCommand {
  action: 'select';
  selector: string;
  values: string[];  // Multiple values
}

// Response
interface SelectResponse {
  selected: string[];
}
```

### Recommendation

Add `multiselect` command or extend `select`:

```rust
// Option 1: Extend select with multiple options
pub struct SelectParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub option: Option<String>,        // Single option
    pub options: Option<Vec<String>>,  // Multiple options
    pub session: Option<String>,
}

// Option 2: Dedicated multiselect command
pub struct MultiSelectParams {
    #[serde(rename = "ref")]
    pub element_ref: String,
    pub options: Vec<String>,
    pub session: Option<String>,
}

pub struct MultiSelectResult {
    pub success: bool,
    pub selected_options: Vec<String>,
    pub message: Option<String>,
}
```

### Implementation Notes

Multi-select in TUI typically involves:
1. Focus the list element
2. Navigate with arrow keys
3. Press Space to toggle selection
4. Repeat for each option

---

## Form Detection in TUI

TUI detection identifies these form element types:

### Checkbox

Detected patterns:
- `[x]`, `[X]` - Checked
- `[ ]` - Unchecked
- `[*]` - Checked (alternative)

```
[ ] Remember me
[x] Accept terms
```

### Radio Button

Detected patterns:
- `(*)`, `(o)` - Selected
- `( )` - Not selected

```
( ) Option A
(*) Option B
( ) Option C
```

### Select/Dropdown

Detected by:
- Visible current selection
- Arrow indicator suggesting dropdown
- Focus indicators

```
Country: [United States    ▼]
```

### List (for selection)

Detected by:
- Multiple items in vertical arrangement
- Selection indicator (highlight, cursor, `>`)

```
> Item 1
  Item 2
  Item 3
```

---

## Best Practices

### Checking Element Type

Before interacting, verify element type via snapshot:

```bash
# Get element info
agent-tui snapshot --format json | jq '.elements[] | select(.ref == "@e5")'
```

Returns:
```json
{
  "ref": "@e5",
  "role": "checkbox",
  "name": "Accept terms",
  "checked": false,
  "focused": false
}
```

### Handling Different Controls

| Element Type | Role | Command |
|--------------|------|---------|
| Checkbox | `checkbox` | `toggle` |
| Radio button | `radiobutton` | `toggle` or `click` |
| Dropdown | `combobox` | `select` |
| List item | `listitem` | `click` |
| Button | `button` | `click` |

### Waiting After Selection

Some TUI apps update dynamically after selection:

```bash
# Select option
agent-tui select @e3 "Option 2"

# Wait for UI update
agent-tui wait --condition stable
```

---

## Summary

| Form Action | Browser | TUI | Status |
|-------------|---------|-----|--------|
| Check checkbox | `check` | `toggle --state true` | ✅ |
| Uncheck checkbox | `uncheck` | `toggle --state false` | ✅ |
| Toggle checkbox | N/A | `toggle` | ✅ |
| Select option | `select` | `select` | ✅ |
| Multi-select | `select` (array) | - | ❌ Missing |
| Click radio | `check` | `toggle` or `click` | ✅ |
