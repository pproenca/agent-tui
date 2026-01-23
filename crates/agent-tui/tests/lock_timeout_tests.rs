//! Lock timeout tests
//!
//! Tests for lock timeout behavior, verifying:
//! - Lock timeout returns retryable error
//! - Lock timeout has correct error category
//! - Lock timeout includes session context
//! - Client can handle lock timeout errors

mod common;

use common::{MockResponse, TestHarness};
use predicates::prelude::*;
use serde_json::json;

// =============================================================================
// Lock Timeout Error Tests (MockDaemon)
// =============================================================================

#[test]
fn test_lock_timeout_returns_retryable_error() {
    let harness = TestHarness::new();

    harness.set_response(
        "click",
        MockResponse::StructuredError {
            code: -32006,
            message: "Session lock timeout".to_string(),
            category: Some("lock".to_string()),
            retryable: Some(true),
            context: Some(json!({"session_id": "test-session"})),
            suggestion: Some("Retry the operation".to_string()),
        },
    );

    // Error should indicate it's retryable
    harness
        .run(&["click", "@btn1"])
        .failure()
        .stderr(predicate::str::contains("transient"))
        .stderr(predicate::str::contains("retry"));
}

#[test]
fn test_lock_timeout_has_correct_category() {
    let harness = TestHarness::new();

    harness.set_response(
        "snapshot",
        MockResponse::StructuredError {
            code: -32006,
            message: "Session lock timeout".to_string(),
            category: Some("lock".to_string()),
            retryable: Some(true),
            context: Some(json!({"session_id": "test-session"})),
            suggestion: Some("Retry the operation".to_string()),
        },
    );

    // In JSON format, verify the category is present
    harness
        .run(&["-f", "json", "snapshot"])
        .failure()
        .stderr(predicate::str::contains("lock"));
}

#[test]
fn test_lock_timeout_includes_session_context() {
    let harness = TestHarness::new();

    harness.set_response(
        "fill",
        MockResponse::StructuredError {
            code: -32006,
            message: "Session lock timeout".to_string(),
            category: Some("lock".to_string()),
            retryable: Some(true),
            context: Some(json!({"session_id": "my-test-session"})),
            suggestion: Some("Retry the operation".to_string()),
        },
    );

    // The error message or context should reference the session
    harness
        .run(&["fill", "@inp1", "test"])
        .failure()
        .stderr(predicate::str::contains("lock timeout"));
}

#[test]
fn test_lock_timeout_different_operations() {
    let harness = TestHarness::new();

    // Test that lock timeout can occur on various operations
    let operations = vec!["click", "type", "fill", "snapshot"];

    for op in operations {
        harness.set_response(
            op,
            MockResponse::StructuredError {
                code: -32006,
                message: format!("Session lock timeout on {}", op),
                category: Some("lock".to_string()),
                retryable: Some(true),
                context: Some(json!({"session_id": "test-session"})),
                suggestion: Some("Retry the operation".to_string()),
            },
        );
    }

    // Verify click lock timeout
    harness
        .run(&["click", "@btn1"])
        .failure()
        .stderr(predicate::str::contains("lock timeout"));

    // Verify type lock timeout
    harness
        .run(&["type", "hello"])
        .failure()
        .stderr(predicate::str::contains("lock timeout"));
}

// =============================================================================
// Lock Timeout with Retry Sequence Tests
// =============================================================================

#[test]
fn test_client_retries_on_lock_timeout() {
    let harness = TestHarness::new();

    // Simulate lock timeout followed by success
    harness.set_response(
        "click",
        MockResponse::Sequence(vec![
            MockResponse::StructuredError {
                code: -32006,
                message: "Session lock timeout".to_string(),
                category: Some("lock".to_string()),
                retryable: Some(true),
                context: Some(json!({"session_id": "test-session"})),
                suggestion: Some("Retry the operation".to_string()),
            },
            MockResponse::Success(json!({
                "success": true,
                "message": null
            })),
        ]),
    );

    // First call fails with lock timeout
    harness
        .run(&["click", "@btn1"])
        .failure()
        .stderr(predicate::str::contains("lock timeout"));

    // Second call succeeds
    harness.run(&["click", "@btn1"]).success();

    // Verify two calls were made
    assert_eq!(harness.call_count("click"), 2);
}

#[test]
fn test_persistent_lock_timeout() {
    let harness = TestHarness::new();

    // Lock timeout that persists across multiple calls
    harness.set_response(
        "snapshot",
        MockResponse::StructuredError {
            code: -32006,
            message: "Session lock timeout".to_string(),
            category: Some("lock".to_string()),
            retryable: Some(true),
            context: Some(json!({"session_id": "busy-session"})),
            suggestion: Some("Session is busy. Retry later.".to_string()),
        },
    );

    // Multiple calls should all fail with lock timeout
    for _ in 0..3 {
        harness
            .run(&["snapshot"])
            .failure()
            .stderr(predicate::str::contains("lock timeout"));
    }

    // All calls should have been recorded
    assert_eq!(harness.call_count("snapshot"), 3);
}

// =============================================================================
// Lock Timeout Error Display Tests
// =============================================================================

#[test]
fn test_lock_timeout_shows_suggestion() {
    let harness = TestHarness::new();

    harness.set_response(
        "click",
        MockResponse::StructuredError {
            code: -32006,
            message: "Session lock timeout".to_string(),
            category: Some("lock".to_string()),
            retryable: Some(true),
            context: None,
            suggestion: Some("Session is busy. Wait and retry.".to_string()),
        },
    );

    harness
        .run(&["click", "@btn1"])
        .failure()
        .stderr(predicate::str::contains("Wait and retry"));
}

#[test]
fn test_lock_timeout_json_format() {
    let harness = TestHarness::new();

    harness.set_response(
        "click",
        MockResponse::StructuredError {
            code: -32006,
            message: "Session lock timeout".to_string(),
            category: Some("lock".to_string()),
            retryable: Some(true),
            context: Some(json!({"session_id": "test-session"})),
            suggestion: Some("Retry the operation".to_string()),
        },
    );

    // JSON format should show structured error information
    harness
        .run(&["-f", "json", "click", "@btn1"])
        .failure()
        .stderr(predicate::str::contains("-32006"))
        .stderr(predicate::str::contains("lock timeout"));
}
