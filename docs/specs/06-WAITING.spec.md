# Waiting Commands

Commands for waiting on conditions before proceeding.

## Command Summary

| Browser Command | TUI Equivalent | Status | Notes |
|-----------------|----------------|--------|-------|
| `wait` | `wait` | ✅ Exists | Identical |
| `waitforurl` | N/A | ❌ N/A | No URLs |
| `waitforloadstate` | `wait --stable` | ✅ Exists | Screen stability |
| `waitforfunction` | N/A | ❌ N/A | No JS eval |
| `waitfordownload` | N/A | ❌ N/A | No downloads |
| `waitforselector` | `wait --element` | ✅ Exists | Wait for element |
| `waitfornavigation` | N/A | ❌ N/A | No navigation |

---

## wait (Both)

**Purpose**: Wait for a condition to be met

### Browser Signature

```typescript
interface WaitCommand {
  action: 'wait';
  selector?: string;
  timeout?: number;
  state?: 'attached' | 'detached' | 'visible' | 'hidden';
}

// Response
interface WaitResponse {
  waited: boolean;
  elapsed: number;
}
```

### TUI Signature

```rust
pub struct WaitParams {
    pub text: Option<String>,           // Wait for text to appear
    pub timeout_ms: Option<u64>,        // Default: 30000
    pub session: Option<String>,
    pub condition: Option<WaitCondition>,
    pub target: Option<String>,         // Element ref or text pattern
}

pub enum WaitCondition {
    Text,           // Wait for text to appear
    Element,        // Wait for element ref to exist
    Focused,        // Wait for element to be focused
    NotVisible,     // Wait for element to disappear
    Stable,         // Wait for screen to stop changing
    TextGone,       // Wait for text to disappear
    Value,          // Wait for input to have specific value
}

pub struct WaitResult {
    pub found: bool,
    pub elapsed_ms: u64,
    pub screen_context: Option<String>,   // Screen state at match
    pub suggestion: Option<String>,       // Helpful hint on timeout
    pub matched_text: Option<String>,     // What was found
    pub element_ref: Option<String>,      // Element that matched
}
```

### CLI Usage

```bash
# Browser
agent-browser wait --selector "#loading" --state hidden
agent-browser wait --selector ".result" --state visible

# TUI
agent-tui wait "Loading complete"
agent-tui wait --condition stable
agent-tui wait --condition element --target @e5
agent-tui wait --condition text-gone --target "Please wait"
```

### Key Differences

| Aspect | Browser | TUI |
|--------|---------|-----|
| Primary target | CSS selector | Text or element ref |
| States | attached, detached, visible, hidden | text, element, focused, stable, etc. |
| Extra info | elapsed time | screen_context, suggestion |

### Wait Conditions Explained

#### `text` (default)
Wait for specific text to appear on screen.

```bash
agent-tui wait "Welcome"
agent-tui wait --condition text --target "Ready"
```

#### `element`
Wait for an element reference to exist in snapshot.

```bash
agent-tui wait --condition element --target @e5
```

#### `focused`
Wait for specific element to receive focus.

```bash
agent-tui wait --condition focused --target @e3
```

#### `not-visible`
Wait for element or text to disappear.

```bash
agent-tui wait --condition not-visible --target @e5
```

#### `stable`
Wait for screen to stop changing (no updates for N ms).

```bash
agent-tui wait --condition stable
agent-tui wait --condition stable --timeout-ms 5000
```

#### `text-gone`
Wait for specific text to disappear.

```bash
agent-tui wait --condition text-gone --target "Loading..."
```

#### `value`
Wait for input element to have specific value.

```bash
agent-tui wait --condition value --target @e2 --text "expected value"
```

### JSON-RPC Example

```json
// Request - wait for text
{
  "jsonrpc": "2.0",
  "method": "wait",
  "params": {
    "text": "Ready",
    "timeout_ms": 10000
  },
  "id": 1
}

// Response - success
{
  "jsonrpc": "2.0",
  "result": {
    "found": true,
    "elapsed_ms": 1523,
    "matched_text": "Ready"
  },
  "id": 1
}

// Response - timeout
{
  "jsonrpc": "2.0",
  "result": {
    "found": false,
    "elapsed_ms": 10000,
    "screen_context": "Current screen shows: Loading...",
    "suggestion": "Text 'Ready' not found. Screen shows 'Loading...'. The operation may still be in progress."
  },
  "id": 1
}
```

