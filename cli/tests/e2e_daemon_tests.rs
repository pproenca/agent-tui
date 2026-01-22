//! E2E tests with mock daemon
//!
//! These tests verify that the CLI correctly communicates with the daemon
//! by using the MockDaemon infrastructure. Unlike argument parsing tests,
//! these tests verify actual JSON-RPC request/response flows.
//!
//! ## Important Notes
//!
//! The CLI uses serde rename attributes, so element_ref fields are serialized
//! as "ref" in JSON. Tests should check for "ref" in params, not "element_ref".

mod common;

use common::{TestHarness, TEST_COLS, TEST_ROWS, TEST_SESSION_ID};
use predicates::prelude::*;
use serde_json::json;

// =============================================================================
// Health Command Tests
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
fn test_health_json_format() {
    let harness = TestHarness::new();

    harness
        .run(&["-f", "json", "health"])
        .success()
        .stdout(predicate::str::contains("\"status\": \"healthy\""))
        .stdout(predicate::str::contains("\"pid\":"))
        .stdout(predicate::str::contains("\"uptime_ms\":"))
        .stdout(predicate::str::contains("\"session_count\":"))
        .stdout(predicate::str::contains("\"version\":"));
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
// Spawn Command Tests
// =============================================================================

#[test]
fn test_spawn_sends_correct_params() {
    let harness = TestHarness::new();

    harness
        .run(&["spawn", "bash"])
        .success()
        .stdout(predicate::str::contains("Session started:"))
        .stdout(predicate::str::contains(TEST_SESSION_ID));

    harness.assert_method_called("spawn");

    // Verify params sent to daemon
    let request = harness.last_request_for("spawn").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["command"], "bash");
    assert_eq!(params["cols"], 120); // default
    assert_eq!(params["rows"], 40); // default
}

#[test]
fn test_spawn_with_custom_dimensions() {
    let harness = TestHarness::new();

    harness
        .run(&["spawn", "--cols", "80", "--rows", "24", "vim"])
        .success();

    let request = harness.last_request_for("spawn").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["command"], "vim");
    assert_eq!(params["cols"], 80);
    assert_eq!(params["rows"], 24);
}

#[test]
fn test_spawn_with_cwd() {
    let harness = TestHarness::new();

    harness.run(&["spawn", "-d", "/tmp", "bash"]).success();

    let request = harness.last_request_for("spawn").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["cwd"], "/tmp");
}

#[test]
fn test_spawn_with_args() {
    let harness = TestHarness::new();

    harness
        .run(&["spawn", "vim", "--", "file.txt", "-n"])
        .success();

    let request = harness.last_request_for("spawn").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["command"], "vim");
    assert_eq!(params["args"], json!(["file.txt", "-n"]));
}

#[test]
fn test_spawn_json_format() {
    let harness = TestHarness::new();

    harness
        .run(&["-f", "json", "spawn", "bash"])
        .success()
        .stdout(predicate::str::contains("\"session_id\":"))
        .stdout(predicate::str::contains("\"pid\":"));
}

// =============================================================================
// Click Command Tests
// =============================================================================

#[test]
fn test_click_sends_element_ref() {
    let harness = TestHarness::new();

    harness
        .run(&["click", "@btn1"])
        .success()
        .stdout(predicate::str::contains("Clicked successfully"));

    // Note: element_ref is serialized as "ref" via serde rename
    harness.assert_method_called_with("click", json!({"ref": "@btn1"}));
}

#[test]
fn test_click_with_session() {
    let harness = TestHarness::new();

    harness
        .run(&["-s", "my-session", "click", "@btn2"])
        .success();

    let request = harness.last_request_for("click").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["ref"], "@btn2");
    assert_eq!(params["session"], "my-session");
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
        .run(&["click", "@invalid"])
        .failure()
        .stderr(predicate::str::contains("Click failed"))
        .stderr(predicate::str::contains("Element not found"));
}

#[test]
fn test_click_json_format() {
    let harness = TestHarness::new();

    harness
        .run(&["-f", "json", "click", "@btn1"])
        .success()
        .stdout(predicate::str::contains("\"success\": true"));
}

// =============================================================================
// Fill Command Tests
// =============================================================================

