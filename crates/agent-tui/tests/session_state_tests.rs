//! Session state machine tests
//!
//! Tests for session lifecycle management and state transitions.

mod common;

use common::{MockResponse, TEST_SESSION_ID, TestHarness};
use predicates::prelude::*;
use serde_json::json;

// =============================================================================
// Session Not Found Tests
// =============================================================================

#[test]
fn test_killed_session_returns_not_found_on_snapshot() {
    let harness = TestHarness::new();

    // First kill returns success
    harness.run(&["kill"]).success();

    // Configure subsequent snapshot to return session not found
    harness.set_error_response("snapshot", -32001, "Session not found");

    harness
        .run(&["screen"])
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_killed_session_returns_not_found_on_click() {
    let harness = TestHarness::new();

    harness.set_error_response("click", -32001, "Session not found: killed-session");

    harness
        .run(&["action", "@btn1"])
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_killed_session_returns_not_found_on_type() {
    let harness = TestHarness::new();

    harness.set_error_response("type", -32001, "Session not found");

    harness
        .run(&["input", "hello"])
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// =============================================================================
// No Active Session Tests
// =============================================================================

#[test]
fn test_snap_without_active_session() {
    let harness = TestHarness::new();

    harness.set_error_response("snapshot", -32002, "No active session");

    harness
        .run(&["screen"])
        .failure()
        .stderr(predicate::str::contains("No active session"));
}

#[test]
fn test_click_without_active_session() {
    let harness = TestHarness::new();

    harness.set_error_response("click", -32002, "No active session");

    harness
        .run(&["action", "@btn1"])
        .failure()
        .stderr(predicate::str::contains("No active session"));
}

// =============================================================================
// Session Limit Tests
// =============================================================================

#[test]
fn test_session_limit_reached() {
    let harness = TestHarness::new();

    harness.set_response(
        "spawn",
        MockResponse::StructuredError {
            code: -32006,
            message: "Maximum session limit reached (16)".to_string(),
            category: Some("busy".to_string()),
            retryable: Some(false),
            context: Some(json!({
                "current_count": 16,
                "max_sessions": 16
            })),
            suggestion: Some("Kill an existing session first".to_string()),
        },
    );

    harness
        .run(&["run", "bash"])
        .failure()
        .stderr(predicate::str::contains("limit"));
}

// =============================================================================
// Active Session Switching Tests
// =============================================================================

#[test]
fn test_sessions_shows_active_marker() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "sessions",
        json!({
            "sessions": [
                {
                    "id": "session-1",
                    "command": "bash",
                    "running": true,
                    "pid": 1234,
                    "created_at": "2024-01-01T00:00:00Z",
                    "size": { "cols": 120, "rows": 40 }
                },
                {
                    "id": "session-2",
                    "command": "vim",
                    "running": true,
                    "pid": 5678,
                    "created_at": "2024-01-01T00:00:00Z",
                    "size": { "cols": 80, "rows": 24 }
                }
            ],
            "active_session": "session-1"
        }),
    );

    harness
        .run(&["sessions"])
        .success()
        .stdout(predicate::str::contains("session-1"))
        .stdout(predicate::str::contains("(active)"));
}

#[test]
fn test_attach_changes_active_session() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "attach",
        json!({
            "success": true,
            "session": "session-2"
        }),
    );

    harness.run(&["attach", "session-2"]).success();

    harness.assert_method_called("attach");
}

#[test]
fn test_attach_nonexistent_session_fails() {
    let harness = TestHarness::new();

    harness.set_error_response("attach", -32001, "Session not found: nonexistent");

    harness
        .run(&["attach", "nonexistent"])
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// =============================================================================
// Session Info Tests
// =============================================================================

#[test]
fn test_sessions_empty_shows_no_sessions() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "sessions",
        json!({
            "sessions": [],
            "active_session": null
        }),
    );

    harness
        .run(&["sessions"])
        .success()
        .stdout(predicate::str::contains("No active sessions"));
}

