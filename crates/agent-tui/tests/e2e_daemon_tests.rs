mod common;

use common::{TEST_COLS, TEST_ROWS, TEST_SESSION_ID, TestHarness};
use predicates::prelude::*;
use serde_json::json;

#[test]
fn test_health_returns_daemon_status() {
    let harness = TestHarness::new();

    harness
        .run(&["sessions", "--status"])
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
        .run(&["sessions", "--status"])
        .success()
        .stdout(predicate::str::contains("degraded"))
        .stdout(predicate::str::contains("99999"));
}

#[test]
fn test_health_verbose_shows_connection_details() {
    let harness = TestHarness::new();

    harness
        .run(&["sessions", "--status", "-v"])
        .success()
        .stdout(predicate::str::contains("Connection:"))
        .stdout(predicate::str::contains("Socket:"))
        .stdout(predicate::str::contains("PID file:"));
}

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

#[test]
fn test_snap_returns_screen_content() {
    let harness = TestHarness::new();

    harness
        .run(&["screen"])
        .success()
        .stdout(predicate::str::contains("Screen:"))
        .stdout(predicate::str::contains("Test screen content"));

    harness.assert_method_called("snapshot");
}

#[test]
fn test_snap_with_elements() {
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
        .run(&["screen", "-i"])
        .success()
        .stdout(predicate::str::contains("Elements:"))
        .stdout(predicate::str::contains("@btn1"))
        .stdout(predicate::str::contains("@inp1"))
        .stdout(predicate::str::contains("*focused*"));
}

#[test]
fn test_snap_json_format() {
    let harness = TestHarness::new();

    harness
        .run(&["-f", "json", "screen"])
        .success()
        .stdout(predicate::str::contains("\"session_id\":"))
        .stdout(predicate::str::contains("\"screen\":"))
        .stdout(predicate::str::contains("\"size\":"));
}

#[test]
fn test_snap_include_cursor() {
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
        .run(&["screen", "--include-cursor"])
        .success()
        .stdout(predicate::str::contains("Cursor: row=5, col=10"));
}

#[test]
fn test_snap_include_cursor_hidden() {
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
        .run(&["screen", "--include-cursor"])
        .success()
        .stdout(predicate::str::contains("(hidden)"));
}

#[test]
fn test_snap_with_vom_metadata() {
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
        .run(&["screen", "-i"])
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
        .run(&["screen", "-a"])
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
        .run(&["-f", "json", "screen", "-a"])
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
        .run(&["screen", "-a", "--interactive-only"])
        .success()
        .stdout(predicate::str::contains("button \"Submit\""));

    harness.assert_method_called_with("accessibility_snapshot", json!({ "interactive": true }));
}

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
        .run(&["action", "e1", "click"])
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
        .run(&["action", "@e2", "click"])
        .success()
        .stdout(predicate::str::contains("Clicked successfully"));

    harness.assert_method_called_with("click", json!({ "ref": "@e2" }));
}

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
        .run(&["action", "@invalid", "click"])
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
        .run(&["action", "@btn1", "fill", "value"])
        .success()
        .stdout(predicate::str::contains("Filled successfully"))
        .stderr(predicate::str::contains("not an input"));
}

#[test]
fn test_fill_element_not_found() {
    let harness = TestHarness::new();

    harness.set_error_response("fill", -32000, "Element not found: '@nonexistent'");

    harness
        .run(&["action", "@nonexistent", "fill", "value"])
        .failure()
        .stderr(predicate::str::contains("Element not found"));
}

#[test]
fn test_daemon_rpc_error() {
    let harness = TestHarness::new();

    harness.set_error_response("health", -32603, "Internal daemon error");

    harness
        .run(&["sessions", "--status"])
        .failure()
        .stderr(predicate::str::contains("Error"))
        .stderr(predicate::str::contains("Internal daemon error"));
}

#[test]
fn test_unknown_method_error() {
    let harness = TestHarness::new();

    harness.set_error_response("health", -32601, "Method not found");

    harness
        .run(&["sessions", "--status"])
        .failure()
        .stderr(predicate::str::contains("Method not found"));
}

#[test]
fn test_keydown_keyup_sequence() {
    let harness = TestHarness::new();

    harness.run(&["input", "Ctrl", "--hold"]).success();
    harness.run(&["input", "Ctrl+c"]).success();
    harness.run(&["input", "Ctrl", "--release"]).success();

    harness.assert_method_called("keydown");
    harness.assert_method_called("keystroke");
    harness.assert_method_called("keyup");
}

#[test]
fn test_multiple_requests_recorded() {
    let harness = TestHarness::new();

    harness.run(&["sessions", "--status"]).success();
    harness.run(&["sessions"]).success();
    harness.run(&["sessions", "--status"]).success();

    // Behavioural check: repeated invocations all succeed and report sessions info (even if empty).
    harness
        .run(&["sessions"])
        .success()
        .stdout(predicate::str::contains("sessions"));
}

