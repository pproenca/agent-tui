# agent-tui Wait Conditions

Synchronization patterns for reliable TUI automation.

## Wait Condition Types

| Condition | Description | Example |
|-----------|-------------|---------|
| **text** | Wait for text to appear | `wait "Success"` |
| **text_gone** | Wait for text to disappear | `wait --text-gone "Loading"` |
| **element** | Wait for element to appear | `wait --element @btn1` |
| **not_visible** | Wait for element to disappear | `wait --not-visible @modal1` |
| **focused** | Wait for element to be focused | `wait --focused @inp1` |
| **stable** | Wait for screen to stop changing | `wait --stable` |
| **value** | Wait for element to have a value | `wait --condition value --target @inp1` |

## Usage Patterns

### Wait for Text
```bash
# Simple text matching
agent-tui wait "Continue"

# With custom timeout
agent-tui wait "Done" --timeout 60000
```

### Wait for Element
```bash
# Wait for button to appear
agent-tui wait --element @btn1

# Wait for button to disappear
agent-tui wait --not-visible @btn1
```

### Wait for Screen Stability
```bash
# Wait until screen stops changing (for dynamic UIs)
agent-tui wait --stable

# With timeout
agent-tui wait --stable --timeout 30000
```

### Wait for Focus
```bash
# Wait until specific element is focused
agent-tui wait --focused @inp1
```

## Timeout Configuration

Default timeout: **30000ms** (30 seconds)

```bash
# Short timeout for fast operations
agent-tui wait "prompt" --timeout 5000

# Long timeout for slow operations
agent-tui wait "Complete" --timeout 180000
```

## Return Values

Successful wait returns:
```json
{
  "found": true,
  "elapsed_ms": 1500,
  "matched_text": "Success",
  "element_ref": "@btn1"
}
```

Failed wait returns:
```json
{
  "found": false,
  "elapsed_ms": 30000,
  "screen_context": "Current screen content...",
  "suggestion": "Try waiting for different text or check the screen content"
}
```

## Best Practices

### 1. Always Wait After Interactions
```bash
# WRONG - may fail if UI hasn't updated
agent-tui fill @inp1 "value"
agent-tui click @btn1  # Element may have moved

# CORRECT - wait for UI to settle
agent-tui fill @inp1 "value"
agent-tui wait --stable
agent-tui snapshot -i
agent-tui click @btn1
```

### 2. Use Stable for Dynamic UIs
```bash
# For UIs that update continuously
agent-tui wait --stable --timeout 30000

# Instead of arbitrary text that might not appear
agent-tui wait "Some text"  # May timeout
```

### 3. Chain Waits for Multi-Step Operations
```bash
agent-tui type "npm install"
agent-tui keystroke Enter
agent-tui wait "added"
agent-tui wait --stable
```

### 4. Handle Timeout Gracefully
```bash
# Check if wait succeeded
if agent-tui wait "Success" --timeout 10000; then
  echo "Operation succeeded"
else
  # Take snapshot to see what happened
  agent-tui snapshot -i
  echo "Operation may have failed"
fi
```

### 5. Appropriate Timeouts by Operation

| Operation | Suggested Timeout |
|-----------|-------------------|
| UI navigation | 5000ms |
| Form submission | 10000ms |
| npm install | 120000ms |
| Build processes | 180000ms |
| Network operations | 60000ms |

## Common Mistakes

### Wrong: Not Waiting
```bash
agent-tui spawn "npm init"
agent-tui fill @inp1 "my-app"  # May fail - UI not ready
```

### Correct: Wait for Readiness
```bash
agent-tui spawn "npm init"
agent-tui wait "package name"
agent-tui snapshot -i
agent-tui fill @inp1 "my-app"
```

### Wrong: Hardcoded Sleep
```bash
sleep 5  # Wastes time or may not be enough
```

### Correct: Event-Based Wait
```bash
agent-tui wait --stable  # Waits exactly as long as needed
```
