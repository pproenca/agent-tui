# Live Demo Script

Goal: show a real terminal app launch, capture screenshots, send input, and verify outcomes in real-time.

## Suggested Demo (htop)

### Step 1: Launch
```bash
agent-tui run htop
```

### Step 2: Observe
```bash
agent-tui screenshot
```
Say: "Here is the current terminal screen. I'll keep re-snapshotting after each action."

### Step 3: Interaction
```bash
agent-tui press F10
```
Say: "I'm sending a keypress to exit."

### Step 4: Verify
```bash
agent-tui wait "Quit" --gone
```
Say: "I wait for the expected text to disappear to confirm the app closed."

### Step 5: Cleanup
```bash
agent-tui kill
```

## Narration Tips
- Emphasize the observe/act/wait loop.
- Call out that every action is followed by a fresh snapshot.
- Highlight using `wait --stable` when the UI is changing.
