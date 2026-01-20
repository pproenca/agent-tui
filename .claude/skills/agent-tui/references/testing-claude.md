# Testing Claude Code with agent-tui

agent-tui can automate Claude Code itself for testing and verification.

## Basic Claude Code Interaction

```bash
# Spawn Claude Code in permissive mode
agent-tui spawn "claude --dangerously-skip-permissions"

# Wait for initialization (the ">" prompt)
agent-tui wait ">" --timeout 120000

# Send a prompt
agent-tui type "Create hello.py that prints Hello World"
agent-tui keystroke Enter

# Wait for completion (screen stabilizes when Claude finishes)
agent-tui wait --stable --timeout 60000

# Verify result
agent-tui snapshot -i
cat hello.py

# Cleanup
agent-tui kill
```

## CI/CD Integration

```bash
#!/bin/bash
# test-claude-code.sh - Automated Claude Code testing
set -e

echo "Starting Claude Code test..."
agent-tui spawn "claude --dangerously-skip-permissions" --cols 120 --rows 40

# Wait for prompt with generous timeout
agent-tui wait ">" --timeout 120000

# Send test task
agent-tui type "Write tests for the auth module"
agent-tui keystroke Enter

# Wait for completion
agent-tui wait --stable --timeout 180000

# Capture final state
agent-tui snapshot > output.txt

# Cleanup
agent-tui kill

# Verify output
grep -q "test" output.txt && echo "SUCCESS" || echo "FAILED"
```

## Multi-Turn Conversation

```bash
# Start Claude Code
agent-tui spawn "claude --dangerously-skip-permissions"
agent-tui wait ">" --timeout 120000

# First prompt
agent-tui type "Create a Python function that reverses a string"
agent-tui keystroke Enter
agent-tui wait --stable --timeout 60000

# Follow-up
agent-tui type "Now add unit tests for that function"
agent-tui keystroke Enter
agent-tui wait --stable --timeout 60000

# Verify files were created
ls -la *.py

# Cleanup
agent-tui kill
```

## Using Tree Format for Verification

```bash
agent-tui spawn "claude --dangerously-skip-permissions"
agent-tui wait ">"

agent-tui type "Create a simple TODO app"
agent-tui keystroke Enter
agent-tui wait --stable

# Get structured view of what Claude did
agent-tui snapshot -i --format tree

agent-tui kill
```

## Error Handling

```bash
#!/bin/bash
set -e

cleanup() {
  agent-tui kill 2>/dev/null || true
}
trap cleanup EXIT

agent-tui spawn "claude --dangerously-skip-permissions"

if ! agent-tui wait ">" --timeout 120000; then
  echo "Failed to initialize Claude Code"
  agent-tui snapshot  # Capture state for debugging
  exit 1
fi

agent-tui type "Your task here"
agent-tui keystroke Enter

if ! agent-tui wait --stable --timeout 180000; then
  echo "Task timed out"
  agent-tui snapshot
  exit 1
fi

echo "Task completed successfully"
```

## Testing Specific Features

### Test File Creation
```bash
agent-tui spawn "claude --dangerously-skip-permissions"
agent-tui wait ">"
agent-tui type "Create a file called test.txt with 'Hello World'"
agent-tui keystroke Enter
agent-tui wait --stable

# Verify
test -f test.txt && grep -q "Hello World" test.txt && echo "PASS"
agent-tui kill
```

### Test Code Execution
```bash
agent-tui spawn "claude --dangerously-skip-permissions"
agent-tui wait ">"
agent-tui type "Run: echo 'test output'"
agent-tui keystroke Enter
agent-tui wait "test output"
agent-tui kill
```

### Test Interactive Prompts
```bash
agent-tui spawn "claude --dangerously-skip-permissions"
agent-tui wait ">"
agent-tui type "Ask me a question about my project"
agent-tui keystroke Enter
agent-tui wait "?" --timeout 30000  # Wait for question mark
agent-tui snapshot -i --format tree  # See the question
agent-tui kill
```

## Best Practices

1. **Use generous timeouts** - Claude operations can take time
2. **Always cleanup** - Use trap to ensure `agent-tui kill` runs
3. **Capture state on failure** - Use `snapshot` before exit on errors
4. **Use permissive mode** - `--dangerously-skip-permissions` for automation
5. **Wait for stability** - Use `--stable` instead of arbitrary text
6. **Verify results externally** - Check files, run tests after Claude completes
7. **Set appropriate terminal size** - Larger terminals for complex output

## Environment Variables

```bash
# Increase terminal size for better output visibility
export AGENT_TUI_COLS=160
export AGENT_TUI_ROWS=50

# Use TCP transport if needed
export AGENT_TUI_TRANSPORT=tcp
export AGENT_TUI_TCP_PORT=19847
```
