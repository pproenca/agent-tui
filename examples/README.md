# agent-tui Examples

This directory contains example scripts demonstrating agent-tui workflows.

## Prerequisites

- agent-tui CLI must be built and in your PATH
- agent-tui daemon must be running or will auto-start
- `jq` is required for `json-workflow.sh`

## Examples

### basic-workflow.sh

Basic demonstration of the spawn -> interact -> kill workflow.

```bash
./basic-workflow.sh
```

Demonstrates:
- `spawn` - Starting a bash session
- `type` - Typing text
- `keystroke` - Sending Enter
- `wait` - Waiting for text
- `snapshot` - Capturing screen
- `kill` - Cleaning up

### test-interactive-prompt.sh

Shows how to interact with scripts that prompt for user input.

```bash
./test-interactive-prompt.sh
```

Demonstrates:
- Handling multiple prompts
- Sequential text entry
- Waiting for specific prompts

### menu-navigation.sh

Demonstrates navigating menu-based interfaces.

```bash
./menu-navigation.sh
```

Demonstrates:
- Menu detection
- Arrow key navigation
- Option selection

### json-workflow.sh

Shows how to use JSON output for programmatic interaction.

```bash
./json-workflow.sh
```

Demonstrates:
- `-f json` flag usage
- Parsing responses with `jq`
- Extracting session info, elements, health data

## Running Examples

1. Build agent-tui:
   ```bash
   cd cli && cargo build --release
   ```

2. Add to PATH (or use full path):
   ```bash
   export PATH=$PATH:$(pwd)/cli/target/release
   ```

3. Run an example:
   ```bash
   cd examples
   ./basic-workflow.sh
   ```

## Writing Your Own Workflows

Key patterns:

```bash
# Always start with spawn
agent-tui spawn "your-command"

# Wait for UI to be ready
agent-tui wait "expected text"

# Snapshot before interacting
agent-tui snapshot -i

# Interact with detected elements
agent-tui fill @e1 "value"
agent-tui keystroke Enter

# Wait for result
agent-tui wait --stable

# Always clean up
agent-tui kill
```
