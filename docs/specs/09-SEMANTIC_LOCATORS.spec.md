# Semantic Locator Commands

Commands for finding elements by semantic properties.

## Command Summary

| Browser Command | TUI Equivalent | Status | Notes |
|-----------------|----------------|--------|-------|
| `getbyrole` | `find --role` | ✅ Exists | Identical |
| `getbytext` | `find --text` | ✅ Exists | Identical |
| `getbylabel` | `find --name` | ✅ Exists | Label = name |
| `getbyplaceholder` | - | ⚠️ Partial | Placeholder prop exists |
| `getbyalttext` | N/A | ❌ N/A | No images |
| `getbytitle` | N/A | ❌ N/A | No title attribute |
| `getbytestid` | N/A | ❌ N/A | No data-testid |
| `nth` | - | ❌ Missing | Could add index support |

---

## find (TUI) / getby* (Browser)

**Purpose**: Find elements by semantic properties

### Browser Approach

Browser has multiple separate commands:

```typescript
interface GetByRoleCommand {
  action: 'getbyrole';
  role: string;
  name?: string;
  subaction: 'click' | 'fill' | 'check' | 'hover';
  value?: string;
}

interface GetByTextCommand {
  action: 'getbytext';
  text: string;
  exact?: boolean;
  subaction: 'click' | 'hover';
}

interface GetByLabelCommand {
  action: 'getbylabel';
  label: string;
  subaction: 'click' | 'fill' | 'check';
  value?: string;
}

interface GetByPlaceholderCommand {
  action: 'getbyplaceholder';
  placeholder: string;
  subaction: 'click' | 'fill';
  value?: string;
}
```

### TUI Approach

TUI consolidates into a single `find` command:

```rust
pub struct FindParams {
    pub session: Option<String>,
    pub role: Option<String>,      // Filter by role
    pub name: Option<String>,      // Filter by accessible name
    pub text: Option<String>,      // Filter by text content
    pub focused: Option<bool>,     // Filter by focus state
}

pub struct FindResult {
    pub elements: Vec<Element>,
    pub count: usize,
}

pub struct Element {
    #[serde(rename = "ref")]
    pub element_ref: String,        // @e1, @e2, etc.
    pub role: String,               // button, textbox, etc.
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
# Browser - combined find + action
agent-browser getbyrole button --name "Submit" --subaction click
agent-browser getbytext "Login" --subaction click
agent-browser getbylabel "Email" --subaction fill --value "test@example.com"

# TUI - separate find, then action
agent-tui find --role button --name "Submit"
# Returns @e5
agent-tui click @e5

agent-tui find --text "Login"
# Returns @e3
agent-tui click @e3

agent-tui find --name "Email"
# Returns @e2
agent-tui fill @e2 "test@example.com"
```

### Key Differences

| Aspect | Browser | TUI |
|--------|---------|-----|
| Commands | Multiple getby* | Single `find` |
| Action | Combined (subaction) | Separate command |
| Return | Direct action result | Element refs for subsequent use |
| Exact match | `exact` option | Substring match by default |

### Advantages of TUI Approach

1. **Composable**: Find once, use ref multiple times
2. **Debuggable**: See what was found before acting
3. **Flexible**: Combine multiple filters
4. **Explicit**: Clear separation of find and act

---

## Role-Based Finding

### Supported Roles

| Role | Description | Detection |
|------|-------------|-----------|
| `button` | Clickable buttons | Bracketed text, F-keys |
| `textbox` | Text input fields | Cursor, borders |
| `checkbox` | Toggle checkboxes | `[x]`, `[ ]` patterns |
| `radiobutton` | Radio options | `(*)`, `( )` patterns |
| `menuitem` | Menu entries | Arrows, highlighting |
| `listitem` | List entries | Consistent indentation |
| `progressbar` | Progress indicators | Bar characters |
| `tab` | Tab controls | Tab-like patterns |
| `dialog` | Modal dialogs | Border detection |
| `combobox` | Dropdowns | Selection + arrow |
| `link` | Hyperlinks | URL patterns, underline |

### Examples

```bash
# Find all buttons
agent-tui find --role button

# Find button with specific name
agent-tui find --role button --name "Submit"

# Find focused element
agent-tui find --focused true

# Find text input by name/label
agent-tui find --role textbox --name "Username"
```

---

## Text-Based Finding

### find --text

```bash
# Find element containing text
agent-tui find --text "Welcome"

# Find element with partial text
agent-tui find --text "Log"  # Matches "Login", "Logout", etc.
```

