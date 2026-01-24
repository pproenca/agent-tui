//! Response edge case tests
//!
//! Tests for handling unusual or edge case responses from the daemon.

mod common;

use common::{TEST_COLS, TEST_ROWS, TEST_SESSION_ID, TestHarness};
use predicates::prelude::*;
use serde_json::json;

// =============================================================================
// Empty Response Tests
// =============================================================================

#[test]
fn test_empty_result_object() {
    let harness = TestHarness::new();

    harness.set_success_response("health", json!({}));

    // Should handle empty result gracefully
    let _ = harness.run(&["status"]);
}

#[test]
fn test_empty_sessions_array() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "sessions",
        json!({
            "sessions": [],
            "active_session": null
        }),
    );

    harness
        .run(&["ls"])
        .success()
        .stdout(predicate::str::contains("No active sessions"));
}

#[test]
fn test_empty_elements_array() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "screen": "Empty screen\n",
            "elements": [],
            "cursor": { "row": 0, "col": 0, "visible": true },
            "size": { "cols": TEST_COLS, "rows": TEST_ROWS }
        }),
    );

    // With empty elements, snapshot -i just shows the screen without element list
    harness
        .run(&["snap", "-i"])
        .success()
        .stdout(predicate::str::contains("Empty screen"));
}

#[test]
fn test_empty_screen_content() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "screen": "",
            "elements": [],
            "cursor": { "row": 0, "col": 0, "visible": true },
            "size": { "cols": TEST_COLS, "rows": TEST_ROWS }
        }),
    );

    harness.run(&["snap"]).success();
}

// =============================================================================
// Large Response Tests
// =============================================================================

#[test]
fn test_large_screen_response() {
    let harness = TestHarness::new();

    // Generate a 500x200 terminal screen
    let large_screen: String = (0..200)
        .map(|i| format!("{:>500}\n", format!("Line {}", i)))
        .collect();

    harness.set_success_response(
        "snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "screen": large_screen,
            "elements": [],
            "cursor": { "row": 0, "col": 0, "visible": true },
            "size": { "cols": 500, "rows": 200 }
        }),
    );

    harness.run(&["snap"]).success();
}

#[test]
fn test_many_elements_response() {
    let harness = TestHarness::new();

    // Generate 100 elements
    let elements: Vec<serde_json::Value> = (0..100)
        .map(|i| {
            json!({
                "ref": format!("@el{}", i),
                "type": "button",
                "label": format!("Button {}", i),
                "position": { "row": i % 40, "col": (i * 3) % 120 },
                "focused": i == 0,
                "selected": false
            })
        })
        .collect();

    harness.set_success_response(
        "snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "screen": "Test screen\n",
            "elements": elements,
            "cursor": { "row": 0, "col": 0, "visible": true },
            "size": { "cols": TEST_COLS, "rows": TEST_ROWS }
        }),
    );

    harness
        .run(&["snap", "-i"])
        .success()
        .stdout(predicate::str::contains("@el0"))
        .stdout(predicate::str::contains("@el99"));
}

// =============================================================================
// Unicode Content Tests
// =============================================================================

#[test]
fn test_unicode_in_screen_content() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "screen": "Hello 世界! こんにちは 🎉 émojis\n",
            "elements": [],
            "cursor": { "row": 0, "col": 0, "visible": true },
            "size": { "cols": TEST_COLS, "rows": TEST_ROWS }
        }),
    );

    harness
        .run(&["snap"])
        .success()
        .stdout(predicate::str::contains("世界"))
        .stdout(predicate::str::contains("こんにちは"));
}

#[test]
fn test_unicode_in_element_labels() {
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
                    "label": "送信 (Submit)",
                    "position": { "row": 5, "col": 10 },
                    "focused": true,
                    "selected": false
                },
                {
                    "ref": "@tab1",
                    "type": "tab",
                    "label": "🏠 Accueil",
                    "position": { "row": 1, "col": 0 },
                    "focused": false,
                    "selected": true
                }
            ],
            "cursor": { "row": 0, "col": 0, "visible": true },
            "size": { "cols": TEST_COLS, "rows": TEST_ROWS }
        }),
    );

    harness
        .run(&["snap", "-i"])
        .success()
        .stdout(predicate::str::contains("送信"));
}

#[test]
fn test_unicode_in_fill_value() {
    let harness = TestHarness::new();

    harness.run(&["fill", "@inp1", "Здравствуйте"]).success();

    let req = harness.last_request_for("fill").unwrap();
    assert_eq!(
        req.params.as_ref().unwrap()["value"].as_str().unwrap(),
        "Здравствуйте"
    );
}

// =============================================================================
// Warning Field Tests
// =============================================================================

#[test]
fn test_warning_field_in_click_response() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "click",
        json!({
            "success": true,
            "message": null,
            "warning": "Element may be partially obscured"
        }),
    );

    harness
        .run(&["click", "@btn1"])
        .success()
        .stderr(predicate::str::contains("obscured"));
}