#[test]
fn test_fill_sends_ref_and_value() {
    let harness = TestHarness::new();

    harness
        .run(&["fill", "@inp1", "test-value"])
        .success()
        .stdout(predicate::str::contains("Filled successfully"));

    harness.assert_method_called_with(
        "fill",
        json!({
            "ref": "@inp1",
            "value": "test-value"
        }),
    );
}

#[test]
fn test_fill_with_spaces_in_value() {
    let harness = TestHarness::new();

    harness.run(&["fill", "@inp1", "hello world"]).success();

    let request = harness.last_request_for("fill").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["value"], "hello world");
}

#[test]
fn test_fill_error_handling() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "fill",
        json!({
            "success": false,
            "message": "Element is not an input: @btn1"
        }),
    );

    harness
        .run(&["fill", "@btn1", "value"])
        .failure()
        .stderr(predicate::str::contains("Fill failed"))
        .stderr(predicate::str::contains("not an input"));
}

// =============================================================================
// Press Command Tests
// =============================================================================

#[test]
fn test_press_sends_key() {
    let harness = TestHarness::new();

    harness
        .run(&["press", "Enter"])
        .success()
        .stdout(predicate::str::contains("Key pressed"));

    harness.assert_method_called_with("keystroke", json!({"key": "Enter"}));
}

#[test]
fn test_press_with_modifier() {
    let harness = TestHarness::new();

    harness.run(&["press", "Ctrl+C"]).success();

    let request = harness.last_request_for("keystroke").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["key"], "Ctrl+C");
}

#[test]
fn test_press_arrow_keys() {
    let harness = TestHarness::new();

    for key in &["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"] {
        harness.clear_requests();
        harness.run(&["press", key]).success();
        harness.assert_method_called_with("keystroke", json!({"key": key}));
    }
}

#[test]
fn test_press_function_keys() {
    let harness = TestHarness::new();

    for key in &["F1", "F5", "F10", "F12"] {
        harness.clear_requests();
        harness.run(&["press", key]).success();
        harness.assert_method_called_with("keystroke", json!({"key": key}));
    }
}

// =============================================================================
// Snapshot Command Tests
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

    // Note: Element uses "ref" and "type" via serde rename
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

    let request = harness.last_request_for("snapshot").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["include_elements"], true);
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
fn test_snapshot_compact_mode() {
    let harness = TestHarness::new();

    harness.run(&["snapshot", "-i", "-c"]).success();

    let request = harness.last_request_for("snapshot").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["compact"], true);
}

#[test]
fn test_snapshot_with_elements_uses_vom() {
    let harness = TestHarness::new();

    // VOM is now the default detection method for elements
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
                    "checked": null,
                    "disabled": false,
                    "hint": null,
                    "vom_id": "550e8400-e29b-41d4-a716-446655440000",
                    "visual_hash": 12345678
                }
            ],
            "cursor": { "row": 0, "col": 0, "visible": true },
            "size": { "cols": TEST_COLS, "rows": TEST_ROWS },
            "stats": {
                "lines": 24,
                "chars": 1920,
                "elements_total": 1,
                "elements_interactive": 1,
                "elements_shown": 1,
                "detection": "vom"
            }
        }),
    );

    harness
        .run(&["snapshot", "-i"])
        .success()
        .stdout(predicate::str::contains("@e1"))
        .stdout(predicate::str::contains("button"));

    // Verify include_elements was passed to the daemon
    let request = harness.last_request_for("snapshot").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["include_elements"], true);
}

#[test]
fn test_snapshot_with_compact_elements() {
    let harness = TestHarness::new();

    harness.run(&["snapshot", "-i", "-c"]).success();

    let request = harness.last_request_for("snapshot").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["include_elements"], true);
    assert_eq!(params["compact"], true);
}

// =============================================================================
// Wait Command Tests
// =============================================================================

#[test]
fn test_wait_for_text_found() {
    let harness = TestHarness::new();

    harness
        .run(&["wait", "Continue"])
        .success()
        .stdout(predicate::str::contains("Found"))
        .stdout(predicate::str::contains("after"));

    let request = harness.last_request_for("wait").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["text"], "Continue");
}

#[test]
fn test_wait_with_custom_timeout() {
    let harness = TestHarness::new();

    harness.run(&["wait", "-t", "5000", "Loading"]).success();

    let request = harness.last_request_for("wait").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["timeout_ms"], 5000);
}

