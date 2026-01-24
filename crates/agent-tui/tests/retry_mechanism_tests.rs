mod common;

use common::{MockResponse, TestHarness, timed};
use predicates::prelude::*;
use serde_json::json;
use std::time::Duration;

#[test]
fn test_sequence_returns_different_responses() {
    let harness = TestHarness::new();

    harness.set_response(
        "health",
        MockResponse::Sequence(vec![
            MockResponse::Error {
                code: -32000,
                message: "Temporary error".to_string(),
            },
            MockResponse::Success(json!({
                "status": "healthy",
                "pid": 12345,
                "uptime_ms": 1000,
                "session_count": 0,
                "version": env!("CARGO_PKG_VERSION")
            })),
        ]),
    );

    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("Temporary error"));

    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("healthy"));

    assert_eq!(harness.call_count("health"), 2);
}

#[test]
fn test_sequence_cycles_through_responses() {
    let harness = TestHarness::new();

    let cli_version = env!("CARGO_PKG_VERSION");

    harness.set_response(
        "health",
        MockResponse::Sequence(vec![
            MockResponse::Success(json!({"status": "healthy", "pid": 1, "uptime_ms": 1000, "session_count": 0, "version": cli_version})),
            MockResponse::Success(json!({"status": "degraded", "pid": 2, "uptime_ms": 2000, "session_count": 1, "version": cli_version})),
            MockResponse::Success(json!({"status": "healthy", "pid": 3, "uptime_ms": 3000, "session_count": 2, "version": cli_version})),
            MockResponse::Success(json!({"status": "healthy", "pid": 1, "uptime_ms": 1000, "session_count": 0, "version": cli_version})),
        ]),
    );

    harness.run(&["daemon", "status"]).success();
    harness.run(&["daemon", "status"]).success();
    harness.run(&["daemon", "status"]).success();
    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("PID: 1"));

    assert_eq!(harness.call_count("health"), 4);
}

#[test]
fn test_sequence_with_disconnect() {
    let harness = TestHarness::new();

    let cli_version = env!("CARGO_PKG_VERSION");

    harness.set_response(
        "health",
        MockResponse::Sequence(vec![
            MockResponse::Disconnect,
            MockResponse::Success(json!({
                "status": "healthy",
                "pid": 12345,
                "uptime_ms": 1000,
                "session_count": 0,
                "version": cli_version
            })),
        ]),
    );

    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("not running"));

    harness.run(&["daemon", "status"]).success();
}

#[test]
fn test_sequence_with_malformed() {
    let harness = TestHarness::new();

    harness.set_response(
        "sessions",
        MockResponse::Sequence(vec![
            MockResponse::Malformed("not json".to_string()),
            MockResponse::Success(json!({
                "sessions": [],
                "active_session": null
            })),
        ]),
    );

    harness.run(&["sessions"]).failure();

    harness
        .run(&["sessions"])
        .success()
        .stdout(predicate::str::contains("No active sessions"));
}

#[test]
fn test_delayed_response_adds_latency() {
    let harness = TestHarness::new();

    harness.set_response(
        "health",
        MockResponse::Delayed(
            Duration::from_millis(200),
            Box::new(MockResponse::Success(json!({
                "status": "healthy",
                "pid": 12345,
                "uptime_ms": 1000,
                "session_count": 0,
                "version": "1.0.0"
            }))),
        ),
    );

    let (result, duration) = timed(|| harness.run(&["daemon", "status"]));

    result.success();

    assert!(
        duration >= Duration::from_millis(150),
        "Expected at least 150ms delay, got {:?}",
        duration
    );
}

#[test]
fn test_delayed_error_response() {
    let harness = TestHarness::new();

    harness.set_response(
        "health",
        MockResponse::Delayed(
            Duration::from_millis(100),
            Box::new(MockResponse::Error {
                code: -32000,
                message: "Delayed error".to_string(),
            }),
        ),
    );

    let (result, duration) = timed(|| harness.run(&["daemon", "status"]));

    result
        .success()
        .stdout(predicate::str::contains("Delayed error"));

    assert!(
        duration >= Duration::from_millis(80),
        "Expected at least 80ms delay, got {:?}",
        duration
    );
}

#[test]
fn test_transient_failure_then_success_pattern() {
    let harness = TestHarness::new();

    harness.set_response(
        "click",
        MockResponse::Sequence(vec![
            MockResponse::Error {
                code: -32007,
                message: "Session locked".to_string(),
            },
            MockResponse::Error {
                code: -32007,
                message: "Session locked".to_string(),
            },
            MockResponse::Success(json!({
                "success": true,
                "message": null
            })),
        ]),
    );

    harness.run(&["action", "@btn1", "click"]).failure();
    harness.run(&["action", "@btn1", "click"]).failure();
    harness
        .run(&["action", "@btn1", "click"])
        .success()
        .stdout(predicate::str::contains("Clicked"));

    assert_eq!(harness.call_count("click"), 3);
}

#[test]
fn test_permanent_failure_pattern() {
    let harness = TestHarness::new();

    harness.set_response(
        "click",
        MockResponse::Error {
            code: -32003,
            message: "Element not found: @missing".to_string(),
        },
    );

    harness.run(&["action", "@missing", "click"]).failure();
    harness.run(&["action", "@missing", "click"]).failure();
    harness.run(&["action", "@missing", "click"]).failure();

    assert_eq!(harness.call_count("click"), 3);
}

#[test]
fn test_call_count_for_method() {
    let harness = TestHarness::new();

    harness.run(&["daemon", "status"]).success();
    harness.run(&["sessions"]).success();
    harness.run(&["daemon", "status"]).success();
    harness.run(&["daemon", "status"]).success();

    assert_eq!(harness.call_count("health"), 4);
    assert_eq!(harness.call_count("sessions"), 1);
    assert_eq!(harness.call_count("nonexistent"), 0);
}

#[test]
fn test_nth_call_params_tracking() {
    let harness = TestHarness::new();

    harness.run(&["action", "@btn1", "click"]).success();
    harness.run(&["action", "@btn2", "click"]).success();
    harness.run(&["action", "@btn3", "click"]).success();

    let first = harness.last_request_for("click").unwrap();

    assert!(
        first.params.as_ref().unwrap()["ref"]
            .as_str()
            .unwrap()
            .contains("btn3")
    );
}

#[test]
fn test_clear_requests_resets_tracking() {
    let harness = TestHarness::new();

    harness.run(&["daemon", "status"]).success();
    harness.run(&["daemon", "status"]).success();
    assert_eq!(harness.call_count("health"), 2);

    harness.clear_requests();
    assert_eq!(harness.call_count("health"), 0);

    harness.run(&["daemon", "status"]).success();
    assert_eq!(harness.call_count("health"), 1);
}