#[test]
fn test_warning_field_in_fill_response() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "fill",
        json!({
            "success": true,
            "ref": "@btn1",
            "value": "test",
            "warning": "Warning: '@btn1' is a button not an input field. Fill may not work as expected."
        }),
    );

    harness
        .run(&["fill", "@btn1", "test"])
        .success()
        .stderr(predicate::str::contains("button"));
}

#[test]
fn test_no_warning_when_field_absent() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "click",
        json!({
            "success": true,
            "message": null
        }),
    );

    // No warning in output
    harness
        .run(&["click", "@btn1"])
        .success()
        .stdout(predicate::str::contains("Clicked"));
}

// =============================================================================
// Null Value Tests
// =============================================================================

#[test]
fn test_null_active_session() {
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
                }
            ],
            "active_session": null
        }),
    );

    // Should show sessions without "(active)" marker
    harness
        .run(&["ls"])
        .success()
        .stdout(predicate::str::contains("session-1"));
}

#[test]
fn test_null_cursor_position() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "screen": "Test\n",
            "elements": [],
            "cursor": null,
            "size": { "cols": TEST_COLS, "rows": TEST_ROWS }
        }),
    );

    // Should handle null cursor gracefully
    harness.run(&["snap"]).success();
}

// =============================================================================
// Special Characters Tests
// =============================================================================

#[test]
fn test_newlines_in_screen_preserved() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "screen": "Line 1\nLine 2\nLine 3\n",
            "elements": [],
            "cursor": { "row": 0, "col": 0, "visible": true },
            "size": { "cols": TEST_COLS, "rows": TEST_ROWS }
        }),
    );

    harness
        .run(&["snap"])
        .success()
        .stdout(predicate::str::contains("Line 1"))
        .stdout(predicate::str::contains("Line 2"))
        .stdout(predicate::str::contains("Line 3"));
}

#[test]
fn test_tabs_in_screen_content() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "screen": "Column1\tColumn2\tColumn3\n",
            "elements": [],
            "cursor": { "row": 0, "col": 0, "visible": true },
            "size": { "cols": TEST_COLS, "rows": TEST_ROWS }
        }),
    );

    harness
        .run(&["snap"])
        .success()
        .stdout(predicate::str::contains("Column1"));
}

#[test]
fn test_ansi_escape_sequences_stripped_when_requested() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "screen": "Normal text\n",
            "elements": [],
            "cursor": { "row": 0, "col": 0, "visible": true },
            "size": { "cols": TEST_COLS, "rows": TEST_ROWS }
        }),
    );

    // Request with strip_ansi
    harness.run(&["snap", "--strip-ansi"]).success();

    let req = harness.last_request_for("snapshot").unwrap();
    assert_eq!(req.params.as_ref().unwrap()["strip_ansi"], true);
}

// =============================================================================
// Extra Fields Tests (forward compatibility)
// =============================================================================

#[test]
fn test_unknown_fields_in_response_ignored() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "health",
        json!({
            "status": "healthy",
            "pid": 12345,
            "uptime_ms": 1000,
            "session_count": 0,
            "version": "1.0.0",
            "future_field": "unknown value",
            "another_future_field": { "nested": true }
        }),
    );

    // Should work despite unknown fields
    harness
        .run(&["status"])
        .success()
        .stdout(predicate::str::contains("healthy"));
}

#[test]
fn test_unknown_element_type_handled() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "screen": "Test\n",
            "elements": [
                {
                    "ref": "@unknown1",
                    "type": "future_widget_type",
                    "label": "Unknown Widget",
                    "position": { "row": 0, "col": 0 },
                    "focused": false,
                    "selected": false
                }
            ],
            "cursor": { "row": 0, "col": 0, "visible": true },
            "size": { "cols": TEST_COLS, "rows": TEST_ROWS }
        }),
    );

    // Should display unknown type without crashing
    harness
        .run(&["snap", "-i"])
        .success()
        .stdout(predicate::str::contains("@unknown1"));
}

// =============================================================================
// JSON Output Mode Tests
// =============================================================================

#[test]
fn test_json_output_preserves_structure() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "screen": "Test\n",
            "elements": [
                {
                    "ref": "@btn1",
                    "type": "button",
                    "label": "OK"
                }
            ],
            "cursor": { "row": 5, "col": 10, "visible": true },
            "size": { "cols": TEST_COLS, "rows": TEST_ROWS }
        }),
    );

    harness
        .run(&["-f", "json", "snap"])
        .success()
        .stdout(predicate::str::contains("\"session_id\""))
        .stdout(predicate::str::contains("\"elements\""))
        .stdout(predicate::str::contains("\"cursor\""));
}

#[test]
fn test_json_output_contains_valid_json() {
    let harness = TestHarness::new();

    let output = harness.run(&["-f", "json", "status"]);
    let stdout = output.get_output().stdout.clone();
    let stdout_str = String::from_utf8_lossy(&stdout);

    // Should be valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout_str);
    assert!(
        parsed.is_ok(),
        "Output should be valid JSON: {}",
        stdout_str
    );
}
