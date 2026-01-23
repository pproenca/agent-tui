//! E2E tests with mock daemon
//!
//! These tests verify CLI↔daemon integration using the MockDaemon infrastructure.
//! Tests focus on:
//! - Error response handling and propagation
//! - Complex response processing (warnings, multi-step operations)
//! - Output formatting for various response types
//! - Multi-command sequences
//!
//! ## Note on Test Pyramid
//! CLI argument parsing and simple param serialization are tested as unit tests
//! in `commands.rs` (parsing) and `handlers.rs` (formatting). This file focuses
//! on true integration scenarios that require a mock daemon.

mod common;

use common::{TEST_COLS, TEST_ROWS, TEST_SESSION_ID, TestHarness};
use predicates::prelude::*;
use serde_json::json;

// =============================================================================
// Health Command Tests - Response Processing
// =============================================================================

#[test]
fn test_health_returns_daemon_status() {
    let harness = TestHarness::new();

    harness
        .run(&["health"])
        .success()
        .stdout(predicate::str::contains("Daemon status:"))
        .stdout(predicate::str::contains("healthy"))
        .stdout(predicate::str::contains("PID:"))
        .stdout(predicate::str::contains("Uptime:"))
        .stdout(predicate::str::contains("Sessions:"))
        .stdout(predicate::str::contains("Version:"));

    harness.assert_method_called("health");
}

#[test]
fn test_health_with_custom_response() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "health",
        json!({
            "status": "degraded",
            "pid": 99999,
            "uptime_ms": 3600000,
            "session_count": 5,
            "version": "2.0.0-custom",
            "degradation_reasons": ["High memory usage"]
        }),
    );

    harness
        .run(&["health"])
        .success()
        .stdout(predicate::str::contains("degraded"))
        .stdout(predicate::str::contains("99999"));
}

#[test]
fn test_health_verbose_shows_connection_details() {
    let harness = TestHarness::new();

    harness
        .run(&["health", "-v"])
        .success()
        .stdout(predicate::str::contains("Connection:"))
        .stdout(predicate::str::contains("Socket:"))
        .stdout(predicate::str::contains("PID file:"));
}

// =============================================================================
// Session Management Tests
// =============================================================================

#[test]
fn test_sessions_empty_list() {
    let harness = TestHarness::new();

    harness
        .run(&["sessions"])
        .success()
        .stdout(predicate::str::contains("No active sessions"));
}

#[test]
fn test_sessions_with_active_sessions() {
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
        .stdout(predicate::str::contains("Active sessions:"))
        .stdout(predicate::str::contains("session-1"))
        .stdout(predicate::str::contains("bash"))
        .stdout(predicate::str::contains("session-2"))
        .stdout(predicate::str::contains("vim"))
        .stdout(predicate::str::contains("(active)"));
}

#[test]
fn test_kill_session() {
    let harness = TestHarness::new();

    harness
        .run(&["kill"])
        .success()
        .stdout(predicate::str::contains("Session"))
        .stdout(predicate::str::contains("killed"));

    harness.assert_method_called("kill");
}

// =============================================================================
// Snapshot Tests - Response Formatting
// =============================================================================

#[test]
fn test_snapshot_returns_screen_content() {
    let harness = TestHarness::new();

    harness
        .run(&["snapshot"])
        .success()
        .stdout(predicate::str::contains("Screen:"))
        .stdout(predicate::str::contains("Test screen content"));

    harness.assert_method_called("snapshot");
}

#[test]
fn test_snapshot_with_elements() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "screen": "Test screen\n",
            "elements": [
                {
                    "ref": "@btn1",
                    "type": "button",
                    "label": "Submit",
                    "position": { "row": 5, "col": 10 },
                    "focused": true,
                    "selected": false
                },
                {
                    "ref": "@inp1",
                    "type": "input",
                    "label": "Name",
                    "value": "",
                    "position": { "row": 3, "col": 5 },
                    "focused": false,
                    "selected": false
                }
            ],
            "cursor": { "row": 0, "col": 0, "visible": true },
            "size": { "cols": TEST_COLS, "rows": TEST_ROWS }
        }),
    );

    harness
        .run(&["snapshot", "-i"])
        .success()
        .stdout(predicate::str::contains("Elements:"))
        .stdout(predicate::str::contains("@btn1"))
        .stdout(predicate::str::contains("@inp1"))
        .stdout(predicate::str::contains("*focused*"));
}

