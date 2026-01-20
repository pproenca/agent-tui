# QA Testing: agent-tui CLI

You are a QA engineer testing the **agent-tui** CLI tool. Your job is to systematically test all commands, find bugs, and report them clearly.

## Setup

### Paths
- **CLI binary**: `./cli/target/release/agent-tui`
- **Daemon**: `node daemon/dist/index.js`

### Before Testing

1. **Ensure the daemon is running:**
   ```bash
   # Check if daemon is running
   pgrep -f "node daemon/dist/index.js"

   # If not running, start it in background:
   cd daemon && node dist/index.js &
   ```

2. **Ensure CLI is built:**
   ```bash
   cd cli && cargo build --release
   ```

3. **Fix spawn-helper permissions (if needed):**
   ```bash
   chmod +x daemon/spawn-helper
   ```

## CLI Commands Reference

| Command | Description | Key Options |
|---------|-------------|-------------|
| `spawn <cmd>` | Start a TUI application | `--cols`, `--rows`, `-d/--cwd` |
| `snapshot` | Capture screen state | `-i` for interactive elements |
| `click <ref>` | Click element by ref (e.g., @btn1) | |
| `fill <ref> <value>` | Fill input element | |
| `keystroke <key>` | Send keystroke | Enter, Tab, Ctrl+C, ArrowDown, etc. |
| `type <text>` | Type literal text | |
| `wait <text>` | Wait for text to appear | `-t/--timeout <ms>` |
| `kill` | Terminate current session | |
| `sessions` | List active sessions | |

**Global Options:** `-s/--session <id>`, `-f/--format <text|json>`

---

## Test Plan

### Phase 1: Basic Functionality

#### Test 1.1: Spawn and Kill
```bash
# Spawn a simple bash session
./cli/target/release/agent-tui spawn bash

# Verify session is running
./cli/target/release/agent-tui sessions

# Kill the session
./cli/target/release/agent-tui kill

# Verify session is gone
./cli/target/release/agent-tui sessions
```

**Expected:** Session starts, appears in list, terminates cleanly.

#### Test 1.2: Type and Snapshot
```bash
# Spawn bash
./cli/target/release/agent-tui spawn bash

# Type a command
./cli/target/release/agent-tui type "echo hello world"
./cli/target/release/agent-tui keystroke Enter

# Take snapshot
./cli/target/release/agent-tui snapshot

# Clean up
./cli/target/release/agent-tui kill
```

**Expected:** Snapshot shows "hello world" output.

#### Test 1.3: Wait Command
```bash
# Spawn bash
./cli/target/release/agent-tui spawn bash

# Type command that produces output
./cli/target/release/agent-tui type "sleep 1 && echo DONE"
./cli/target/release/agent-tui keystroke Enter

# Wait for output
./cli/target/release/agent-tui wait "DONE" --timeout 5000

# Clean up
./cli/target/release/agent-tui kill
```

**Expected:** Wait completes successfully after ~1 second.

### Phase 2: Interactive Elements

#### Test 2.1: Snapshot with Elements
```bash
# Spawn an interactive app (htop, vim, or similar)
./cli/target/release/agent-tui spawn htop

# Snapshot with elements
./cli/target/release/agent-tui snapshot -i

# Clean up
./cli/target/release/agent-tui keystroke q
./cli/target/release/agent-tui kill
```

**Expected:** Elements list shows interactive items with refs like @btn1.

#### Test 2.2: Click Element
```bash
# Spawn an interactive app
./cli/target/release/agent-tui spawn htop

# Get elements
./cli/target/release/agent-tui snapshot -i

# Click an element (use ref from snapshot)
./cli/target/release/agent-tui click @btn1

# Verify state changed
./cli/target/release/agent-tui snapshot

# Clean up
./cli/target/release/agent-tui kill
```

#### Test 2.3: Fill Element
```bash
# Spawn vim (creates input scenario)
./cli/target/release/agent-tui spawn vim

# Enter insert mode
./cli/target/release/agent-tui keystroke i

# Type some text
./cli/target/release/agent-tui type "Hello from agent-tui"

# Exit without saving
./cli/target/release/agent-tui keystroke Escape
./cli/target/release/agent-tui type ":q!"
./cli/target/release/agent-tui keystroke Enter
```

### Phase 3: Multiple Sessions

#### Test 3.1: Concurrent Sessions
```bash
# Start first session
./cli/target/release/agent-tui spawn bash -s session1

# Start second session
./cli/target/release/agent-tui spawn bash -s session2

# List sessions (should show 2)
./cli/target/release/agent-tui sessions

# Type in each session
./cli/target/release/agent-tui -s session1 type "echo session1"
./cli/target/release/agent-tui -s session1 keystroke Enter

./cli/target/release/agent-tui -s session2 type "echo session2"
./cli/target/release/agent-tui -s session2 keystroke Enter

# Snapshot each
./cli/target/release/agent-tui -s session1 snapshot
./cli/target/release/agent-tui -s session2 snapshot

# Clean up
./cli/target/release/agent-tui -s session1 kill
./cli/target/release/agent-tui -s session2 kill
```

**Expected:** Sessions are independent, each shows its own output.

### Phase 4: Output Formats

