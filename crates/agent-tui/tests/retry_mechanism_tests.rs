//! Retry mechanism tests
//!
//! Tests for the MockDaemon's Sequence and Delayed response types, which enable
//! testing retry scenarios. Also tests timing behavior for delayed responses.

mod common;

use common::{MockResponse, TestHarness, timed};
use predicates::prelude::*;
use serde_json::json;
use std::time::Duration;

// =============================================================================
// Sequence Response Tests
// =============================================================================

#[test]
fn test_sequence_returns_different_responses() {
    let harness = TestHarness::new();

    // First call fails, second succeeds
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
                "version": "1.0.0"
            })),
        ]),
    );

    // First call should fail
    harness
        .run(&["status"])
        .failure()
        .stderr(predicate::str::contains("Temporary error"));

    // Second call should succeed
    harness
        .run(&["status"])
        .success()
        .stdout(predicate::str::contains("healthy"));

    // Verify both calls were made
    assert_eq!(harness.call_count("health"), 2);
}

#[test]
fn test_sequence_cycles_through_responses() {
    let harness = TestHarness::new();

    // Cycle through 3 different responses
    harness.set_response(
        "health",
        MockResponse::Sequence(vec![
            MockResponse::Success(json!({
                "status": "healthy",
                "pid": 1,
                "uptime_ms": 1000,
                "session_count": 0,
                "version": "1.0.0"
            })),
            MockResponse::Success(json!({
                "status": "degraded",
                "pid": 2,
                "uptime_ms": 2000,
                "session_count": 1,
                "version": "1.0.0"
            })),
            MockResponse::Success(json!({
                "status": "healthy",
                "pid": 3,
                "uptime_ms": 3000,
                "session_count": 2,
                "version": "1.0.0"
            })),
        ]),
    );

    // Should cycle back to first response after exhausting sequence
    harness.run(&["status"]).success(); // pid=1
    harness.run(&["status"]).success(); // pid=2
    harness.run(&["status"]).success(); // pid=3
    harness
        .run(&["status"])
        .success()
        .stdout(predicate::str::contains("PID: 1")); // cycles back

    assert_eq!(harness.call_count("health"), 4);
}

#[test]
fn test_sequence_with_disconnect() {
    let harness = TestHarness::new();

    // First call disconnects, second succeeds
    harness.set_response(
        "health",
        MockResponse::Sequence(vec![
            MockResponse::Disconnect,
            MockResponse::Success(json!({
                "status": "healthy",
                "pid": 12345,
                "uptime_ms": 1000,
                "session_count": 0,
                "version": "1.0.0"
            })),
        ]),
    );

    // First call fails due to disconnect
    harness.run(&["status"]).failure();

    // Second call succeeds
    harness.run(&["status"]).success();
}

#[test]
fn test_sequence_with_malformed() {
    let harness = TestHarness::new();

    // First call returns malformed, second succeeds
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

    // First call fails
    harness.run(&["ls"]).failure();

    // Second call succeeds
    harness
        .run(&["ls"])
        .success()
        .stdout(predicate::str::contains("No active sessions"));
}

// =============================================================================
// Delayed Response Tests
// =============================================================================

#[test]
fn test_delayed_response_adds_latency() {
    let harness = TestHarness::new();

    // Delay response by 200ms
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

    let (result, duration) = timed(|| harness.run(&["status"]));

    result.success();

    // Should take at least 200ms
    assert!(
        duration >= Duration::from_millis(150), // Allow some tolerance
        "Expected at least 150ms delay, got {:?}",
        duration
    );
}

#[test]
fn test_delayed_error_response() {
    let harness = TestHarness::new();

    // Delay error response by 100ms
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

    let (result, duration) = timed(|| harness.run(&["status"]));

    result
        .failure()
        .stderr(predicate::str::contains("Delayed error"));

    assert!(
        duration >= Duration::from_millis(80), // Allow tolerance
        "Expected at least 80ms delay, got {:?}",
        duration
    );
}

// =============================================================================
// Simulated Retry Scenarios
// =============================================================================

#[test]
fn test_transient_failure_then_success_pattern() {
    let harness = TestHarness::new();

    // Simulate: lock timeout, lock timeout, success
    harness.set_response(
        "click",
        MockResponse::Sequence(vec![
            MockResponse::Error {
                code: -32007, // LOCK_TIMEOUT
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

    // Without retry logic in CLI, each call is independent
    harness.run(&["click", "@btn1"]).failure();
    harness.run(&["click", "@btn1"]).failure();
    harness
        .run(&["click", "@btn1"])
        .success()
        .stdout(predicate::str::contains("Clicked"));

    assert_eq!(harness.call_count("click"), 3);
}

#[test]
fn test_permanent_failure_pattern() {
    let harness = TestHarness::new();

    // Always fails with element not found (non-retryable)
    harness.set_response(
        "click",
        MockResponse::Error {
            code: -32003, // ELEMENT_NOT_FOUND
            message: "Element not found: @missing".to_string(),
        },
    );

    // Should fail consistently
    harness.run(&["click", "@missing"]).failure();
    harness.run(&["click", "@missing"]).failure();
    harness.run(&["click", "@missing"]).failure();

    assert_eq!(harness.call_count("click"), 3);
}

// =============================================================================
// Call Tracking Tests
// =============================================================================

#[test]
fn test_call_count_for_method() {
    let harness = TestHarness::new();

    harness.run(&["status"]).success();
    harness.run(&["ls"]).success();
    harness.run(&["status"]).success();
    harness.run(&["status"]).success();

    assert_eq!(harness.call_count("health"), 3);
    assert_eq!(harness.call_count("sessions"), 1);
    assert_eq!(harness.call_count("nonexistent"), 0);
}

#[test]
fn test_nth_call_params_tracking() {
    let harness = TestHarness::new();

    harness.run(&["click", "@btn1"]).success();
    harness.run(&["click", "@btn2"]).success();
    harness.run(&["click", "@btn3"]).success();

    let first = harness.last_request_for("click").unwrap();
    // last_request_for returns the last call, not first
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

    harness.run(&["status"]).success();
    harness.run(&["status"]).success();
    assert_eq!(harness.call_count("health"), 2);

    harness.clear_requests();
    assert_eq!(harness.call_count("health"), 0);

    harness.run(&["status"]).success();
    assert_eq!(harness.call_count("health"), 1);
}