### Implementation Notes

- Text matching is case-insensitive by default
- Searches element names, values, and visible text
- Returns all matching elements

### Future Enhancement: Exact Match

Consider adding `exact` option for precise matching:

```rust
pub struct FindParams {
    // ... existing fields
    pub exact: Option<bool>,  // Exact text match
}
```

```bash
agent-tui find --text "Log" --exact  # Only exact match
```

---

## Name/Label Finding

### find --name

The `name` field corresponds to accessible name, which in TUI often comes from:
- Label text adjacent to input
- Button text
- Menu item text

```bash
# Find by accessible name
agent-tui find --name "Email Address"

# Combine with role for precision
agent-tui find --role textbox --name "Email"
```

### Relationship to Browser's getbylabel

Browser's `getbylabel` finds inputs by their associated `<label>` element.

In TUI, labels are detected by:
- Text directly before/above input fields
- Text with `:` suffix adjacent to inputs
- Consistent patterns in form layouts

---

## Combining Filters

Multiple filters are AND-combined:

```bash
# Find checkbox that is checked
agent-tui find --role checkbox | jq 'select(.checked == true)'

# Find disabled buttons
agent-tui find --role button | jq 'select(.disabled == true)'

# Find focused textbox
agent-tui find --role textbox --focused true
```

---

## JSON-RPC Example

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "find",
  "params": {
    "role": "button",
    "name": "Submit"
  },
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "elements": [
      {
        "ref": "@e5",
        "role": "button",
        "name": "Submit",
        "focused": false,
        "disabled": false,
        "position": { "row": 20, "col": 40, "width": 8 }
      }
    ],
    "count": 1
  },
  "id": 1
}
```

---

## nth (Browser) - MISSING IN TUI

**Purpose**: Select nth matching element

### Browser Usage

```typescript
// Get second button
page.getByRole('button').nth(1)  // 0-indexed
```

### Recommendation

Add `nth` parameter to find:

```rust
pub struct FindParams {
    // ... existing fields
    pub nth: Option<usize>,  // Select nth result (0-indexed)
}
```

Or handle in CLI with index:

```bash
# Get second matching button
agent-tui find --role button --nth 1
```

### Alternative: Array Indexing

Since find returns array, can use jq:

```bash
agent-tui find --role button --format json | jq '.[1]'
```

---

## Browser-Only Locators

### getbyplaceholder

```typescript
interface GetByPlaceholderCommand {
  action: 'getbyplaceholder';
  placeholder: string;
}
```

⚠️ **Partial support** - TUI elements can have placeholder detection but not commonly used.

### getbyalttext

```typescript
interface GetByAltTextCommand {
  action: 'getbyalttext';
  alt: string;
}
```

❌ **Not applicable** - No images in TUI.

### getbytitle

```typescript
interface GetByTitleCommand {
  action: 'getbytitle';
  title: string;
}
```

❌ **Not applicable** - No title attribute in TUI.

### getbytestid

```typescript
interface GetByTestIdCommand {
  action: 'getbytestid';
  testId: string;
}
```

❌ **Not applicable** - No data-testid in TUI.

---

## Best Practices

### Prefer Specific Locators

```bash
# Good: Specific
agent-tui find --role button --name "Submit Form"

# Avoid: Too broad
agent-tui find --text "Submit"  # May match multiple elements
```

### Verify Before Acting

```bash
# Find first, verify count
result=$(agent-tui find --role button --name "Delete")
count=$(echo "$result" | jq '.count')
if [ "$count" -eq 1 ]; then
  ref=$(echo "$result" | jq -r '.elements[0].ref')
  agent-tui click "$ref"
fi
```

### Use Role + Name for Reliability

```bash
# Most reliable pattern
agent-tui find --role textbox --name "Username"
agent-tui find --role button --name "Login"
```

---

## Summary

| Locator Strategy | Browser | TUI | Status |
|------------------|---------|-----|--------|
| By role | `getbyrole` | `find --role` | ✅ |
| By text | `getbytext` | `find --text` | ✅ |
| By label/name | `getbylabel` | `find --name` | ✅ |
| By placeholder | `getbyplaceholder` | - | ⚠️ Partial |
| By index | `.nth(n)` | - | ❌ Missing |
| By test ID | `getbytestid` | N/A | ❌ N/A |
| Exact match | `exact: true` | - | ❌ Missing |
| Combined action | `subaction` | Separate commands | Different approach |
