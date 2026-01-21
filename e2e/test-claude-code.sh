#!/bin/bash
# Claude Code E2E Test
# Demonstrates multi-turn conversation automation with Claude Code
#
# Prerequisites:
#   - agent-tui CLI built and in PATH
#   - claude CLI installed
#
# Usage: ./e2e/test-claude-code.sh

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() {
    echo -e "${BLUE}[$(date +%H:%M:%S)]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Cleanup on exit
cleanup() {
    log "Cleaning up..."
    agent-tui cleanup --all 2>/dev/null || true
    rm -f hello.py 2>/dev/null || true
}

trap cleanup EXIT

# Check prerequisites
if ! command -v agent-tui &> /dev/null; then
    error "agent-tui not found. Please build and install first."
    exit 1
fi

if ! command -v claude &> /dev/null; then
    error "claude CLI not found. Please install Claude Code."
    exit 1
fi

echo "============================================"
echo "Claude Code E2E Automation Test"
echo "============================================"
echo ""

# Step 1: Spawn Claude Code with --dangerously-skip-permissions
log "Spawning Claude Code session..."
agent-tui spawn "claude --dangerously-skip-permissions" --cols 160 --rows 50

# Step 2: Wait for Claude prompt
log "Waiting for Claude prompt..."
if ! agent-tui wait "/[>❯]/" --timeout 120000; then
    error "Claude did not show prompt within 2 minutes"
    agent-tui snapshot
    exit 1
fi
success "Claude prompt detected"

# Step 3: Take a snapshot to verify state
log "Taking initial snapshot..."
agent-tui snapshot -i

# Step 4: Type a request to Claude
log "Sending request to Claude..."
agent-tui type "Create hello.py that prints Hello World"
agent-tui press Enter

# Step 5: Wait for Claude to complete (stable screen)
log "Waiting for Claude to finish..."
if ! agent-tui wait --stable --timeout 60000; then
    error "Claude did not stabilize within 1 minute"
    agent-tui snapshot
    exit 1
fi
success "Claude completed response"

# Step 6: Take snapshot with elements
log "Capturing final state..."
agent-tui snapshot -i

# Step 7: Verify the file was created
log "Verifying hello.py was created..."
if [ -f "hello.py" ]; then
    success "hello.py was created"
    echo ""
    echo "Contents of hello.py:"
    echo "--------------------"
    cat hello.py
    echo "--------------------"

    # Run the file to verify it works
    log "Running hello.py..."
    if python3 hello.py | grep -q "Hello"; then
        success "hello.py works correctly!"
    else
        error "hello.py did not output expected text"
    fi
else
    error "hello.py was not created"
    log "Current directory contents:"
    ls -la
fi

# Step 8: Send exit command
log "Exiting Claude..."
agent-tui type "/exit"
agent-tui press Enter
sleep 2

# Step 9: Kill session
log "Killing session..."
agent-tui kill

echo ""
echo "============================================"
echo -e "${GREEN}Claude Code E2E Test Complete!${NC}"
echo "============================================"
