#!/bin/bash
# Basic agent-tui workflow example
# Demonstrates: spawn, snapshot, type, keystroke, wait, kill

set -e

echo "=== Basic agent-tui Workflow ==="
echo

# Start a bash session
echo "1. Starting bash session..."
agent-tui spawn bash
sleep 1

# Type a command
echo "2. Typing a command..."
agent-tui type "echo 'Hello from agent-tui!'"
agent-tui keystroke Enter

# Wait for output
echo "3. Waiting for output..."
agent-tui wait "Hello from agent-tui"

# Take a snapshot to verify
echo "4. Taking snapshot..."
agent-tui snapshot

# Clean up
echo "5. Cleaning up..."
agent-tui kill

echo
echo "=== Workflow Complete ==="
