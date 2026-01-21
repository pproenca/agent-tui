#!/bin/bash
# Interactive prompt workflow example
# Demonstrates interaction with prompts that ask for user input

set -e

echo "=== Interactive Prompt Workflow ==="
echo

# Create a simple interactive script
TEMP_SCRIPT=$(mktemp)
cat > "$TEMP_SCRIPT" << 'EOF'
#!/bin/bash
echo "Welcome to the Setup Wizard!"
echo
read -p "Enter your name: " name
echo "Hello, $name!"
read -p "Enter your favorite color: " color
echo "Great choice! $color is a nice color."
echo
echo "Setup complete!"
EOF
chmod +x "$TEMP_SCRIPT"

# Run the interactive script
echo "1. Starting interactive wizard..."
agent-tui spawn "$TEMP_SCRIPT"

# Wait for the first prompt
echo "2. Waiting for name prompt..."
agent-tui wait "Enter your name"

# Get the current state
echo "3. Taking snapshot with elements..."
agent-tui snapshot -i

# Type name and confirm
echo "4. Entering name..."
agent-tui type "Alice"
agent-tui press Enter

# Wait for color prompt
echo "5. Waiting for color prompt..."
agent-tui wait "favorite color"

# Type color and confirm
echo "6. Entering color..."
agent-tui type "blue"
agent-tui press Enter

# Wait for completion
echo "7. Waiting for completion..."
agent-tui wait "Setup complete"

# Final snapshot
echo "8. Final snapshot..."
agent-tui snapshot

# Clean up
echo "9. Cleaning up..."
agent-tui kill
rm -f "$TEMP_SCRIPT"

echo
echo "=== Workflow Complete ==="