#[test]
fn test_snapshot_json_format() {
    let harness = TestHarness::new();

    harness
        .run(&["-f", "json", "snapshot"])
        .success()
        .stdout(predicate::str::contains("\"session_id\":"))
        .stdout(predicate::str::contains("\"screen\":"))
        .stdout(predicate::str::contains("\"size\":"));
}

#[test]
fn test_snapshot_include_cursor() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "screen": "Test screen\n",
            "cursor": { "row": 5, "col": 10, "visible": true },
            "size": { "cols": TEST_COLS, "rows": TEST_ROWS }
        }),
    );

    harness
        .run(&["snapshot", "--include-cursor"])
        .success()
        .stdout(predicate::str::contains("Cursor: row=5, col=10"));
}

#[test]
fn test_snapshot_include_cursor_hidden() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "screen": "Test screen\n",
            "cursor": { "row": 0, "col": 0, "visible": false },
            "size": { "cols": TEST_COLS, "rows": TEST_ROWS }
        }),
    );

    harness
        .run(&["snapshot", "--include-cursor"])
        .success()
        .stdout(predicate::str::contains("(hidden)"));
}

#[test]
fn test_snapshot_with_vom_metadata() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "screen": "Test screen\n",
            "elements": [
                {
                    "ref": "@e1",
                    "type": "button",
                    "label": "[OK]",
                    "value": null,
                    "position": { "row": 5, "col": 10, "width": 4, "height": 1 },
                    "focused": false,
                    "selected": false,
                    "vom_id": "550e8400-e29b-41d4-a716-446655440000",
                    "visual_hash": 12345678
                }
            ],
            "cursor": { "row": 0, "col": 0, "visible": true },
            "size": { "cols": TEST_COLS, "rows": TEST_ROWS },
            "stats": {
                "detection": "vom"
            }
        }),
    );

    harness
        .run(&["snapshot", "-i"])
        .success()
        .stdout(predicate::str::contains("@e1"))
        .stdout(predicate::str::contains("button"));
}

#[test]
fn test_accessibility_snapshot_returns_tree_format() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "accessibility_snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "tree": "- button \"Submit\" [ref=e1]\n- textbox \"Search\" [ref=e2]\n- static_text \"Welcome\"",
            "refs": {
                "e1": { "row": 5, "col": 10, "width": 8, "height": 1 },
                "e2": { "row": 7, "col": 10, "width": 20, "height": 1 }
            },
            "stats": {
                "total_elements": 3,
                "interactive_elements": 2,
                "filtered_elements": 0
            }
        }),
    );

    harness
        .run(&["snapshot", "-a"])
        .success()
        .stdout(predicate::str::contains("button \"Submit\" [ref=e1]"))
        .stdout(predicate::str::contains("textbox \"Search\" [ref=e2]"))
        .stdout(predicate::str::contains("static_text \"Welcome\""));

    harness.assert_method_called("accessibility_snapshot");
}

#[test]
fn test_accessibility_snapshot_json_format() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "accessibility_snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "tree": "- button \"OK\" [ref=e1]",
            "refs": {
                "e1": { "row": 0, "col": 0, "width": 4, "height": 1 }
            },
            "stats": {
                "total_elements": 1,
                "interactive_elements": 1,
                "filtered_elements": 0
            }
        }),
    );

    harness
        .run(&["-f", "json", "snapshot", "-a"])
        .success()
        .stdout(predicate::str::contains("\"tree\":"))
        .stdout(predicate::str::contains("\"refs\":"))
        .stdout(predicate::str::contains("\"stats\":"));
}

#[test]
fn test_accessibility_snapshot_interactive_only() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "accessibility_snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "tree": "- button \"Submit\" [ref=e1]",
            "refs": {
                "e1": { "row": 5, "col": 10, "width": 8, "height": 1 }
            },
            "stats": {
                "total_elements": 1,
                "interactive_elements": 1,
                "filtered_elements": 2
            }
        }),
    );

    harness
        .run(&["snapshot", "-a", "--interactive-only"])
        .success()
        .stdout(predicate::str::contains("button \"Submit\""));

    harness.assert_method_called_with("accessibility_snapshot", json!({ "interactive": true }));
}