#[test]
fn test_wait_timeout_failure() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "wait",
        json!({
            "found": false,
            "elapsed_ms": 30000,
            "suggestion": "Text not found. Check the current screen.",
            "screen_context": "Actual screen content here"
        }),
    );

    harness
        .run(&["wait", "NonExistent"])
        .failure()
        .stderr(predicate::str::contains("Timeout"))
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_wait_for_element() {
    let harness = TestHarness::new();

    harness.run(&["wait", "--element", "@btn1"]).success();

    let request = harness.last_request_for("wait").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["target"], "@btn1");
    assert_eq!(params["condition"], "element");
}

#[test]
fn test_wait_for_stable() {
    let harness = TestHarness::new();

    harness.run(&["wait", "--stable"]).success();

    let request = harness.last_request_for("wait").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["condition"], "stable");
}

// =============================================================================
// Sessions Command Tests
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

    // SessionInfo requires created_at field
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

// =============================================================================
// Kill Command Tests
// =============================================================================

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
fn test_kill_specific_session() {
    let harness = TestHarness::new();

    harness.run(&["-s", "my-session", "kill"]).success();

    let request = harness.last_request_for("kill").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["session"], "my-session");
}

// =============================================================================
// Type Command Tests
// =============================================================================

#[test]
fn test_type_text() {
    let harness = TestHarness::new();

    harness
        .run(&["type", "Hello, World!"])
        .success()
        .stdout(predicate::str::contains("Text typed"));

    harness.assert_method_called_with("type", json!({"text": "Hello, World!"}));
}

// =============================================================================
// Snapshot with strip-ansi and include-cursor Tests
// =============================================================================

#[test]
fn test_snapshot_strip_ansi() {
    let harness = TestHarness::new();

    harness.run(&["snapshot", "--strip-ansi"]).success();

    let request = harness.last_request_for("snapshot").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["strip_ansi"], true);
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

    let request = harness.last_request_for("snapshot").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["include_cursor"], true);
}

#[test]
fn test_snapshot_strip_ansi_and_include_cursor_together() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "screen": "Plain text screen\n",
            "cursor": { "row": 3, "col": 7, "visible": true },
            "size": { "cols": TEST_COLS, "rows": TEST_ROWS }
        }),
    );

    harness
        .run(&["snapshot", "--strip-ansi", "--include-cursor"])
        .success()
        .stdout(predicate::str::contains("Cursor: row=3, col=7"));

    let request = harness.last_request_for("snapshot").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["strip_ansi"], true);
    assert_eq!(params["include_cursor"], true);
}

#[test]
fn test_snapshot_elements_with_strip_ansi() {
    let harness = TestHarness::new();

    harness.run(&["snapshot", "-i", "--strip-ansi"]).success();

    let request = harness.last_request_for("snapshot").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["include_elements"], true);
    assert_eq!(params["strip_ansi"], true);
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
fn test_snapshot_all_flags_e2e() {
    let harness = TestHarness::new();

    harness
        .run(&[
            "snapshot",
            "-i",
            "-c",
            "--region",
            "modal",
            "--strip-ansi",
            "--include-cursor",
        ])
        .success();

    let request = harness.last_request_for("snapshot").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["include_elements"], true);
    assert_eq!(params["compact"], true);
    assert_eq!(params["region"], "modal");
    assert_eq!(params["strip_ansi"], true);
    assert_eq!(params["include_cursor"], true);
}

// =============================================================================
// Focus/Clear/Toggle Command Tests
// =============================================================================

#[test]
fn test_focus_element() {
    let harness = TestHarness::new();

    harness
        .run(&["focus", "@inp1"])
        .success()
        .stdout(predicate::str::contains("Focused: @inp1"));

    harness.assert_method_called_with("focus", json!({"ref": "@inp1"}));
}

#[test]
fn test_clear_element() {
    let harness = TestHarness::new();

    harness
        .run(&["clear", "@inp1"])
        .success()
        .stdout(predicate::str::contains("Cleared: @inp1"));

    harness.assert_method_called_with("clear", json!({"ref": "@inp1"}));
}