### Request - wait for stable

```json
{
  "jsonrpc": "2.0",
  "method": "wait",
  "params": {
    "condition": "stable",
    "timeout_ms": 5000
  },
  "id": 1
}
```

---

## waitforloadstate (Browser) → wait --stable (TUI)

**Purpose**: Wait for page/screen to finish loading

### Browser Signature

```typescript
interface WaitForLoadStateCommand {
  action: 'waitforloadstate';
  state: 'load' | 'domcontentloaded' | 'networkidle';
  timeout?: number;
}
```

### TUI Equivalent

```bash
agent-tui wait --condition stable
```

The `stable` condition waits until the screen content stops changing, which is analogous to browser's load state.

### Implementation

TUI stability detection:
1. Take snapshot
2. Wait short interval (e.g., 100ms)
3. Take another snapshot
4. If identical, wait longer interval
5. If still identical after threshold (e.g., 500ms stable), return success
6. If different, reset and repeat

---

## waitforselector (Browser) → wait --element (TUI)

**Purpose**: Wait for element to appear

### Browser Signature

```typescript
interface WaitForSelectorCommand {
  action: 'waitforselector';
  selector: string;
  state?: 'attached' | 'detached' | 'visible' | 'hidden';
  timeout?: number;
}
```

### TUI Equivalent

```bash
# Wait for element to appear
agent-tui wait --condition element --target @e5

# Wait for element to disappear
agent-tui wait --condition not-visible --target @e5
```

---

## Browser-Only Wait Commands

### waitforurl

```typescript
interface WaitForUrlCommand {
  action: 'waitforurl';
  url: string | RegExp;
  timeout?: number;
}
```

❌ **Not applicable** - No URLs in TUI.

### waitforfunction

```typescript
interface WaitForFunctionCommand {
  action: 'waitforfunction';
  function: string;  // JavaScript function
  polling?: number;
  timeout?: number;
}
```

❌ **Not applicable** - No JavaScript evaluation in TUI.

### waitfordownload

```typescript
interface WaitForDownloadCommand {
  action: 'waitfordownload';
  timeout?: number;
}
```

❌ **Not applicable** - No download events in TUI.

### waitfornavigation

```typescript
interface WaitForNavigationCommand {
  action: 'waitfornavigation';
  url?: string | RegExp;
  waitUntil?: 'load' | 'domcontentloaded' | 'networkidle';
  timeout?: number;
}
```

❌ **Not applicable** - No page navigation in TUI.

---

## Best Practices

### Use Appropriate Conditions

| Scenario | Condition |
|----------|-----------|
| Waiting for success message | `text` |
| Waiting for dialog to appear | `element` |
| Waiting for spinner to finish | `text-gone` or `not-visible` |
| Waiting for initial render | `stable` |
| Waiting for input to be ready | `focused` |

### Set Reasonable Timeouts

```bash
# Quick operations (UI updates)
agent-tui wait "Done" --timeout-ms 5000

# Longer operations (data loading)
agent-tui wait "Results loaded" --timeout-ms 30000

# Very long operations
agent-tui wait "Processing complete" --timeout-ms 120000
```

### Handle Timeout Gracefully

The `suggestion` field in timeout responses helps diagnose issues:

```json
{
  "found": false,
  "elapsed_ms": 30000,
  "suggestion": "Text 'Success' not found. Screen shows error: 'Connection failed'. Check network connectivity."
}
```

---

## Summary

| Browser | TUI | Purpose |
|---------|-----|---------|
| `wait --selector X --state visible` | `wait --condition element --target @e1` | Element appears |
| `wait --selector X --state hidden` | `wait --condition not-visible --target @e1` | Element disappears |
| `waitforloadstate networkidle` | `wait --condition stable` | Screen stops changing |
| `wait --selector X --state attached` | `wait "text"` | Content appears |
| N/A | `wait --condition focused --target @e1` | Element gets focus |
| N/A | `wait --condition text-gone --target "text"` | Text disappears |
