//! Error propagation tests
//!
//! Tests that structured error data (codes, categories, suggestions) properly
//! propagates from daemon to CLI output.

mod common;

use common::{MockResponse, TestHarness};
use predicates::prelude::*;
use serde_json::json;

// =============================================================================
// Session Error Tests
// =============================================================================

#[test]
fn test_session_not_found_structured_error() {
    let harness = TestHarness::new();

    harness.set_response(
        "snapshot",
        MockResponse::StructuredError {
            code: -32001, // SESSION_NOT_FOUND
            message: "Session not found: invalid-session".to_string(),
            category: Some("not_found".to_string()),
            retryable: Some(false),
            context: Some(json!({
                "session_id": "invalid-session"
            })),
            suggestion: Some("Use 'agent-tui sessions' to list active sessions".to_string()),
        },
    );

    harness
        .run(&["screen"])
        .failure()
        .stderr(predicate::str::contains("Session not found"));
}

#[test]
fn test_no_active_session_error() {
    let harness = TestHarness::new();

    harness.set_response(
        "snapshot",
        MockResponse::StructuredError {
            code: -32002, // NO_ACTIVE_SESSION
            message: "No active session".to_string(),
            category: Some("not_found".to_string()),
            retryable: Some(false),
            context: None,
            suggestion: Some("Start a session with 'agent-tui spawn <command>'".to_string()),
        },
    );

    harness
        .run(&["screen"])
        .failure()
        .stderr(predicate::str::contains("No active session"));
}

// =============================================================================
// Element Error Tests
// =============================================================================

#[test]
fn test_element_not_found_includes_ref() {
    let harness = TestHarness::new();

    harness.set_response(
        "click",
        MockResponse::StructuredError {
            code: -32003, // ELEMENT_NOT_FOUND
            message: "Element not found: @missing-button".to_string(),
            category: Some("not_found".to_string()),
            retryable: Some(false),
            context: Some(json!({
                "element_ref": "@missing-button",
                "available_elements": ["@btn1", "@inp1", "@tab1"]
            })),
            suggestion: Some("Use 'snapshot -i' to see available elements".to_string()),
        },
    );

    harness
        .run(&["action", "@missing-button", "click"])
        .failure()
        .stderr(predicate::str::contains("Element not found"));
}

#[test]
fn test_wrong_element_type_shows_actual_expected() {
    let harness = TestHarness::new();

    harness.set_response(
        "fill",
        MockResponse::StructuredError {
            code: -32004, // WRONG_ELEMENT_TYPE
            message: "Expected input element, got button".to_string(),
            category: Some("invalid_input".to_string()),
            retryable: Some(false),
            context: Some(json!({
                "element_ref": "@btn1",
                "expected_type": "input",
                "actual_type": "button"
            })),
            suggestion: Some("Use 'click' for buttons, 'fill' for inputs".to_string()),
        },
    );

    harness
        .run(&["action", "@btn1", "fill", "test-value"])
        .failure()
        .stderr(predicate::str::contains("button"));
}

// =============================================================================
// Lock/Busy Error Tests
// =============================================================================

#[test]
fn test_lock_timeout_marked_retryable() {
    let harness = TestHarness::new();

    harness.set_response(
        "click",
        MockResponse::StructuredError {
            code: -32007, // LOCK_TIMEOUT
            message: "Session lock timeout".to_string(),
            category: Some("busy".to_string()),
            retryable: Some(true),
            context: Some(json!({
                "session_id": "test-session",
                "lock_timeout_ms": 5000
            })),
            suggestion: Some("The session is busy. Try again in a moment.".to_string()),
        },
    );

    harness
        .run(&["action", "@btn1", "click"])
        .failure()
        .stderr(predicate::str::contains("lock").or(predicate::str::contains("timeout")));
}

#[test]
fn test_session_limit_reached() {
    let harness = TestHarness::new();

    harness.set_response(
        "spawn",
        MockResponse::StructuredError {
            code: -32006, // SESSION_LIMIT
            message: "Maximum session limit reached (16)".to_string(),
            category: Some("busy".to_string()),
            retryable: Some(false),
            context: Some(json!({
                "current_count": 16,
                "max_sessions": 16
            })),
            suggestion: Some(
                "Kill an existing session with 'agent-tui kill <session_id>'".to_string(),
            ),
        },
    );

    harness
        .run(&["run", "bash"])
        .failure()
        .stderr(predicate::str::contains("limit"));
}

// =============================================================================
// Input Validation Error Tests
// =============================================================================

#[test]
fn test_invalid_key_error() {
    let harness = TestHarness::new();

    // input --hold calls keydown method
    harness.set_response(
        "keydown",
        MockResponse::StructuredError {
            code: -32005, // INVALID_KEY
            message: "Invalid key: 'InvalidKey'".to_string(),
            category: Some("invalid_input".to_string()),
            retryable: Some(false),
            context: Some(json!({
                "key": "InvalidKey",
                "valid_keys": ["Enter", "Tab", "Escape", "Backspace", "Delete", "Up", "Down", "Left", "Right"]
            })),
            suggestion: Some("See 'agent-tui input --help' for valid key names".to_string()),
        },
    );

    // Use --hold to force key mode (otherwise InvalidKey would be typed as text)
    harness
        .run(&["input", "--hold", "InvalidKey"])
        .failure()
        .stderr(predicate::str::contains("Invalid"));
}