#[test]
fn test_toggle_checkbox() {
    let harness = TestHarness::new();

    harness
        .run(&["toggle", "@cb1"])
        .success()
        .stdout(predicate::str::contains("@cb1"))
        .stdout(predicate::str::contains("checked"));

    harness.assert_method_called_with("toggle", json!({"ref": "@cb1"}));
}

// =============================================================================
// Get/Is Command Tests (subcommand format)
// =============================================================================

#[test]
fn test_get_text() {
    let harness = TestHarness::new();

    harness
        .run(&["get", "text", "@btn1"])
        .success()
        .stdout(predicate::str::contains("Test Text"));

    harness.assert_method_called("get_text");
}

#[test]
fn test_get_value() {
    let harness = TestHarness::new();

    harness
        .run(&["get", "value", "@inp1"])
        .success()
        .stdout(predicate::str::contains("test-value"));

    harness.assert_method_called("get_value");
}

#[test]
fn test_is_visible() {
    let harness = TestHarness::new();

    harness
        .run(&["is", "visible", "@btn1"])
        .success()
        .stdout(predicate::str::contains("@btn1 is visible"));

    harness.assert_method_called("is_visible");
}

#[test]
fn test_is_focused() {
    let harness = TestHarness::new();

    harness
        .run(&["is", "focused", "@inp1"])
        .success()
        .stdout(predicate::str::contains("@inp1 is focused"));

    harness.assert_method_called("is_focused");
}

#[test]
fn test_is_enabled() {
    let harness = TestHarness::new();

    harness
        .run(&["is", "enabled", "@btn1"])
        .success()
        .stdout(predicate::str::contains("@btn1 is enabled"));

    harness.assert_method_called("is_enabled");

    // Verify request params
    let request = harness.last_request_for("is_enabled").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["ref"], "@btn1");
}

#[test]
fn test_is_checked() {
    let harness = TestHarness::new();

    harness
        .run(&["is", "checked", "@cb1"])
        .success()
        .stdout(predicate::str::contains("@cb1 is checked"));

    harness.assert_method_called("is_checked");

    // Verify request params
    let request = harness.last_request_for("is_checked").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["ref"], "@cb1");
}

#[test]
fn test_count_elements() {
    let harness = TestHarness::new();

    harness
        .run(&["count", "--role", "button"])
        .success()
        .stdout(predicate::str::contains("5"));

    harness.assert_method_called("count");

    // Verify request params
    let request = harness.last_request_for("count").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["role"], "button");
}

#[test]
fn test_count_with_text() {
    let harness = TestHarness::new();

    harness
        .run(&["count", "--text", "Submit"])
        .success()
        .stdout(predicate::str::contains("5"));

    harness.assert_method_called("count");

    // Verify request params
    let request = harness.last_request_for("count").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["text"], "Submit");
}

// =============================================================================
// Select/Scroll Command Tests
// =============================================================================

#[test]
fn test_select_option() {
    let harness = TestHarness::new();

    harness
        .run(&["select", "@sel1", "option1"])
        .success()
        .stdout(predicate::str::contains("Selected: option1"));

    harness.assert_method_called_with(
        "select",
        json!({
            "ref": "@sel1",
            "option": "option1"
        }),
    );
}

#[test]
fn test_scroll_direction() {
    let harness = TestHarness::new();

    for direction in &["up", "down", "left", "right"] {
        harness.clear_requests();
        harness
            .run(&["scroll", direction])
            .success()
            .stdout(predicate::str::contains(format!(
                "Scrolled {} 5",
                direction
            )));

        let request = harness.last_request_for("scroll").unwrap();
        let params = request.params.unwrap();
        assert_eq!(params["direction"], *direction);
        assert_eq!(params["amount"], 5); // default
    }
}

#[test]
fn test_scroll_with_amount() {
    let harness = TestHarness::new();

    harness.run(&["scroll", "down", "-a", "10"]).success();

    let request = harness.last_request_for("scroll").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["amount"], 10);
}

// =============================================================================
// Resize Command Tests
// =============================================================================

