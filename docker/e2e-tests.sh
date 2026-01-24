#!/bin/bash
# E2E tests for agent-tui running against htop
# This script is the entrypoint for the Docker test container

set -euo pipefail

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly NC='\033[0m'

# Configuration constants
readonly SOCKET_WAIT_TIMEOUT_ITERATIONS=20  # 20 * 0.5s = 10s
readonly SOCKET_POLL_INTERVAL=0.5
readonly STABLE_WAIT_TIMEOUT_MS=10000
readonly SNAPSHOT_PREVIEW_LINES=3
readonly UI_UPDATE_DELAY=0.5

# Globals (set during test execution)
TESTS_PASSED=0
TESTS_FAILED=0
SESSION_ID=""   # Set by test_spawn_htop, used by subsequent tests
DAEMON_PID=""   # Set by test_daemon_startup, used by cleanup

#######################################
# Logs an info message with yellow [INFO] prefix.
# Arguments:
#   $@: Message to log
#######################################
log_info() {
    printf '%b[INFO]%b %s\n' "${YELLOW}" "${NC}" "$*"
}

#######################################
# Logs a pass message and increments TESTS_PASSED.
# Globals:
#   TESTS_PASSED: Incremented by 1
# Arguments:
#   $@: Message to log
#######################################
log_pass() {
    printf '%b[PASS]%b %s\n' "${GREEN}" "${NC}" "$*"
    ((++TESTS_PASSED)) || true
}

#######################################
# Logs a fail message and increments TESTS_FAILED.
# Globals:
#   TESTS_FAILED: Incremented by 1
# Arguments:
#   $@: Message to log
#######################################
log_fail() {
    printf '%b[FAIL]%b %s\n' "${RED}" "${NC}" "$*"
    ((++TESTS_FAILED)) || true
}

#######################################
# Logs a section header with decorative borders.
# Arguments:
#   $@: Section title
#######################################
log_section() {
    printf '\n'
    printf '========================================\n'
    printf '%s\n' "$*"
    printf '========================================\n'
}

