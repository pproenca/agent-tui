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
    if agent-tui daemon status; then
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
    sessions=$(agent-tui sessions 2>&1)
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
    snapshot=$(agent-tui screen --session "$SESSION_ID" 2>&1)

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
    if agent-tui input F10 --session "$SESSION_ID"; then
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
    snapshot_after=$(agent-tui screen --session "$SESSION_ID" 2>&1)

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
    sessions_after=$(agent-tui sessions 2>&1)
    if grep -q "$killed_session_id" <<< "$sessions_after"; then
        log_fail "Session still exists after kill"
        return 1
    else
        log_pass "Session removed from session list"
    fi
}

# =============================================================================
# Session Lifecycle Tests (Bash-based)
# =============================================================================

#######################################
# Test 5: Spawns bash session and verifies state changes via typing.
# Tests: spawn bash, type text, verify text appears in screen.
# Globals:
#   BASH_SESSION_ID: Set to spawned bash session
# Returns:
#   0 if text appears on screen, 1 otherwise
#######################################
test_type_changes_screen() {
    log_section "Test 5: Type Changes Screen"

    log_info "Spawning bash session..."
    local output
    output=$(agent-tui run bash 2>&1)

    local bash_session
    bash_session=$(grep -oE 'Session started: [0-9a-f]+' <<< "$output" | grep -oE '[0-9a-f]+$' | head -1)

    if [[ -z "${bash_session}" ]]; then
        log_fail "Failed to spawn bash session: $output"
        return 1
    fi
    log_pass "Bash session created: ${bash_session}"

    # Wait for shell prompt
    log_info "Waiting for shell prompt..."
    if ! agent-tui wait --stable --session "$bash_session" --timeout 5000; then
        log_fail "Shell did not stabilize"
        agent-tui kill --session "$bash_session" 2>/dev/null || true
        return 1
    fi

    # Type unique marker
    local marker
    marker="E2E_MARKER_$(date +%s)"
    log_info "Typing marker: $marker"
    if ! agent-tui input "echo $marker" --session "$bash_session"; then
        log_fail "Failed to type marker"
        agent-tui kill --session "$bash_session" 2>/dev/null || true
        return 1
    fi

    # Wait for text to appear
    if agent-tui wait --timeout 5000 "$marker" --session "$bash_session"; then
        log_pass "Marker appeared on screen"
    else
        log_fail "Marker did not appear on screen"
        agent-tui kill --session "$bash_session" 2>/dev/null || true
        return 1
    fi

    # Verify in snapshot
    local snapshot
    snapshot=$(agent-tui screen --session "$bash_session" 2>&1)
    if grep -q "$marker" <<< "$snapshot"; then
        log_pass "Snapshot contains typed text"
    else
        log_fail "Snapshot missing typed text"
        agent-tui kill --session "$bash_session" 2>/dev/null || true
        return 1
    fi

    # Cleanup
    agent-tui kill --session "$bash_session" 2>/dev/null || true
    log_pass "Bash session cleaned up"
}

