# Split terminal demo: agent-tui controlling Claude
# Uses tmux for side-by-side pane effect
# Generate with: vhs assets/split-demo.tape
# Build first: cd cli && cargo build --release

Output assets/split-demo.gif

Require tmux
Require agent-tui

Set Shell "bash"
Set FontSize 14
Set Width 1400
Set Height 700
Set Theme "Catppuccin Mocha"
Set Padding 20
Set TypingSpeed 50ms
Set PlaybackSpeed 1.0

Env PATH "${PWD}/cli/target/release:${PATH}"

# Start fresh tmux session and attach
Hide
Type "tmux kill-session -t demo 2>/dev/null; tmux new-session -d -s demo && tmux attach -t demo"
Enter
Sleep 500ms
Show

# Split into left/right panes (Ctrl+b %)
Ctrl+b
Type "%"
Sleep 300ms

# === LEFT PANE: Spawn and attach to Claude ===
Ctrl+b
Left
Sleep 200ms

Type "# Spawn Claude and attach interactively"
Enter
Sleep 500ms

Type "agent-tui spawn claude"
Enter
Sleep 2s

# Now attach interactively to see Claude's UI
Type "agent-tui attach -i"
Enter
Sleep 5s

# === RIGHT PANE: Control Claude ===
Ctrl+b
Right
Sleep 300ms

Type "# Send Enter keystroke to Claude"
Enter
Sleep 500ms

Type "agent-tui press Enter"
Enter
Sleep 3s

# Show the result in the left pane
Sleep 2s

# Cleanup
Hide
Ctrl+b
Type ":"
Type "kill-session"
Enter
Show