#[test]
fn test_resize_terminal() {
    let harness = TestHarness::new();

    // Set custom response with the expected size
    harness.set_success_response(
        "resize",
        json!({
            "success": true,
            "session_id": TEST_SESSION_ID,
            "size": { "cols": 80, "rows": 24 }
        }),
    );

    harness
        .run(&["resize", "--cols", "80", "--rows", "24"])
        .success()
        .stdout(predicate::str::contains("resized to 80x24"));

    let request = harness.last_request_for("resize").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["cols"], 80);
    assert_eq!(params["rows"], 24);
}

// =============================================================================
// Attach Command Tests
// =============================================================================

#[test]
fn test_attach_session() {
    let harness = TestHarness::new();

    harness
        .run(&["attach", "my-session"])
        .success()
        .stdout(predicate::str::contains("Attached to session"));

    let request = harness.last_request_for("attach").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["session"], "my-session");
}

// =============================================================================
// Recording Commands Tests
// =============================================================================

#[test]
fn test_record_start() {
    let harness = TestHarness::new();

    harness
        .run(&["record-start"])
        .success()
        .stdout(predicate::str::contains("Recording started"));

    harness.assert_method_called("record_start");
}

/// Test record-stop command.
#[test]
fn test_record_stop() {
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

    harness.assert_method_called("record_stop");
}

#[test]
fn test_record_status() {
    let harness = TestHarness::new();

    harness
        .run(&["record-status"])
        .success()
        .stdout(predicate::str::contains("Not recording"));

    harness.assert_method_called("record_status");
}

// =============================================================================
// Trace/Console Command Tests
// =============================================================================

#[test]
fn test_trace_command() {
    let harness = TestHarness::new();

    harness.run(&["trace"]).success();

    let request = harness.last_request_for("trace").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["count"], 10); // default
}

#[test]
fn test_console_command() {
    let harness = TestHarness::new();

    harness
        .run(&["console"])
        .success()
        .stdout(predicate::str::contains("line 1"))
        .stdout(predicate::str::contains("line 2"));

    let request = harness.last_request_for("console").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["count"], 100); // default
    assert_eq!(params["clear"], false); // default
}

// =============================================================================
// Find Command Tests
// =============================================================================

#[test]
fn test_find_by_role() {
    let harness = TestHarness::new();

    // Element uses "ref" and "type" via serde rename
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

    let request = harness.last_request_for("find").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["role"], "button");
}

#[test]
fn test_find_focused() {
    let harness = TestHarness::new();

    harness.run(&["find", "--focused"]).success();

    let request = harness.last_request_for("find").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["focused"], true);
}

// =============================================================================
// Error Handling Tests
// =============================================================================

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

    // The mock daemon returns -32601 for unknown methods
    // This test verifies the error propagates correctly
    harness.set_error_response("health", -32601, "Method not found");

    harness
        .run(&["health"])
        .failure()
        .stderr(predicate::str::contains("Method not found"));
}

// =============================================================================
// Request Recording Tests
// =============================================================================

#[test]
fn test_multiple_requests_recorded() {
    let harness = TestHarness::new();

    // Execute multiple commands
    harness.run(&["health"]).success();
    harness.run(&["sessions"]).success();
    harness.run(&["health"]).success();

    // Verify all requests were recorded
    let requests = harness.get_requests();
    assert!(requests.len() >= 3);

    // Health should have been called twice
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
// Version Command Tests
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

// =============================================================================
// Env Command Tests
// =============================================================================

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
// Keydown/Keyup Command Tests
// =============================================================================

#[test]
fn test_keydown_sends_key() {
    let harness = TestHarness::new();

    harness
        .run(&["keydown", "Ctrl"])
        .success()
        .stdout(predicate::str::contains("Key held"));

    harness.assert_method_called_with("keydown", json!({"key": "Ctrl"}));
}

#[test]
fn test_keydown_with_modifier() {
    let harness = TestHarness::new();

    harness.run(&["keydown", "Shift"]).success();

    let request = harness.last_request_for("keydown").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["key"], "Shift");
}

#[test]
fn test_keyup_sends_key() {
    let harness = TestHarness::new();

    harness
        .run(&["keyup", "Ctrl"])
        .success()
        .stdout(predicate::str::contains("Key released"));

    harness.assert_method_called_with("keyup", json!({"key": "Ctrl"}));
}

#[test]
fn test_keyup_with_modifier() {
    let harness = TestHarness::new();

    harness.run(&["keyup", "Alt"]).success();

    let request = harness.last_request_for("keyup").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["key"], "Alt");
}