#[test]
fn test_sessions_shows_multiple_sessions() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "sessions",
        json!({
            "sessions": [
                {
                    "id": "session-1",
                    "command": "bash",
                    "running": true,
                    "pid": 1001,
                    "created_at": "2024-01-01T00:00:00Z",
                    "size": { "cols": 120, "rows": 40 }
                },
                {
                    "id": "session-2",
                    "command": "vim",
                    "running": true,
                    "pid": 1002,
                    "created_at": "2024-01-01T00:00:00Z",
                    "size": { "cols": 80, "rows": 24 }
                },
                {
                    "id": "session-3",
                    "command": "htop",
                    "running": true,
                    "pid": 1003,
                    "created_at": "2024-01-01T00:00:00Z",
                    "size": { "cols": 100, "rows": 30 }
                }
            ],
            "active_session": "session-2"
        }),
    );

    harness
        .run(&["sessions"])
        .success()
        .stdout(predicate::str::contains("session-1"))
        .stdout(predicate::str::contains("session-2"))
        .stdout(predicate::str::contains("session-3"))
        .stdout(predicate::str::contains("vim"));
}

// =============================================================================
// Kill Session Tests
// =============================================================================

#[test]
fn test_kill_active_session() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "kill",
        json!({
            "success": true,
            "session_id": TEST_SESSION_ID
        }),
    );

    harness
        .run(&["kill"])
        .success()
        .stdout(predicate::str::contains("killed"));
}

#[test]
fn test_kill_specific_session() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "kill",
        json!({
            "success": true,
            "session_id": "specific-session"
        }),
    );

    harness.run(&["kill", "-s", "specific-session"]).success();
    harness.assert_method_called("kill");
}

#[test]
fn test_kill_nonexistent_session_fails() {
    let harness = TestHarness::new();

    harness.set_error_response("kill", -32001, "Session not found: nonexistent");

    harness
        .run(&["kill", "-s", "nonexistent"])
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// =============================================================================
// Restart Session Tests
// =============================================================================

#[test]
fn test_restart_session() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "restart",
        json!({
            "success": true,
            "old_session_id": "old-session",
            "new_session_id": "new-session",
            "command": "bash",
            "pid": 12345
        }),
    );

    harness
        .run(&["restart"])
        .success()
        .stdout(predicate::str::contains("new-session"));
}

#[test]
fn test_restart_nonexistent_session_fails() {
    let harness = TestHarness::new();

    harness.set_error_response("restart", -32001, "Session not found");

    harness
        .run(&["restart"])
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// =============================================================================
// Session-Specific Command Tests
// =============================================================================

#[test]
fn test_snap_with_session_option() {
    let harness = TestHarness::new();

    harness.run(&["-s", "specific-session", "screen"]).success();
    harness.assert_method_called("snapshot");
}

#[test]
fn test_click_with_session_option() {
    let harness = TestHarness::new();

    harness
        .run(&["-s", "specific-session", "action", "@btn1"])
        .success();

    harness.assert_method_called("click");
}

// =============================================================================
// Session Running State Tests
// =============================================================================

#[test]
fn test_sessions_shows_running_state() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "sessions",
        json!({
            "sessions": [
                {
                    "id": "running-session",
                    "command": "bash",
                    "running": true,
                    "pid": 1234,
                    "created_at": "2024-01-01T00:00:00Z",
                    "size": { "cols": 120, "rows": 40 }
                },
                {
                    "id": "exited-session",
                    "command": "echo hi",
                    "running": false,
                    "pid": 0,
                    "created_at": "2024-01-01T00:00:00Z",
                    "size": { "cols": 80, "rows": 24 }
                }
            ],
            "active_session": "running-session"
        }),
    );

    harness
        .run(&["sessions"])
        .success()
        .stdout(predicate::str::contains("running-session"))
        .stdout(predicate::str::contains("exited-session"));
}
