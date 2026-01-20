# VHS Demo Script for agent-tui
# Generate with: vhs assets/demo.tape
# Install VHS: brew install charmbracelet/tap/vhs
# Build first: cd cli && cargo build --release

# Dependency validation
Require agent-tui

# Output declaration
Output assets/demo.gif

# Terminal settings
Set Shell "bash"
Set FontSize 14
Set Width 1000
Set Height 600
Set Theme "Catppuccin Mocha"
Set Padding 20
Set TypingSpeed 40ms
Set PlaybackSpeed 1.0

# Environment setup
Env PATH "${PWD}/cli/target/release:${PATH}"

# Title
Type "# agent-tui: AI agents controlling Claude Code"
Enter
Sleep 1s

# Step 1: Spawn Claude Code
Type "agent-tui spawn 'claude'"
Enter
Sleep 3s

Type "# Claude Code is now running in a virtual terminal"
Enter
Sleep 1s

# Step 2: Take a snapshot
Type "agent-tui snapshot --interactive"
Enter
Sleep 1.5s

Type "# We can see the Claude prompt and detect elements"
Enter
Sleep 1s

# Step 3: Send a message to Claude
Type "agent-tui type 'Hello Claude! What is 2+2?'"
Enter
Sleep 1000ms

Type "agent-tui keystroke Enter"
Enter
Sleep 3s

Type "# Claude is thinking..."
Enter
Sleep 2s

# Step 4: Take another snapshot to see the response
Type "agent-tui snapshot"
Enter
Sleep 1.5s

Type "# We captured Claude's response!"
Enter
Sleep 1.5s

# Step 5: Continue the conversation
Type "agent-tui type 'Now multiply that by 10'"
Enter
Sleep 1000ms

Type "agent-tui keystroke Enter"
Enter
Sleep 2s

# Final
Type "# AI agents can now orchestrate other AI agents"
Enter
Sleep 2s

# Clean exit
Type "agent-tui kill"
Enter
Sleep 1000ms

Type "# The future is here. That's agent-tui!"
Enter
Sleep 3000ms