// =============================================================================
// Click by Ref Tests - Accessibility Tree Integration
// =============================================================================

#[test]
fn test_click_with_accessibility_ref() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "click",
        json!({
            "success": true,
            "message": null
        }),
    );

    harness
        .run(&["click", "e1"])
        .success()
        .stdout(predicate::str::contains("Clicked successfully"));

    harness.assert_method_called_with("click", json!({ "ref": "e1" }));
}

#[test]
fn test_click_with_at_prefix_ref() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "click",
        json!({
            "success": true,
            "message": null
        }),
    );

    harness
        .run(&["click", "@e2"])
        .success()
        .stdout(predicate::str::contains("Clicked successfully"));

    harness.assert_method_called_with("click", json!({ "ref": "@e2" }));
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_click_error_handling() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "click",
        json!({
            "success": false,
            "message": "Element not found: @invalid"
        }),
    );

    harness
        .run(&["click", "@invalid"])
        .failure()
        .stderr(predicate::str::contains("Click failed"))
        .stderr(predicate::str::contains("Element not found"));
}

#[test]
fn test_fill_warning_for_non_input() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "fill",
        json!({
            "success": true,
            "ref": "@btn1",
            "value": "value",
            "warning": "Warning: '@btn1' is a button not an input field. Fill may not work as expected. Use 'snapshot -i' to see element types."
        }),
    );

    harness
        .run(&["fill", "@btn1", "value"])
        .success()
        .stdout(predicate::str::contains("Filled successfully"))
        .stderr(predicate::str::contains("not an input"));
}

#[test]
fn test_fill_element_not_found() {
    let harness = TestHarness::new();

    harness.set_error_response("fill", -32000, "Element not found: '@nonexistent'");

    harness
        .run(&["fill", "@nonexistent", "value"])
        .failure()
        .stderr(predicate::str::contains("Element not found"));
}

#[test]
fn test_daemon_rpc_error() {
    let harness = TestHarness::new();

    harness.set_error_response("health", -32603, "Internal daemon error");

    harness
        .run(&["health"])
        .failure()
        .stderr(predicate::str::contains("Error"))
        .stderr(predicate::str::contains("Internal daemon error"));
}

#[test]
fn test_unknown_method_error() {
    let harness = TestHarness::new();

    harness.set_error_response("health", -32601, "Method not found");

    harness
        .run(&["health"])
        .failure()
        .stderr(predicate::str::contains("Method not found"));
}

#[test]
fn test_scrollintoview_element_not_found() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "scroll_into_view",
        json!({
            "success": false,
            "message": "Element not found: @missing"
        }),
    );

    harness
        .run(&["scrollintoview", "@missing"])
        .failure()
        .stderr(predicate::str::contains("Element not found"));
}

#[test]
fn test_multiselect_element_not_a_select() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "multiselect",
        json!({
            "success": false,
            "message": "Element is not a select: @btn1"
        }),
    );

    harness
        .run(&["multiselect", "@btn1", "option1"])
        .failure()
        .stderr(predicate::str::contains("not a select"));
}

// =============================================================================
// Multi-Command Sequence Tests
// =============================================================================

#[test]
fn test_keydown_keyup_sequence() {
    let harness = TestHarness::new();

    harness.run(&["keydown", "Ctrl"]).success();
    harness.run(&["press", "c"]).success();
    harness.run(&["keyup", "Ctrl"]).success();

    harness.assert_method_called("keydown");
    harness.assert_method_called("keystroke");
    harness.assert_method_called("keyup");
}

#[test]
fn test_multiple_requests_recorded() {
    let harness = TestHarness::new();

    harness.run(&["health"]).success();
    harness.run(&["sessions"]).success();
    harness.run(&["health"]).success();

    let requests = harness.get_requests();
    assert!(requests.len() >= 3);

    assert_eq!(harness.call_count("health"), 2);
    assert_eq!(harness.call_count("sessions"), 1);
}

#[test]
fn test_clear_requests_works() {
    let harness = TestHarness::new();

    harness.run(&["health"]).success();
    assert_eq!(harness.call_count("health"), 1);

    harness.clear_requests();
    assert_eq!(harness.call_count("health"), 0);

    harness.run(&["health"]).success();
    assert_eq!(harness.call_count("health"), 1);
}

// =============================================================================
// Version/Env Information Tests
// =============================================================================

