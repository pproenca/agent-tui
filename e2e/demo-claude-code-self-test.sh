#!/bin/bash
#
# Demo: Claude Code tests itself using agent-tui
#
# This script demonstrates agent-tui's ability to interact with Claude Code,
# effectively allowing Claude Code to test itself. This is a powerful capability
# for automated testing, CI/CD, and self-verification.
#
# Prerequisites:
# - agent-tui daemon running (agent-tui-daemon)
# - Claude Code CLI installed (claude)
#
# Usage:
#   ./demo-claude-code-self-test.sh
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     agent-tui Demo: Claude Code Self-Test                      ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════════╝${NC}"
echo

# Check if agent-tui is available
if ! command -v agent-tui &> /dev/null; then
    echo -e "${RED}Error: agent-tui CLI not found. Please build and install it first.${NC}"
    echo "  cd cli && cargo build --release"
    exit 1
fi

# Check daemon health
echo -e "${YELLOW}[1/6]${NC} Checking daemon health..."
if ! agent-tui health > /dev/null 2>&1; then
    echo -e "${RED}Error: agent-tui daemon not running. Start it with:${NC}"
    echo "  cd daemon && npm start"
    exit 1
fi
echo -e "${GREEN}✓ Daemon is healthy${NC}"
echo

# Create a test directory
TEST_DIR=$(mktemp -d)
echo -e "${YELLOW}[2/6]${NC} Created test directory: ${TEST_DIR}"
echo

# Spawn Claude Code
echo -e "${YELLOW}[3/6]${NC} Spawning Claude Code..."
SESSION_ID=$(agent-tui spawn "claude --dangerously-skip-permissions" --cwd "$TEST_DIR" -f json | jq -r '.session_id')
echo -e "${GREEN}✓ Session started: ${SESSION_ID}${NC}"
echo

# Wait for Claude Code to initialize
echo -e "${YELLOW}[4/6]${NC} Waiting for Claude Code to initialize..."
if agent-tui wait "/[>❯]/" --timeout 120000 > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Claude Code is ready${NC}"
else
    echo -e "${RED}✗ Timeout waiting for Claude Code${NC}"
    agent-tui kill
    rm -rf "$TEST_DIR"
    exit 1
fi
echo

# Show current screen
echo -e "${YELLOW}[5/6]${NC} Current screen state:"
echo "────────────────────────────────────────"
agent-tui screen | head -20
echo "────────────────────────────────────────"
echo

# Test: Ask Claude to write a simple file
echo -e "${YELLOW}[6/6]${NC} Asking Claude to write hello.py..."
agent-tui type "Write a Python file called hello.py that prints 'Hello from agent-tui!' - just write the file, no explanation needed"
agent-tui press Enter

# Wait for Claude to complete
echo "Waiting for Claude to respond..."
if agent-tui wait --stable --timeout 60000 > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Claude completed the task${NC}"
else
    echo -e "${YELLOW}! Task may still be in progress${NC}"
fi
echo

# Show final screen
echo "Final screen state:"
echo "────────────────────────────────────────"
agent-tui screen | head -30
echo "────────────────────────────────────────"
echo

# Check if the file was created
if [ -f "$TEST_DIR/hello.py" ]; then
    echo -e "${GREEN}✓ File created successfully!${NC}"
    echo "Contents of hello.py:"
    echo "────────────────────────────────────────"
    cat "$TEST_DIR/hello.py"
    echo "────────────────────────────────────────"
    echo

    # Run the file
    echo "Running hello.py:"
    python3 "$TEST_DIR/hello.py" || true
    echo
else
    echo -e "${YELLOW}! File not found (Claude may need more time)${NC}"
fi

# Cleanup
echo "Cleaning up..."
agent-tui type "/exit"
agent-tui press Enter
sleep 2
agent-tui kill 2>/dev/null || true
rm -rf "$TEST_DIR"

echo
echo -e "${GREEN}╔════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║     Demo Complete!                                             ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════════╝${NC}"
echo
echo "This demo showed how agent-tui can:"
echo "  1. Spawn Claude Code in a virtual terminal"
echo "  2. Wait for the interface to be ready"
echo "  3. Send commands and key presses"
echo "  4. Wait for task completion"
echo "  5. Verify the results"
echo
echo "Use these patterns for automated testing and CI/CD integration!"
