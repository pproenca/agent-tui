mod common;

use common::{MockResponse, TestHarness};
use predicates::prelude::*;
use serde_json::json;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// LSB exit code 3: program is not running
const EXIT_NOT_RUNNING: i32 = 3;

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
                "version": env!("AGENT_TUI_VERSION")
            })),
        ]),
    );

    // First call returns error, which means daemon status shows "not running"
    harness
        .run(&["daemon", "status"])
        .code(EXIT_NOT_RUNNING)
        .stdout(predicate::str::contains("not running"));

    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("healthy"));
}

#[test]
fn test_sequence_cycles_through_responses() {
    let harness = TestHarness::new();

    let cli_version = env!("AGENT_TUI_VERSION");

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
}

#[test]
fn test_sequence_with_disconnect() {
    let harness = TestHarness::new();

    let cli_version = env!("AGENT_TUI_VERSION");

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

    // First call gets disconnect, which shows "not running" with exit code 3
    harness
        .run(&["daemon", "status"])
        .code(EXIT_NOT_RUNNING)
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
    let harness = Arc::new(TestHarness::new());

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

    let handle = {
        let h = Arc::clone(&harness);
        thread::spawn(move || h.run(&["daemon", "status"]))
    };

    harness.wait_for_request_count(1);
    harness.wait_for_pending_delays(1);
    harness.advance_time(Duration::from_millis(150));
    assert!(
        !handle.is_finished(),
        "Expected command to wait for delayed response"
    );
    harness.advance_time(Duration::from_millis(100));

    handle.join().expect("Thread panicked").success();
}

#[test]
fn test_delayed_error_response() {
    let harness = Arc::new(TestHarness::new());

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

    let handle = {
        let h = Arc::clone(&harness);
        thread::spawn(move || h.run(&["daemon", "status"]))
    };

    harness.wait_for_request_count(1);
    harness.wait_for_pending_delays(1);
    harness.advance_time(Duration::from_millis(80));
    assert!(
        !handle.is_finished(),
        "Expected command to wait for delayed error response"
    );
    harness.advance_time(Duration::from_millis(30));

    // Error from health check means daemon status shows "not running"
    handle
        .join()
        .expect("Thread panicked")
        .code(EXIT_NOT_RUNNING)
        .stdout(predicate::str::contains("not running"));
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
}