#[test]
fn test_version_shows_cli_and_daemon() {
    let harness = TestHarness::new();

    harness
        .run(&["version"])
        .success()
        .stdout(predicate::str::contains("agent-tui"))
        .stdout(predicate::str::contains("CLI version:"))
        .stdout(predicate::str::contains("Daemon version:"));
}

#[test]
fn test_env_shows_configuration() {
    let harness = TestHarness::new();

    harness
        .run(&["env"])
        .success()
        .stdout(predicate::str::contains("Environment Configuration:"))
        .stdout(predicate::str::contains("Transport:"))
        .stdout(predicate::str::contains("Socket:"));
}

// =============================================================================
// Assert Command Tests
// =============================================================================

#[test]
fn test_assert_text_condition() {
    let harness = TestHarness::new();

    harness
        .run(&["assert", "text:Test screen"])
        .success()
        .stdout(predicate::str::contains("Assertion passed"));

    let request = harness.last_request_for("snapshot").unwrap();
    let params = request.params.unwrap();
    assert_eq!(
        params["strip_ansi"], true,
        "assert text should strip ANSI codes"
    );
}

// =============================================================================
// Find Command Response Tests
// =============================================================================

#[test]
fn test_find_returns_matching_elements() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "find",
        json!({
            "elements": [
                {
                    "ref": "@btn1",
                    "type": "button",
                    "label": "Submit",
                    "position": { "row": 5, "col": 10 },
                    "focused": false,
                    "selected": false
                }
            ],
            "count": 1
        }),
    );

    harness
        .run(&["find", "--role", "button"])
        .success()
        .stdout(predicate::str::contains("Found 1 element"))
        .stdout(predicate::str::contains("@btn1"));
}

#[test]
fn test_find_no_results() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "find",
        json!({
            "elements": [],
            "count": 0
        }),
    );

    harness
        .run(&["find", "--role", "nonexistent"])
        .success()
        .stdout(predicate::str::contains("No elements found"));
}

// =============================================================================
// Recording Command Response Tests
// =============================================================================

#[test]
fn test_record_stop_with_frame_count() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "record_stop",
        json!({
            "success": true,
            "session_id": TEST_SESSION_ID,
            "frame_count": 100,
            "duration_ms": 5000
        }),
    );

    harness
        .run(&["record-stop"])
        .success()
        .stdout(predicate::str::contains("Recording stopped"))
        .stdout(predicate::str::contains("100 frames"));
}

// =============================================================================
// Wait Command Response Tests
// =============================================================================

#[test]
fn test_wait_timeout() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "wait",
        json!({
            "found": false,
            "elapsed_ms": 5000
        }),
    );

    harness
        .run(&["wait", "-t", "5000", "NotFound"])
        .failure()
        .stderr(predicate::str::contains("Timeout"))
        .stderr(predicate::str::contains("5000ms"));
}

#[test]
fn test_wait_found() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "wait",
        json!({
            "found": true,
            "elapsed_ms": 150
        }),
    );

    harness
        .run(&["wait", "Continue"])
        .success()
        .stdout(predicate::str::contains("Found"))
        .stdout(predicate::str::contains("150ms"));
}

// =============================================================================
// State Check Response Tests
// =============================================================================

#[test]
fn test_is_visible_element_not_found() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "is_visible",
        json!({
            "found": false,
            "visible": false
        }),
    );

    harness
        .run(&["is", "visible", "@missing"])
        .failure()
        .stderr(predicate::str::contains("Element not found"));
}

#[test]
fn test_is_visible_not_visible() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "is_visible",
        json!({
            "found": true,
            "visible": false
        }),
    );

    harness
        .run(&["is", "visible", "@btn1"])
        .failure()
        .stdout(predicate::str::contains("not visible"));
}

// =============================================================================
// Get Command Response Tests
// =============================================================================

#[test]
fn test_get_focused_not_found() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "get_focused",
        json!({
            "found": false
        }),
    );

    harness
        .run(&["get", "focused"])
        .failure()
        .stderr(predicate::str::contains("No focused element"));
}

// =============================================================================
// Completions Command Tests (minimal - just verifies command runs)
// =============================================================================

#[test]
fn test_completions_bash() {
    let harness = TestHarness::new();

    harness
        .run(&["completions", "bash"])
        .success()
        .stdout(predicate::str::contains("complete"));
}