#######################################
# Cleans up test resources on exit.
# Kills any active session and daemon process.
# Globals:
#   SESSION_ID: Read to kill session if set
#   DAEMON_PID: Read to kill daemon if set
#######################################
cleanup() {
    log_info "Cleaning up..."
    if [[ -n "${SESSION_ID:-}" ]]; then
        agent-tui kill --session "$SESSION_ID" 2>/dev/null || true
    fi
    if [[ -n "${DAEMON_PID:-}" ]]; then
        kill "$DAEMON_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

#######################################
# Test 1: Starts the daemon and verifies socket creation.
# Globals:
#   DAEMON_PID: Set to daemon's PID
#   AGENT_TUI_SOCKET: Read to verify socket path
#   SOCKET_WAIT_TIMEOUT_ITERATIONS: Max polling iterations
#   SOCKET_POLL_INTERVAL: Sleep time between polls
# Returns:
#   0 if daemon starts successfully, 1 otherwise
#######################################
test_daemon_startup() {
    log_section "Test 1: Daemon Startup"

    log_info "Starting daemon in background..."
    agent-tui daemon start --foreground &
    DAEMON_PID=$!

    # Wait for socket to appear
    local timeout=${SOCKET_WAIT_TIMEOUT_ITERATIONS}
    local elapsed=0
    while [[ ! -S "${AGENT_TUI_SOCKET}" ]] && (( elapsed < timeout )); do
        sleep "${SOCKET_POLL_INTERVAL}"
        ((++elapsed)) || true
    done

    if [[ ! -S "${AGENT_TUI_SOCKET}" ]]; then
        log_fail "Socket not created at ${AGENT_TUI_SOCKET} after ${timeout} iterations"
        return 1
    fi
    log_pass "Socket created at ${AGENT_TUI_SOCKET}"

    # Verify daemon is responsive
    log_info "Checking daemon status..."
    if agent-tui status; then
        log_pass "Daemon status check passed"
    else
        log_fail "Daemon status check failed"
        return 1
    fi
}

#######################################
# Test 2: Spawns htop session and verifies it becomes stable.
# Globals:
#   SESSION_ID: Set to the spawned session ID
#   STABLE_WAIT_TIMEOUT_MS: Timeout for stable wait
# Returns:
#   0 if htop spawns successfully, 1 otherwise
#######################################
test_spawn_htop() {
    log_section "Test 2: Spawn htop"

    log_info "Spawning htop session..."
    local output
    output=$(agent-tui run htop 2>&1)

    # Extract session ID from output (format: "Session started: <8-char-hex>")
    SESSION_ID=$(grep -oE 'Session started: [0-9a-f]+' <<< "$output" | grep -oE '[0-9a-f]+$' | head -1)

    if [[ -z "${SESSION_ID}" ]]; then
        log_fail "Failed to extract session ID from output: $output"
        return 1
    fi
    log_pass "Session created: ${SESSION_ID}"

    # Wait for stable render
    log_info "Waiting for stable render..."
    if agent-tui wait --stable --session "$SESSION_ID" --timeout "${STABLE_WAIT_TIMEOUT_MS}"; then
        log_pass "htop rendered and stable"
    else
        log_fail "htop did not stabilize"
        return 1
    fi

    # Verify session is active
    log_info "Verifying session is active..."
    local sessions
    sessions=$(agent-tui ls 2>&1)
    if grep -q "$SESSION_ID" <<< "$sessions"; then
        log_pass "Session is active in session list"
    else
        log_fail "Session not found in session list: $sessions"
        return 1
    fi
}

#######################################
# Test 3: Takes snapshot and verifies VOM content.
# Globals:
#   SESSION_ID: Read to identify session
#   SNAPSHOT_PREVIEW_LINES: Number of preview lines to show
# Returns:
#   0 if snapshot contains expected content, 1 otherwise
#######################################
test_snapshot_vom() {
    log_section "Test 3: Snapshot and VOM Verification"

    log_info "Taking accessibility snapshot..."
    local snapshot
    snapshot=$(agent-tui snap --session "$SESSION_ID" 2>&1)

    # Verify snapshot is non-empty
    if [[ -z "$snapshot" ]]; then
        log_fail "Snapshot is empty"
        return 1
    fi
    log_pass "Snapshot captured (${#snapshot} bytes)"

    # Check for htop-specific content
    if grep -qi "PID\|CPU\|MEM\|htop\|F1Help" <<< "$snapshot"; then
        log_pass "Snapshot contains htop screen content"
    else
        log_fail "Snapshot missing expected htop content"
        printf 'Snapshot was: %s\n' "${snapshot:0:500}"
        return 1
    fi

    # Check for Screen header (text format)
    if grep -q "Screen:" <<< "$snapshot"; then
        log_pass "Snapshot has Screen section"
    fi

    # Print first few lines for debugging
    log_info "First ${SNAPSHOT_PREVIEW_LINES} lines of snapshot:"
    head -"${SNAPSHOT_PREVIEW_LINES}" <<< "$snapshot"
}

#######################################
# Test 4: Tests keystroke interaction and session termination.
# Globals:
#   SESSION_ID: Read and cleared after kill
#   UI_UPDATE_DELAY: Sleep time after keystroke
# Returns:
#   0 if interaction succeeds, 1 otherwise
#######################################
test_keystroke_interaction() {
    log_section "Test 4: Keystroke Interaction"

    # Send F10 to htop (opens quit dialog)
    log_info "Sending F10 keystroke to htop..."
    if agent-tui key F10 --session "$SESSION_ID"; then
        log_pass "F10 keystroke sent successfully"
    else
        log_fail "Failed to send F10 keystroke"
        return 1
    fi

    # Wait briefly for UI to update
    sleep "${UI_UPDATE_DELAY}"

    # Take another snapshot to verify UI changed
    log_info "Taking post-keystroke snapshot..."
    local snapshot_after
    snapshot_after=$(agent-tui snap --session "$SESSION_ID" 2>&1)

    if [[ -n "$snapshot_after" ]]; then
        log_pass "Post-keystroke snapshot captured"
    else
        log_fail "Failed to capture post-keystroke snapshot"
        return 1
    fi

    # Clean up: kill the session
    log_info "Killing htop session..."
    local killed_session_id="$SESSION_ID"
    if agent-tui kill --session "$SESSION_ID"; then
        log_pass "Session terminated successfully"
        SESSION_ID=""  # Clear so cleanup doesn't try again
    else
        log_fail "Failed to terminate session"
        return 1
    fi

    # Verify session is gone
    log_info "Verifying session was removed..."
    local sessions_after
    sessions_after=$(agent-tui ls 2>&1)
    if grep -q "$killed_session_id" <<< "$sessions_after"; then
        log_fail "Session still exists after kill"
        return 1
    else
        log_pass "Session removed from session list"
    fi
}

#######################################
# Main entry point. Runs all tests and reports summary.
# Globals:
#   TESTS_PASSED: Read for summary
#   TESTS_FAILED: Read for summary and exit code
#   TERM, COLUMNS, LINES: Read for logging
#   AGENT_TUI_SOCKET: Read for logging
# Returns:
#   0 if all tests pass, 1 if any fail
#######################################
main() {
    log_section "agent-tui E2E Tests"
    log_info "TERM=$TERM COLUMNS=$COLUMNS LINES=$LINES"
    log_info "Socket path: ${AGENT_TUI_SOCKET}"

    # Run tests
    test_daemon_startup
    test_spawn_htop
    test_snapshot_vom
    test_keystroke_interaction

    # Summary
    log_section "Test Summary"
    printf 'Passed: %d\n' "$TESTS_PASSED"
    printf 'Failed: %d\n' "$TESTS_FAILED"

    if (( TESTS_FAILED > 0 )); then
        log_fail "Some tests failed!"
        exit 1
    else
        log_pass "All tests passed!"
        exit 0
    fi
}

main "$@"
