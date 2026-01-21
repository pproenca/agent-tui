#!/bin/bash
# interactive-installer.sh - Generic pattern for automating interactive installers
#
# This script demonstrates a reusable pattern for automating any
# interactive TUI installer using agent-tui.
#
# Prerequisites:
# - agent-tui installed and in PATH
#
# Usage:
#   ./interactive-installer.sh <command> [options...]
#
# Examples:
#   ./interactive-installer.sh "npm init"
#   ./interactive-installer.sh "cargo new --name myproject"
#   ./interactive-installer.sh "python -m venv .venv"

set -e

# Configuration
COMMAND="$1"
TIMEOUT_DEFAULT=30000
TERMINAL_COLS=120
TERMINAL_ROWS=40

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_step() { echo -e "${BLUE}[STEP]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

show_usage() {
    echo "Usage: $0 <command> [options...]"
    echo ""
    echo "Examples:"
    echo "  $0 'npm init'"
    echo "  $0 'npx create-react-app my-app'"
    echo "  $0 'cargo init --name myproject'"
    echo ""
    echo "Interactive commands:"
    echo "  type <text>       - Type text into the terminal"
    echo "  key <key>         - Press a key (Enter, Tab, ArrowDown, etc.)"
    echo "  wait <text>       - Wait for text to appear"
    echo "  stable            - Wait for screen to stabilize"
    echo "  snapshot          - Take and display a snapshot"
    echo "  quit              - Exit the script"
}

cleanup() {
    log_info "Cleaning up..."
    agent-tui kill 2>/dev/null || true
}

trap cleanup EXIT

interactive_loop() {
    log_info "Entering interactive mode. Commands: type, key, wait, stable, snapshot, quit"
    echo ""

    while true; do
        # Show current screen
        echo "--- Current Screen ---"
        agent-tui screen 2>/dev/null || echo "(no screen available)"
        echo "----------------------"
        echo ""

        # Read command
        read -p "> " -r cmd args

        case "$cmd" in
            type)
                if [ -n "$args" ]; then
                    log_step "Typing: $args"
                    agent-tui type "$args"
                else
                    log_warn "Usage: type <text>"
                fi
                ;;
            key)
                if [ -n "$args" ]; then
                    log_step "Sending key: $args"
                    agent-tui press "$args"
                else
                    log_warn "Usage: key <key> (e.g., Enter, Tab, ArrowDown)"
                fi
                ;;
            wait)
                if [ -n "$args" ]; then
                    log_step "Waiting for: $args"
                    if agent-tui wait "$args" --timeout $TIMEOUT_DEFAULT; then
                        log_info "Found: $args"
                    else
                        log_warn "Timeout waiting for: $args"
                    fi
                else
                    log_warn "Usage: wait <text>"
                fi
                ;;
            stable)
                log_step "Waiting for screen to stabilize..."
                if agent-tui wait --stable --timeout $TIMEOUT_DEFAULT; then
                    log_info "Screen stabilized"
                else
                    log_warn "Timeout waiting for stability"
                fi
                ;;
            snapshot|snap)
                log_step "Taking snapshot..."
                agent-tui snapshot -i
                ;;
            elements)
                log_step "Showing elements..."
                agent-tui snapshot -i -f json 2>/dev/null | jq '.elements' 2>/dev/null || agent-tui snapshot -i
                ;;
            quit|exit|q)
                log_info "Exiting..."
                break
                ;;
            help|h|\?)
                echo "Commands:"
                echo "  type <text>  - Type text"
                echo "  key <key>    - Press key"
                echo "  wait <text>  - Wait for text"
                echo "  stable       - Wait for stability"
                echo "  snapshot     - Show snapshot"
                echo "  elements     - Show detected elements"
                echo "  quit         - Exit"
                ;;
            "")
                # Empty command, just refresh screen
                ;;
            *)
                log_warn "Unknown command: $cmd (type 'help' for commands)"
                ;;
        esac

        echo ""
    done
}

main() {
    if [ -z "$COMMAND" ]; then
        show_usage
        exit 1
    fi

    log_info "Interactive Installer Helper"
    log_info "Command: $COMMAND"
    echo ""

    # Check agent-tui
    if ! command -v agent-tui &> /dev/null; then
        log_error "agent-tui not found"
        exit 1
    fi

    # Check daemon
    log_info "Checking daemon..."
    agent-tui health || exit 1

    # Spawn the command
    log_step "Spawning: $COMMAND"
    agent-tui spawn "$COMMAND" --cols $TERMINAL_COLS --rows $TERMINAL_ROWS

    # Wait a moment for startup
    sleep 1

    # Enter interactive loop
    interactive_loop
}

main "$@"