#[test]
fn test_keydown_keyup_sequence() {
    let harness = TestHarness::new();

    // Simulate a modifier key sequence
    harness.run(&["keydown", "Ctrl"]).success();
    harness.run(&["press", "c"]).success();
    harness.run(&["keyup", "Ctrl"]).success();

    // Verify all three methods were called
    harness.assert_method_called("keydown");
    harness.assert_method_called("keystroke");
    harness.assert_method_called("keyup");
}

// =============================================================================
// Scrollintoview Command Tests
// =============================================================================

#[test]
fn test_scrollintoview_element() {
    let harness = TestHarness::new();

    harness
        .run(&["scrollintoview", "@btn1"])
        .success()
        .stdout(predicate::str::contains("Scrolled to @btn1"));

    harness.assert_method_called_with("scroll_into_view", json!({"ref": "@btn1"}));
}

#[test]
fn test_scrollintoview_with_session() {
    let harness = TestHarness::new();

    harness
        .run(&["-s", "my-session", "scrollintoview", "@item5"])
        .success();

    let request = harness.last_request_for("scroll_into_view").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["ref"], "@item5");
    assert_eq!(params["session"], "my-session");
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

// =============================================================================
// Multiselect Command Tests
// =============================================================================

#[test]
fn test_multiselect_single_option() {
    let harness = TestHarness::new();

    harness
        .run(&["multiselect", "@sel1", "option1"])
        .success()
        .stdout(predicate::str::contains("Selected:"));

    let request = harness.last_request_for("multiselect").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["ref"], "@sel1");
    assert_eq!(params["options"], json!(["option1"]));
}

#[test]
fn test_multiselect_multiple_options() {
    let harness = TestHarness::new();

    harness
        .run(&["multiselect", "@sel1", "option1", "option2", "option3"])
        .success()
        .stdout(predicate::str::contains("Selected:"));

    let request = harness.last_request_for("multiselect").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["ref"], "@sel1");
    assert_eq!(params["options"], json!(["option1", "option2", "option3"]));
}

#[test]
fn test_multiselect_with_session() {
    let harness = TestHarness::new();

    harness
        .run(&["-s", "my-session", "multiselect", "@sel2", "opt1", "opt2"])
        .success();

    let request = harness.last_request_for("multiselect").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["session"], "my-session");
    assert_eq!(params["ref"], "@sel2");
    assert_eq!(params["options"], json!(["opt1", "opt2"]));
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
// Find with Placeholder Tests
// =============================================================================

#[test]
fn test_find_by_placeholder() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "find",
        json!({
            "elements": [
                {
                    "ref": "@inp1",
                    "type": "input",
                    "label": "",
                    "hint": "Search...",
                    "position": { "row": 2, "col": 5 },
                    "focused": false,
                    "selected": false
                }
            ],
            "count": 1
        }),
    );

    harness
        .run(&["find", "--placeholder", "Search"])
        .success()
        .stdout(predicate::str::contains("Found 1 element"))
        .stdout(predicate::str::contains("@inp1"));

    let request = harness.last_request_for("find").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["placeholder"], "Search");
}

#[test]
fn test_find_by_placeholder_with_exact() {
    let harness = TestHarness::new();

    harness
        .run(&["find", "--placeholder", "Search...", "--exact"])
        .success();

    let request = harness.last_request_for("find").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["placeholder"], "Search...");
    assert_eq!(params["exact"], true);
}

// =============================================================================
// Dblclick Command Tests
// =============================================================================

#[test]
fn test_dblclick_sends_element_ref() {
    let harness = TestHarness::new();

    harness
        .run(&["dblclick", "@btn1"])
        .success()
        .stdout(predicate::str::contains("Double-clicked"));

    harness.assert_method_called_with("dbl_click", json!({"ref": "@btn1"}));
}

#[test]
fn test_dblclick_with_session() {
    let harness = TestHarness::new();

    harness
        .run(&["-s", "my-session", "dblclick", "@item1"])
        .success();

    let request = harness.last_request_for("dbl_click").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["ref"], "@item1");
    assert_eq!(params["session"], "my-session");
}

