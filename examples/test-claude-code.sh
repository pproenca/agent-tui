#!/bin/bash
# test-claude-code.sh - Demo script for testing Claude Code with agent-tui
#
# This script demonstrates how to use agent-tui to automate Claude Code
# for testing and verification purposes.
#
# Element refs use the new simplified format: @e1, @e2, @e3
# (agent-browser style, sequential per snapshot)
#
# Prerequisites:
# - agent-tui installed and in PATH
# - claude CLI installed
#
# Usage:
#   ./test-claude-code.sh
#
# This will:
# 1. Spawn Claude Code in permissive mode
# 2. Send a simple task
# 3. Wait for completion
# 4. Verify the result
# 5. Clean up

set -e

# Configuration
TIMEOUT_STARTUP=120000   # 2 minutes for Claude to start
TIMEOUT_TASK=60000       # 1 minute for task completion
TERMINAL_COLS=120
TERMINAL_ROWS=40

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

cleanup() {
    log_info "Cleaning up..."
    agent-tui kill 2>/dev/null || true
    rm -f hello.py 2>/dev/null || true
}

# Set up cleanup trap
trap cleanup EXIT

# Main test
main() {
    log_info "Starting Claude Code test with agent-tui..."

    # Check if agent-tui is available
    if ! command -v agent-tui &> /dev/null; then
        log_error "agent-tui not found in PATH"
        exit 1
    fi

    # Check if claude is available
    if ! command -v claude &> /dev/null; then
        log_error "claude CLI not found in PATH"
        exit 1
    fi

    # Check daemon health
    log_info "Checking agent-tui daemon health..."
    agent-tui health -v || {
        log_error "Daemon health check failed"
        exit 1
    }

    # Remove any existing test file
    rm -f hello.py

    # Spawn Claude Code in permissive mode
    log_info "Spawning Claude Code..."
    agent-tui spawn "claude --dangerously-skip-permissions" --cols $TERMINAL_COLS --rows $TERMINAL_ROWS

    # Wait for Claude to initialize (look for the prompt)
    log_info "Waiting for Claude Code to initialize..."
    if ! agent-tui wait "/[>❯]/" --timeout $TIMEOUT_STARTUP; then
        log_error "Claude Code failed to start within timeout"
        agent-tui snapshot
        exit 1
    fi

    log_info "Claude Code is ready!"

    # Send a test task
    log_info "Sending test task: Create hello.py..."
    agent-tui type "Create a file called hello.py that prints 'Hello World'"
    agent-tui keystroke Enter

    # Wait for completion (screen stabilizes)
    log_info "Waiting for task completion..."
    if ! agent-tui wait --stable --timeout $TIMEOUT_TASK; then
        log_warn "Stability wait timed out, checking result anyway..."
    fi

    # Take a snapshot to see the result
    log_info "Taking snapshot of final state..."
    agent-tui snapshot -i

    # Verify the file was created
    log_info "Verifying result..."
    if [ -f "hello.py" ]; then
        log_info "SUCCESS: hello.py was created!"
        echo ""
        echo "--- File contents ---"
        cat hello.py
        echo "---------------------"
        echo ""

        # Try to run it
        log_info "Running hello.py..."
        python3 hello.py 2>/dev/null || python hello.py 2>/dev/null || {
            log_warn "Could not run hello.py (Python not found or syntax error)"
        }
    else
        log_error "FAILED: hello.py was not created"
        echo ""
        echo "--- Current screen state ---"
        agent-tui snapshot
        echo "----------------------------"
        exit 1
    fi

    log_info "Test completed successfully!"
}

# Run main
main "$@"