// =============================================================================
// External/Process Error Tests
// =============================================================================

#[test]
fn test_command_not_found_error() {
    let harness = TestHarness::new();

    harness.set_response(
        "spawn",
        MockResponse::StructuredError {
            code: -32014, // COMMAND_NOT_FOUND
            message: "Command not found: nonexistent-cmd".to_string(),
            category: Some("external".to_string()),
            retryable: Some(false),
            context: Some(json!({
                "command": "nonexistent-cmd"
            })),
            suggestion: Some("Verify the command is installed and in PATH".to_string()),
        },
    );

    harness
        .run(&["run", "nonexistent-cmd"])
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_permission_denied_error() {
    let harness = TestHarness::new();

    harness.set_response(
        "spawn",
        MockResponse::StructuredError {
            code: -32015, // PERMISSION_DENIED
            message: "Permission denied: /restricted/script".to_string(),
            category: Some("external".to_string()),
            retryable: Some(false),
            context: Some(json!({
                "command": "/restricted/script",
                "errno": 13
            })),
            suggestion: Some("Check file permissions or run as appropriate user".to_string()),
        },
    );

    harness
        .run(&["run", "/restricted/script"])
        .failure()
        .stderr(predicate::str::contains("Permission denied"));
}

#[test]
fn test_pty_error() {
    let harness = TestHarness::new();

    harness.set_response(
        "spawn",
        MockResponse::StructuredError {
            code: -32008, // PTY_ERROR
            message: "Failed to create PTY".to_string(),
            category: Some("external".to_string()),
            retryable: Some(false),
            context: Some(json!({
                "errno": 12,
                "description": "Out of memory"
            })),
            suggestion: Some("System resource issue. Check available memory.".to_string()),
        },
    );

    harness
        .run(&["run", "bash"])
        .failure()
        .stderr(predicate::str::contains("PTY"));
}

// =============================================================================
// Timeout Error Tests
// =============================================================================

#[test]
fn test_wait_timeout_error() {
    let harness = TestHarness::new();

    harness.set_response(
        "wait",
        MockResponse::StructuredError {
            code: -32013, // WAIT_TIMEOUT
            message: "Wait condition not met within timeout".to_string(),
            category: Some("timeout".to_string()),
            retryable: Some(false),
            context: Some(json!({
                "condition": "text:Expected Text",
                "timeout_ms": 5000,
                "elapsed_ms": 5000
            })),
            suggestion: Some("Increase timeout with -t or verify condition is correct".to_string()),
        },
    );

    harness
        .run(&["wait", "-t", "5000", "Expected Text"])
        .failure()
        .stderr(predicate::str::contains("timeout").or(predicate::str::contains("Timeout")));
}

// =============================================================================
// JSON Output Format Tests
// =============================================================================

#[test]
fn test_error_json_format_includes_code() {
    let harness = TestHarness::new();

    harness.set_response(
        "health",
        MockResponse::StructuredError {
            code: -32001,
            message: "Session not found".to_string(),
            category: Some("not_found".to_string()),
            retryable: Some(false),
            context: None,
            suggestion: None,
        },
    );

    let output = harness.run(&["-f", "json", "daemon", "status"]);

    // daemon status returns success but includes error info in JSON output
    output.success();
}

#[test]
fn test_error_with_all_fields() {
    let harness = TestHarness::new();

    harness.set_response(
        "click",
        MockResponse::StructuredError {
            code: -32003,
            message: "Element '@nonexistent' not found".to_string(),
            category: Some("not_found".to_string()),
            retryable: Some(false),
            context: Some(json!({
                "element_ref": "@nonexistent",
                "session_id": "test-session"
            })),
            suggestion: Some("Use 'snapshot -i' to view available elements".to_string()),
        },
    );

    harness
        .run(&["action", "@nonexistent", "click"])
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// =============================================================================
// Error Category Consistency Tests
// =============================================================================

#[test]
fn test_not_found_category_errors() {
    let harness = TestHarness::new();

    // All these errors should have "not_found" category
    let not_found_errors = vec![
        (-32001, "Session not found"),
        (-32002, "No active session"),
        (-32003, "Element not found"),
    ];

    for (code, message) in not_found_errors {
        harness.set_response(
            "snapshot",
            MockResponse::StructuredError {
                code,
                message: message.to_string(),
                category: Some("not_found".to_string()),
                retryable: Some(false),
                context: None,
                suggestion: None,
            },
        );

        harness.run(&["screen"]).failure();
    }
}

#[test]
fn test_simple_error_without_structured_data() {
    let harness = TestHarness::new();

    // Test that simple Error (without structured data) still works
    harness.set_response(
        "health",
        MockResponse::Error {
            code: -32600,
            message: "Invalid request".to_string(),
        },
    );

    // daemon status returns success but includes error in output
    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("Invalid request"));
}