// =============================================================================
// Restart Command Tests
// =============================================================================

#[test]
fn test_restart_session() {
    let harness = TestHarness::new();

    harness
        .run(&["restart"])
        .success()
        .stdout(predicate::str::contains("Restarted"));

    harness.assert_method_called("restart");
}

// =============================================================================
// Selectall Command Tests
// =============================================================================

#[test]
fn test_selectall_element() {
    let harness = TestHarness::new();

    harness
        .run(&["selectall", "@inp1"])
        .success()
        .stdout(predicate::str::contains("Selected all"));

    harness.assert_method_called_with("select_all", json!({"ref": "@inp1"}));
}

// =============================================================================
// Get Focused/Title Tests
// =============================================================================

#[test]
fn test_get_focused() {
    let harness = TestHarness::new();

    harness
        .run(&["get", "focused"])
        .success()
        .stdout(predicate::str::contains("@inp1"));

    harness.assert_method_called("get_focused");
}

#[test]
fn test_get_title() {
    let harness = TestHarness::new();

    harness
        .run(&["get", "title"])
        .success()
        .stdout(predicate::str::contains("bash"));

    harness.assert_method_called("get_title");
}

// =============================================================================
// Check/Uncheck Command Tests
// =============================================================================

#[test]
fn test_check_checkbox() {
    let harness = TestHarness::new();

    harness
        .run(&["check", "@cb1"])
        .success()
        .stdout(predicate::str::contains("checked"));

    // check uses toggle with state: true
    let request = harness.last_request_for("toggle").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["ref"], "@cb1");
    assert_eq!(params["state"], true);
}

#[test]
fn test_uncheck_checkbox() {
    let harness = TestHarness::new();

    harness
        .run(&["uncheck", "@cb1"])
        .success()
        .stdout(predicate::str::contains("unchecked"));

    // uncheck uses toggle with state: false
    let request = harness.last_request_for("toggle").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["ref"], "@cb1");
    assert_eq!(params["state"], false);
}

// =============================================================================
// Errors Command Tests
// =============================================================================

#[test]
fn test_errors_command() {
    let harness = TestHarness::new();

    harness
        .run(&["errors"])
        .success()
        .stdout(predicate::str::contains("No errors captured"));

    harness.assert_method_called("errors");
}

#[test]
fn test_errors_with_count() {
    let harness = TestHarness::new();

    harness.run(&["errors", "-n", "25"]).success();

    let request = harness.last_request_for("errors").unwrap();
    let params = request.params.unwrap();
    assert_eq!(params["count"], 25);
}

// =============================================================================
// Assert Command Tests
// =============================================================================

#[test]
fn test_assert_text_condition() {
    let harness = TestHarness::new();

    // Assert uses snapshot internally to check for text
    harness
        .run(&["assert", "text:Test screen"])
        .success()
        .stdout(predicate::str::contains("Assertion passed"));

    // Verify snapshot was called with strip_ansi for consistent text matching
    let request = harness.last_request_for("snapshot").unwrap();
    let params = request.params.unwrap();
    assert_eq!(
        params["strip_ansi"], true,
        "assert text should strip ANSI codes"
    );
}

// =============================================================================
// Cleanup Command Tests
// =============================================================================

#[test]
fn test_cleanup_sessions() {
    let harness = TestHarness::new();

    harness
        .run(&["cleanup"])
        .success()
        .stdout(predicate::str::contains("No sessions to clean up"));

    // Cleanup calls sessions to list them
    harness.assert_method_called("sessions");
}

// =============================================================================
// Completions Command Tests
// =============================================================================

#[test]
fn test_completions_bash() {
    let harness = TestHarness::new();

    harness
        .run(&["completions", "bash"])
        .success()
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn test_completions_zsh() {
    let harness = TestHarness::new();

    harness
        .run(&["completions", "zsh"])
        .success()
        .stdout(predicate::str::contains("compdef"));
}

// =============================================================================
// Demo Command Tests
// =============================================================================

#[test]
fn test_demo_spawns_internal_tui() {
    let harness = TestHarness::new();

    harness
        .run(&["demo"])
        .success()
        .stdout(predicate::str::contains("Demo started"));

    // Demo calls spawn with the agent-tui binary itself
    harness.assert_method_called("spawn");
}
