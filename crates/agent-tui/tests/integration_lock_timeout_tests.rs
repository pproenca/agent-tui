mod common;

use common::{MockResponse, TestHarness};
use predicates::prelude::*;
use serde_json::json;

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

    harness
        .run(&["action", "@btn1", "click"])
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

    harness
        .run(&["-f", "json", "screenshot"])
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

    harness
        .run(&["action", "@inp1", "fill", "test"])
        .failure()
        .stderr(predicate::str::contains("lock timeout"));
}

#[test]
fn test_lock_timeout_different_operations() {
    let harness = TestHarness::new();

    let operations = vec!["click", "type", "fill", "screenshot"];

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

    harness
        .run(&["action", "@btn1", "click"])
        .failure()
        .stderr(predicate::str::contains("lock timeout"));

    harness
        .run(&["input", "hello"])
        .failure()
        .stderr(predicate::str::contains("lock timeout"));
}

#[test]
fn test_client_retries_on_lock_timeout() {
    let harness = TestHarness::new();

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

    harness
        .run(&["action", "@btn1", "click"])
        .failure()
        .stderr(predicate::str::contains("lock timeout"));

    // A subsequent attempt should succeed once the lock clears (second response in sequence).
    harness
        .run(&["action", "@btn1", "click"])
        .success()
        .stdout(predicate::str::contains("Clicked").or(predicate::str::contains("success")));
}

#[test]
fn test_persistent_lock_timeout() {
    let harness = TestHarness::new();

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

    for _ in 0..3 {
        harness
            .run(&["screenshot"])
            .failure()
            .stderr(predicate::str::contains("lock timeout"));
    }
}

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
        .run(&["action", "@btn1", "click"])
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

    harness
        .run(&["-f", "json", "action", "@btn1", "click"])
        .failure()
        .stderr(predicate::str::contains("-32006"))
        .stderr(predicate::str::contains("lock timeout"));
}