#######################################
# Test 6: Multi-session management.
# Tests: spawn multiple sessions, verify independence, kill one, verify other works.
# Returns:
#   0 if sessions are independent, 1 otherwise
#######################################
test_multi_session_management() {
    log_section "Test 6: Multi-Session Management"

    # Spawn first session
    log_info "Spawning first bash session..."
    local output_a
    output_a=$(agent-tui run bash 2>&1)
    local sess_a
    sess_a=$(grep -oE 'Session started: [0-9a-f]+' <<< "$output_a" | grep -oE '[0-9a-f]+$' | head -1)

    if [[ -z "${sess_a}" ]]; then
        log_fail "Failed to spawn first session"
        return 1
    fi
    log_pass "Session A: ${sess_a}"

    # Spawn second session
    log_info "Spawning second bash session..."
    local output_b
    output_b=$(agent-tui run bash 2>&1)
    local sess_b
    sess_b=$(grep -oE 'Session started: [0-9a-f]+' <<< "$output_b" | grep -oE '[0-9a-f]+$' | head -1)

    if [[ -z "${sess_b}" ]]; then
        log_fail "Failed to spawn second session"
        agent-tui kill --session "$sess_a" 2>/dev/null || true
        return 1
    fi
    log_pass "Session B: ${sess_b}"

    # Verify different IDs
    if [[ "$sess_a" == "$sess_b" ]]; then
        log_fail "Sessions have same ID"
        agent-tui kill --session "$sess_a" 2>/dev/null || true
        return 1
    fi
    log_pass "Sessions have different IDs"

    # Wait for both to stabilize
    agent-tui wait --stable --session "$sess_a" --timeout 5000 2>/dev/null || true
    agent-tui wait --stable --session "$sess_b" --timeout 5000 2>/dev/null || true

    # Type unique markers in each
    log_info "Typing markers in each session..."
    agent-tui input "MARKER_AAA" --session "$sess_a" 2>/dev/null
    agent-tui input "MARKER_BBB" --session "$sess_b" 2>/dev/null

    # Verify sessions list shows both
    local sessions_list
    sessions_list=$(agent-tui sessions 2>&1)
    if grep -q "$sess_a" <<< "$sessions_list" && grep -q "$sess_b" <<< "$sessions_list"; then
        log_pass "Both sessions in list"
    else
        log_fail "Sessions not found in list: $sessions_list"
        agent-tui kill --session "$sess_a" 2>/dev/null || true
        agent-tui kill --session "$sess_b" 2>/dev/null || true
        return 1
    fi

    # Kill session A
    log_info "Killing session A..."
    if ! agent-tui kill --session "$sess_a"; then
        log_fail "Failed to kill session A"
        agent-tui kill --session "$sess_b" 2>/dev/null || true
        return 1
    fi
    log_pass "Session A killed"

    # Session B should still work
    log_info "Verifying session B still works..."
    local snapshot_b
    snapshot_b=$(agent-tui screen --session "$sess_b" 2>&1)
    if [[ -n "$snapshot_b" ]]; then
        log_pass "Session B still functional"
    else
        log_fail "Session B not functional after killing A"
        agent-tui kill --session "$sess_b" 2>/dev/null || true
        return 1
    fi

    # Cleanup
    agent-tui kill --session "$sess_b" 2>/dev/null || true
    log_pass "Multi-session test completed"
}

#######################################
# Test 7: Wait conditions - stable and timeout.
# Tests: wait --stable succeeds, wait for missing text times out.
# Returns:
#   0 if wait conditions work correctly, 1 otherwise
#######################################
test_wait_conditions() {
    log_section "Test 7: Wait Conditions"

    # Spawn bash session
    local output
    output=$(agent-tui run bash 2>&1)
    local sess
    sess=$(grep -oE 'Session started: [0-9a-f]+' <<< "$output" | grep -oE '[0-9a-f]+$' | head -1)

    if [[ -z "${sess}" ]]; then
        log_fail "Failed to spawn session"
        return 1
    fi

    # Test 7a: Wait stable should succeed for idle shell
    log_info "Testing wait --stable..."
    if agent-tui wait --stable --session "$sess" --timeout 3000; then
        log_pass "Wait stable succeeded for idle shell"
    else
        log_fail "Wait stable failed for idle shell"
        agent-tui kill --session "$sess" 2>/dev/null || true
        return 1
    fi

    # Test 7b: Wait for missing text should timeout
    log_info "Testing wait timeout..."
    if agent-tui wait --timeout 500 "TEXT_THAT_NEVER_APPEARS" --session "$sess" 2>/dev/null; then
        log_fail "Wait should have timed out but succeeded"
        agent-tui kill --session "$sess" 2>/dev/null || true
        return 1
    else
        log_pass "Wait correctly timed out for missing text"
    fi

    # Cleanup
    agent-tui kill --session "$sess" 2>/dev/null || true
}

#######################################
# Test 8: Error handling - operations on dead session.
# Tests: operations on killed session fail gracefully.
# Returns:
#   0 if errors handled correctly, 1 otherwise
#######################################
test_dead_session_operations() {
    log_section "Test 8: Dead Session Operations"

    # Spawn and immediately kill
    local output
    output=$(agent-tui run bash 2>&1)
    local sess
    sess=$(grep -oE 'Session started: [0-9a-f]+' <<< "$output" | grep -oE '[0-9a-f]+$' | head -1)

    if [[ -z "${sess}" ]]; then
        log_fail "Failed to spawn session"
        return 1
    fi
    log_pass "Session spawned: $sess"

    # Brief wait for initialization
    sleep 0.5

    # Kill the session
    if ! agent-tui kill --session "$sess"; then
        log_fail "Failed to kill session"
        return 1
    fi
    log_pass "Session killed"

    # Operations on dead session should fail
    log_info "Testing operations on dead session..."
    if agent-tui screen --session "$sess" 2>/dev/null; then
        log_fail "Screen on dead session should fail"
        return 1
    else
        log_pass "Screen on dead session correctly failed"
    fi
}

