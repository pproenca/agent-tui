#!/bin/bash
# create-next-app.sh - Demo script for automating Next.js project creation
#
# This script demonstrates how to use agent-tui to automate the
# create-next-app interactive wizard.
#
# Prerequisites:
# - agent-tui installed and in PATH
# - Node.js and npm installed
#
# Usage:
#   ./create-next-app.sh [project-name]
#
# This will:
# 1. Spawn the create-next-app wizard
# 2. Fill in project name
# 3. Navigate through options
# 4. Wait for completion

set -e

# Configuration
PROJECT_NAME="${1:-my-next-app}"
TIMEOUT_STARTUP=30000
TIMEOUT_INSTALL=300000  # 5 minutes for npm install

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

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
    log_info "Cleaning up session..."
    agent-tui kill 2>/dev/null || true
}

trap cleanup EXIT

main() {
    log_info "Creating Next.js app: $PROJECT_NAME"

    # Check if agent-tui is available
    if ! command -v agent-tui &> /dev/null; then
        log_error "agent-tui not found in PATH"
        exit 1
    fi

    # Check if npx is available
    if ! command -v npx &> /dev/null; then
        log_error "npx not found in PATH"
        exit 1
    fi

    # Check daemon health
    log_info "Checking agent-tui daemon..."
    agent-tui health || exit 1

    # Spawn create-next-app
    log_info "Spawning create-next-app wizard..."
    agent-tui spawn "npx create-next-app@latest" --cols 120 --rows 40

    # Wait for project name prompt
    log_info "Waiting for project name prompt..."
    if ! agent-tui wait "project" --timeout $TIMEOUT_STARTUP; then
        log_error "Did not see project name prompt"
        agent-tui snapshot
        exit 1
    fi

    # Take snapshot to see current state
    agent-tui snapshot -i

    # Enter project name
    log_info "Entering project name: $PROJECT_NAME"
    agent-tui type "$PROJECT_NAME"
    agent-tui press Enter

    # Wait for TypeScript question
    log_info "Waiting for TypeScript prompt..."
    if agent-tui wait "TypeScript" --timeout 10000 2>/dev/null; then
        log_info "Selecting TypeScript: Yes"
        agent-tui press Enter  # Default is usually Yes
    fi

    # Wait for ESLint question
    log_info "Waiting for ESLint prompt..."
    if agent-tui wait "ESLint" --timeout 10000 2>/dev/null; then
        log_info "Selecting ESLint: Yes"
        agent-tui press Enter
    fi

    # Wait for Tailwind CSS question
    log_info "Waiting for Tailwind prompt..."
    if agent-tui wait "Tailwind" --timeout 10000 2>/dev/null; then
        log_info "Selecting Tailwind: Yes"
        agent-tui press Enter
    fi

    # Wait for src/ directory question
    log_info "Waiting for src/ directory prompt..."
    if agent-tui wait "src/" --timeout 10000 2>/dev/null; then
        log_info "Selecting src/ directory: Yes"
        agent-tui press Enter
    fi

    # Wait for App Router question
    log_info "Waiting for App Router prompt..."
    if agent-tui wait "App Router" --timeout 10000 2>/dev/null; then
        log_info "Selecting App Router: Yes"
        agent-tui press Enter
    fi

    # Wait for import alias question
    log_info "Waiting for import alias prompt..."
    if agent-tui wait "import alias" --timeout 10000 2>/dev/null; then
        log_info "Accepting default import alias"
        agent-tui press Enter
    fi

    # Wait for installation to complete
    log_info "Waiting for installation to complete (this may take a few minutes)..."
    if ! agent-tui wait --stable --timeout $TIMEOUT_INSTALL; then
        log_warn "Stability wait timed out, checking result..."
    fi

    # Take final snapshot
    log_info "Final state:"
    agent-tui snapshot

    # Check if project directory was created
    if [ -d "$PROJECT_NAME" ]; then
        log_info "SUCCESS: Project '$PROJECT_NAME' created!"
        echo ""
        echo "To get started:"
        echo "  cd $PROJECT_NAME"
        echo "  npm run dev"
    else
        log_error "Project directory not found"
        exit 1
    fi
}

main "$@"
