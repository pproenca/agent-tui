#[path = "../common/mod.rs"]
mod common;

use common::{MockResponse, TestHarness};
use predicates::prelude::*;
use serde_json::json;

#[test]
fn test_dbl_click_succeeds_stable_element() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "dbl_click",
        json!({
            "success": true,
            "message": null
        }),
    );

    harness
        .run(&["action", "@btn1", "dblclick"])
        .success()
        .stdout(predicate::str::is_empty().not());

    harness.assert_method_called("dbl_click");
}

#[test]
fn test_dbl_click_fails_nonexistent_element() {
    let harness = TestHarness::new();

    harness.set_response(
        "dbl_click",
        MockResponse::StructuredError {
            code: -32003,
            message: "Element not found: @missing".to_string(),
            category: Some("element".to_string()),
            retryable: Some(false),
            context: Some(json!({
                "element_ref": "@missing",
                "session_id": "test-session"
            })),
            suggestion: Some("Use 'screenshot -e' to see available elements".to_string()),
        },
    );

    harness
        .run(&["action", "@missing", "dblclick"])
        .failure()
        .stderr(predicate::str::contains("Element not found"));

    harness.assert_method_called("dbl_click");
}

#[test]
fn test_dbl_click_returns_structured_error() {
    let harness = TestHarness::new();

    harness.set_response(
        "dbl_click",
        MockResponse::StructuredError {
            code: -32003,
            message: "Element '@btn1' disappeared during double-click".to_string(),
            category: Some("element".to_string()),
            retryable: Some(true),
            context: Some(json!({
                "element_ref": "@btn1",
                "session_id": "test-session"
            })),
            suggestion: Some(
                "Element may have been removed. Use 'wait' to ensure stability".to_string(),
            ),
        },
    );

    harness
        .run(&["action", "@btn1", "dblclick"])
        .failure()
        .stderr(predicate::str::contains("Element"))
        .stderr(predicate::str::contains("disappeared"));
}

#[test]
fn test_dbl_click_with_explicit_session() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "dbl_click",
        json!({
            "success": true,
            "message": null
        }),
    );

    harness
        .run(&["-s", "my-session", "action", "@btn1", "dblclick"])
        .success();

    harness.assert_method_called_with(
        "dbl_click",
        json!({
            "ref": "@btn1",
            "session": "my-session"
        }),
    );
}

#[test]
fn test_dbl_click_lock_timeout_is_retryable() {
    let harness = TestHarness::new();

    harness.set_response(
        "dbl_click",
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
        .run(&["action", "@btn1", "dblclick"])
        .failure()
        .stderr(predicate::str::contains("transient"));
}

#[test]
fn test_dbl_click_with_text_ref() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "dbl_click",
        json!({
            "success": true,
            "message": null
        }),
    );

    harness.run(&["action", "Submit", "dblclick"]).success();

    harness.assert_method_called_with("dbl_click", json!({"ref": "Submit"}));
}

#[test]
fn test_dbl_click_succeeds_without_warning() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "dbl_click",
        json!({
            "success": true
        }),
    );

    harness
        .run(&["action", "static-text", "dblclick"])
        .success()
        .stdout(predicate::str::contains("Double-clicked successfully"));
}