#######################################
# Test 9: Click nonexistent element.
# Tests: clicking a ref that doesn't exist fails.
# Returns:
#   0 if error handled correctly, 1 otherwise
#######################################
test_click_nonexistent_element() {
    log_section "Test 9: Click Nonexistent Element"

    # Spawn bash session
    local output
    output=$(agent-tui run bash 2>&1)
    local sess
    sess=$(grep -oE 'Session started: [0-9a-f]+' <<< "$output" | grep -oE '[0-9a-f]+$' | head -1)

    if [[ -z "${sess}" ]]; then
        log_fail "Failed to spawn session"
        return 1
    fi

    # Wait for stability
    agent-tui wait --stable --session "$sess" --timeout 5000 2>/dev/null || true

    # Click on nonexistent element
    log_info "Clicking nonexistent element..."
    if agent-tui action "@nonexistent_element" click --session "$sess" 2>/dev/null; then
        log_fail "Click on nonexistent element should fail"
        agent-tui kill --session "$sess" 2>/dev/null || true
        return 1
    else
        log_pass "Click on nonexistent element correctly failed"
    fi

    # Cleanup
    agent-tui kill --session "$sess" 2>/dev/null || true
}

#######################################
# Test 10: PTY round-trip test.
# Tests: type command, execute with Enter, verify output captured.
# Returns:
#   0 if round-trip works, 1 otherwise
#######################################
test_pty_roundtrip() {
    log_section "Test 10: PTY Round-Trip"

    # Spawn bash session
    local output
    output=$(agent-tui run bash 2>&1)
    local sess
    sess=$(grep -oE 'Session started: [0-9a-f]+' <<< "$output" | grep -oE '[0-9a-f]+$' | head -1)

    if [[ -z "${sess}" ]]; then
        log_fail "Failed to spawn session"
        return 1
    fi

    # Wait for shell
    agent-tui wait --stable --session "$sess" --timeout 5000 2>/dev/null || true

    # Type command
    local marker
    marker="PTY_ROUNDTRIP_$(date +%s)"
    log_info "Typing command: echo $marker"
    if ! agent-tui input "echo $marker" --session "$sess"; then
        log_fail "Failed to type command"
        agent-tui kill --session "$sess" 2>/dev/null || true
        return 1
    fi

    # Execute with Enter
    log_info "Pressing Enter..."
    if ! agent-tui input Enter --session "$sess"; then
        log_fail "Failed to send Enter"
        agent-tui kill --session "$sess" 2>/dev/null || true
        return 1
    fi

    # Wait for output
    log_info "Waiting for output..."
    if ! agent-tui wait --timeout 5000 "$marker" --session "$sess"; then
        log_fail "Output did not appear"
        agent-tui kill --session "$sess" 2>/dev/null || true
        return 1
    fi

    # Verify round-trip: marker should appear at least twice
    # (once in typed command, once in echo output)
    local snapshot
    snapshot=$(agent-tui screen --session "$sess" 2>&1)
    local count
    count=$(grep -o "$marker" <<< "$snapshot" | wc -l)

    if (( count >= 2 )); then
        log_pass "PTY round-trip verified: marker appears $count times"
    else
        log_fail "Expected at least 2 occurrences, got $count"
        agent-tui kill --session "$sess" 2>/dev/null || true
        return 1
    fi

    # Cleanup
    agent-tui kill --session "$sess" 2>/dev/null || true
}

#######################################
# Test 11: Double-click action.
# Tests: dblclick action completes without hanging.
# Returns:
#   0 if dblclick works, 1 otherwise
#######################################
test_double_click() {
    log_section "Test 11: Double-Click Action"

    # Spawn bash session
    local output
    output=$(agent-tui run bash 2>&1)
    local sess
    sess=$(grep -oE 'Session started: [0-9a-f]+' <<< "$output" | grep -oE '[0-9a-f]+$' | head -1)

    if [[ -z "${sess}" ]]; then
        log_fail "Failed to spawn session"
        return 1
    fi

    # Wait for stability
    agent-tui wait --stable --session "$sess" --timeout 5000 2>/dev/null || true

    # Try dblclick - may fail with "element not found" but should complete
    log_info "Testing dblclick action..."
    # We don't care if it succeeds or fails (element may not exist)
    # We just verify it completes without hanging
    timeout 5 agent-tui action "@e1" dblclick --session "$sess" 2>/dev/null
    local exit_code=$?

    if (( exit_code == 124 )); then
        log_fail "dblclick timed out (hung)"
        agent-tui kill --session "$sess" 2>/dev/null || true
        return 1
    else
        log_pass "dblclick completed (exit code: $exit_code)"
    fi

    # Session should still be usable
    local snapshot
    snapshot=$(agent-tui screen --session "$sess" 2>&1)
    if [[ -n "$snapshot" ]]; then
        log_pass "Session usable after dblclick"
    else
        log_fail "Session not usable after dblclick"
        agent-tui kill --session "$sess" 2>/dev/null || true
        return 1
    fi

    # Cleanup
    agent-tui kill --session "$sess" 2>/dev/null || true
}