#### Test 4.1: JSON Output
```bash
./cli/target/release/agent-tui spawn bash
./cli/target/release/agent-tui -f json sessions
./cli/target/release/agent-tui -f json snapshot
./cli/target/release/agent-tui kill
```

**Expected:** Valid JSON output for all commands.

### Phase 5: Edge Cases & Error Handling

#### Test 5.1: Invalid Session Reference
```bash
./cli/target/release/agent-tui -s nonexistent snapshot
```

**Expected:** Clear error message about invalid session.

#### Test 5.2: Invalid Element Reference
```bash
./cli/target/release/agent-tui spawn bash
./cli/target/release/agent-tui click @invalid999
./cli/target/release/agent-tui kill
```

**Expected:** Clear error about element not found.

#### Test 5.3: Wait Timeout
```bash
./cli/target/release/agent-tui spawn bash
./cli/target/release/agent-tui wait "THIS_WILL_NEVER_APPEAR" --timeout 2000
./cli/target/release/agent-tui kill
```

**Expected:** Times out with clear message after 2 seconds.

#### Test 5.4: Special Characters
```bash
./cli/target/release/agent-tui spawn bash
./cli/target/release/agent-tui type "echo 'quotes' \"double\" \$VAR"
./cli/target/release/agent-tui keystroke Enter
./cli/target/release/agent-tui snapshot
./cli/target/release/agent-tui kill
```

**Expected:** Special characters handled correctly.

#### Test 5.5: Keystroke Variations
```bash
./cli/target/release/agent-tui spawn bash

# Test various keystrokes
./cli/target/release/agent-tui keystroke Tab
./cli/target/release/agent-tui keystroke ArrowUp
./cli/target/release/agent-tui keystroke ArrowDown
./cli/target/release/agent-tui keystroke ArrowLeft
./cli/target/release/agent-tui keystroke ArrowRight
./cli/target/release/agent-tui keystroke Backspace
./cli/target/release/agent-tui keystroke Delete
./cli/target/release/agent-tui keystroke Home
./cli/target/release/agent-tui keystroke End
./cli/target/release/agent-tui keystroke "Ctrl+C"
./cli/target/release/agent-tui keystroke "Ctrl+D"

./cli/target/release/agent-tui kill
```

**Expected:** All keystrokes sent without error.

#### Test 5.6: Rapid Commands
```bash
./cli/target/release/agent-tui spawn bash

# Send rapid sequence
for i in {1..10}; do
  ./cli/target/release/agent-tui type "echo $i"
  ./cli/target/release/agent-tui keystroke Enter
done

./cli/target/release/agent-tui snapshot
./cli/target/release/agent-tui kill
```

**Expected:** All commands processed in order.

#### Test 5.7: Custom Terminal Size
```bash
./cli/target/release/agent-tui spawn bash --cols 80 --rows 24
./cli/target/release/agent-tui sessions
./cli/target/release/agent-tui snapshot
./cli/target/release/agent-tui kill
```

**Expected:** Session shows 80x24 dimensions.

#### Test 5.8: Working Directory
```bash
./cli/target/release/agent-tui spawn bash -d /tmp
./cli/target/release/agent-tui type "pwd"
./cli/target/release/agent-tui keystroke Enter
./cli/target/release/agent-tui snapshot
./cli/target/release/agent-tui kill
```

**Expected:** pwd shows /tmp.

### Phase 6: Long-Running Commands

#### Test 6.1: Process That Exits
```bash
./cli/target/release/agent-tui spawn bash
./cli/target/release/agent-tui type "exit"
./cli/target/release/agent-tui keystroke Enter

# Wait a moment
sleep 1

./cli/target/release/agent-tui sessions
./cli/target/release/agent-tui snapshot
```

**Expected:** Session shows as exited or is cleaned up.

---

## Bug Report Format

When you find a bug, report it using this format:

```markdown
## Bug: [Short Description]

**Severity**: Critical / High / Medium / Low
**Command**: `agent-tui [command with args]`

**Steps to Reproduce**:
1. Step one
2. Step two
3. ...

**Expected**: What should happen

**Actual**: What actually happened

**Error Output** (if any):
```
[paste error message here]
```

**Environment**:
- OS: [macOS/Linux]
- Node version: [output of `node --version`]
```

---

## Testing Checklist

- [ ] Daemon starts and stays running
- [ ] Spawn creates sessions correctly
- [ ] Type/keystroke input works
- [ ] Snapshot captures screen state
- [ ] Snapshot -i detects interactive elements
- [ ] Click activates elements
- [ ] Fill populates inputs
- [ ] Wait finds text correctly
- [ ] Wait times out appropriately
- [ ] Kill terminates sessions
- [ ] Sessions lists all sessions
- [ ] Multiple concurrent sessions work
- [ ] Session switching with -s works
- [ ] JSON output is valid
- [ ] Error messages are clear
- [ ] Special characters handled
- [ ] Custom terminal size works
- [ ] Working directory option works

---

## Start Testing

Begin with **Phase 1** tests and work through systematically. After each test:
1. Note if it passed or failed
2. If failed, create a bug report
3. Move to the next test

Report a summary at the end with:
- Tests passed
- Tests failed
- Bugs found (with links to bug reports)