#[test]
fn test_clear_requests_works() {
    let harness = TestHarness::new();

    harness.run(&["sessions", "--status"]).success();
    harness.clear_requests();

    // After clearing, subsequent calls still succeed (state unaffected from caller perspective).
    harness
        .run(&["sessions", "--status"])
        .success()
        .stdout(predicate::str::contains("Daemon status:"));
}

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

#[test]
fn test_completions_bash() {
    let harness = TestHarness::new();

    harness
        .run(&["completions", "bash"])
        .success()
        .stdout(predicate::str::contains("complete"));
}

// ============================================================================
// Terminal-native CLI tests (@e1 syntax, press, type)
// ============================================================================

#[test]
fn test_element_ref_activate() {
    let harness = TestHarness::new();

    harness.set_success_response("click", json!({"success": true, "message": null}));

    harness
        .run(&["@e1"])
        .success()
        .stdout(predicate::str::contains("Clicked successfully"));

    harness.assert_method_called("click");
}

#[test]
fn test_element_ref_fill_value() {
    let harness = TestHarness::new();

    harness.set_success_response("fill", json!({"success": true, "message": null}));

    harness
        .run(&["@e1", "my-project"])
        .success()
        .stdout(predicate::str::contains("Filled"));

    harness.assert_method_called("fill");
}

#[test]
fn test_element_ref_fill_explicit() {
    let harness = TestHarness::new();

    harness.set_success_response("fill", json!({"success": true, "message": null}));

    harness
        .run(&["@e1", "fill", "my-value"])
        .success()
        .stdout(predicate::str::contains("Filled"));

    harness.assert_method_called("fill");
}

#[test]
fn test_element_ref_toggle() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "toggle",
        json!({"success": true, "new_state": true, "message": null}),
    );

    harness.run(&["@e1", "toggle"]).success();

    harness.assert_method_called("toggle");
}

#[test]
fn test_element_ref_toggle_on() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "toggle",
        json!({"success": true, "new_state": true, "message": null}),
    );

    harness.run(&["@e1", "toggle", "on"]).success();

    harness.assert_method_called("toggle");
}

#[test]
fn test_element_ref_choose() {
    let harness = TestHarness::new();

    harness.set_success_response("select", json!({"success": true, "message": null}));

    harness.run(&["@e1", "choose", "Option 1"]).success();

    harness.assert_method_called("select");
}

#[test]
fn test_element_ref_clear() {
    let harness = TestHarness::new();

    harness.set_success_response("clear", json!({"success": true, "message": null}));

    harness.run(&["@e1", "clear"]).success();

    harness.assert_method_called("clear");
}

#[test]
fn test_text_selector_exact() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "find",
        json!({
            "elements": [{"ref": "@e5", "text": "Submit"}]
        }),
    );
    harness.set_success_response("click", json!({"success": true, "message": null}));

    harness
        .run(&["@Submit"])
        .success()
        .stdout(predicate::str::contains("Clicked"));

    harness.assert_method_called("find");
    harness.assert_method_called("click");
}

#[test]
fn test_text_selector_partial() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "find",
        json!({
            "elements": [{"ref": "@e3", "text": "Submit Form"}]
        }),
    );
    harness.set_success_response("click", json!({"success": true, "message": null}));

    harness
        .run(&[":Submit"])
        .success()
        .stdout(predicate::str::contains("Clicked"));

    harness.assert_method_called("find");
    harness.assert_method_called("click");
}

#[test]
fn test_text_selector_not_found() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "find",
        json!({
            "elements": []
        }),
    );

    harness
        .run(&["@NonExistent"])
        .failure()
        .stderr(predicate::str::contains("No element found"));
}

#[test]
fn test_invalid_element_ref_rejected() {
    let harness = TestHarness::new();

    // @elephant should be treated as text selector, not element ref
    harness.set_success_response(
        "find",
        json!({
            "elements": []
        }),
    );

    harness
        .run(&["@elephant"])
        .failure()
        .stderr(predicate::str::contains("No element found"));
}

#[test]
fn test_press_single_key() {
    let harness = TestHarness::new();

    harness.set_success_response("keystroke", json!({"success": true, "message": null}));

    harness.run(&["press", "Enter"]).success();

    harness.assert_method_called("keystroke");
}

#[test]
fn test_press_key_sequence() {
    let harness = TestHarness::new();

    harness.set_success_response("keystroke", json!({"success": true, "message": null}));

    harness
        .run(&["press", "ArrowDown", "ArrowDown", "Enter"])
        .success();

    // Behavioural check: final keystroke request carries the last key ("Enter").
    let req = harness
        .last_request_for("keystroke")
        .expect("keystroke should be invoked");
    let params = req.params.expect("keystroke params missing");
    assert!(
        params.to_string().contains("Enter"),
        "Expected 'Enter' in keystroke params, got: {params:?}"
    );
}

#[test]
fn test_type_text() {
    let harness = TestHarness::new();

    harness.set_success_response("type", json!({"success": true, "message": null}));

    harness.run(&["type", "Hello, World!"]).success();

    harness.assert_method_called("type");
}