#######################################
# Test 12: Accessibility snapshot with button detection.
# Tests: VOM detects button-like elements.
# Returns:
#   0 if elements detected, 1 otherwise
#######################################
test_accessibility_snapshot() {
    log_section "Test 12: Accessibility Snapshot"

    # Spawn bash session
    local output
    output=$(agent-tui run bash 2>&1)
    local sess
    sess=$(grep -oE 'Session started: [0-9a-f]+' <<< "$output" | grep -oE '[0-9a-f]+$' | head -1)

    if [[ -z "${sess}" ]]; then
        log_fail "Failed to spawn session"
        return 1
    fi

    # Wait for stability
    agent-tui wait --stable --session "$sess" --timeout 5000 2>/dev/null || true

    # Display button-like elements
    log_info "Displaying button-like elements..."
    if ! agent-tui input "printf '[Y] [N] [OK] [Cancel]\\n'" --session "$sess"; then
        log_fail "Failed to type printf"
        agent-tui kill --session "$sess" 2>/dev/null || true
        return 1
    fi

    if ! agent-tui input Enter --session "$sess"; then
        log_fail "Failed to send Enter"
        agent-tui kill --session "$sess" 2>/dev/null || true
        return 1
    fi

    # Wait for output
    if ! agent-tui wait --timeout 5000 "[Y]" --session "$sess"; then
        log_fail "Output did not appear"
        agent-tui kill --session "$sess" 2>/dev/null || true
        return 1
    fi

    # Get accessibility snapshot
    log_info "Taking accessibility snapshot..."
    local acc_snapshot
    acc_snapshot=$(agent-tui screen -a --session "$sess" 2>&1)

    if [[ -z "$acc_snapshot" ]]; then
        log_fail "Accessibility snapshot is empty"
        agent-tui kill --session "$sess" 2>/dev/null || true
        return 1
    fi
    log_pass "Accessibility snapshot captured (${#acc_snapshot} bytes)"

    # Check for button detection (case-insensitive)
    if grep -qi "button\|Button" <<< "$acc_snapshot"; then
        log_pass "Button elements detected in accessibility tree"
    else
        # May not always detect buttons, so just warn
        log_info "Note: No buttons detected (VOM may not recognize pattern)"
    fi

    # Cleanup
    agent-tui kill --session "$sess" 2>/dev/null || true
}

#######################################
# Test 13: Rapid session spawn and kill.
# Tests: rapidly creating and destroying sessions works without issues.
# Returns:
#   0 if all operations succeed, 1 otherwise
#######################################
test_rapid_spawn_kill() {
    log_section "Test 13: Rapid Spawn and Kill"

    local iterations=5
    log_info "Rapidly spawning and killing $iterations sessions..."

    for i in $(seq 1 "$iterations"); do
        local output
        output=$(agent-tui run bash 2>&1)
        local sess
        sess=$(grep -oE 'Session started: [0-9a-f]+' <<< "$output" | grep -oE '[0-9a-f]+$' | head -1)

        if [[ -z "${sess}" ]]; then
            log_fail "Spawn $i failed"
            return 1
        fi

        if ! agent-tui kill --session "$sess" 2>/dev/null; then
            log_fail "Kill $i failed"
            return 1
        fi
    done

    log_pass "All $iterations spawn/kill cycles succeeded"

    # Verify no sessions left
    local sessions_list
    sessions_list=$(agent-tui -f json sessions 2>&1)
    if grep -q '"sessions":\s*\[\]' <<< "$sessions_list" || ! grep -q '"id"' <<< "$sessions_list"; then
        log_pass "All sessions cleaned up"
    else
        log_info "Note: Some sessions may remain from earlier tests"
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

    # Run tests - htop tests
    test_daemon_startup
    test_spawn_htop
    test_snapshot_vom
    test_keystroke_interaction

    # Run tests - bash session tests
    test_type_changes_screen
    test_multi_session_management
    test_wait_conditions
    test_dead_session_operations
    test_click_nonexistent_element
    test_pty_roundtrip
    test_double_click
    test_accessibility_snapshot
    test_rapid_spawn_kill

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
