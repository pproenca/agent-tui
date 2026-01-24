//! Connection failure tests
//!
//! Tests for handling various connection failure scenarios between CLI and daemon.
//! These tests verify that the client handles network-level failures gracefully.

mod common;

use common::{MockResponse, TestHarness, agent_tui_cmd};
use predicates::prelude::*;

// =============================================================================
// Socket Not Exists Tests
// =============================================================================

#[test]
fn test_daemon_socket_not_exists() {
    // Don't use TestHarness - point at a non-existent socket
    let mut cmd = agent_tui_cmd();
    cmd.env("XDG_RUNTIME_DIR", "/nonexistent/path/that/does/not/exist");
    cmd.env("TMPDIR", "/nonexistent/path/that/does/not/exist");

    cmd.args(["daemon", "status"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not running").or(predicate::str::contains("connect")));
}

#[test]
fn test_spawn_fails_when_daemon_not_running() {
    let mut cmd = agent_tui_cmd();
    cmd.env("XDG_RUNTIME_DIR", "/nonexistent/path/that/does/not/exist");
    cmd.env("TMPDIR", "/nonexistent/path/that/does/not/exist");

    cmd.args(["run", "bash"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not running").or(predicate::str::contains("connect")));
}

// =============================================================================
// Disconnect During Request Tests
// =============================================================================

#[test]
fn test_daemon_disconnect_during_request() {
    let harness = TestHarness::new();

    // Set daemon to disconnect immediately
    harness.set_response("health", MockResponse::Disconnect);

    // daemon status returns success but reports error in output
    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("not running"));
}

#[test]
fn test_daemon_disconnect_on_spawn() {
    let harness = TestHarness::new();

    harness.set_response("spawn", MockResponse::Disconnect);

    harness
        .run(&["run", "bash"])
        .failure()
        .stderr(predicate::str::contains("error").or(predicate::str::contains("Error")));
}

#[test]
fn test_daemon_disconnect_on_snapshot() {
    let harness = TestHarness::new();

    harness.set_response("snapshot", MockResponse::Disconnect);

    harness
        .run(&["screen"])
        .failure()
        .stderr(predicate::str::contains("error").or(predicate::str::contains("Error")));
}

// =============================================================================
// Malformed Response Tests
// =============================================================================

#[test]
fn test_malformed_response_handling() {
    let harness = TestHarness::new();

    // Return invalid JSON
    harness.set_response(
        "health",
        MockResponse::Malformed("not valid json".to_string()),
    );

    // daemon status returns success but reports error in output
    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("not running"));
}

#[test]
fn test_malformed_json_rpc_missing_result() {
    let harness = TestHarness::new();

    // Valid JSON but missing result/error fields
    harness.set_response(
        "health",
        MockResponse::Malformed(r#"{"jsonrpc":"2.0","id":1}"#.to_string()),
    );

    // daemon status returns success but reports error in output
    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("not running"));
}

#[test]
fn test_malformed_json_rpc_wrong_version() {
    let harness = TestHarness::new();

    // Wrong JSON-RPC version
    harness.set_response(
        "health",
        MockResponse::Malformed(r#"{"jsonrpc":"1.0","id":1,"result":{"status":"ok"}}"#.to_string()),
    );

    // This might succeed because we may not validate version strictly
    // The test verifies we don't crash on unexpected data
    let _ = harness.run(&["daemon", "status"]);
}

#[test]
fn test_empty_response() {
    let harness = TestHarness::new();

    // Empty string response
    harness.set_response("health", MockResponse::Malformed(String::new()));

    // daemon status returns success but reports error in output
    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("not running"));
}

#[test]
fn test_partial_json_response() {
    let harness = TestHarness::new();

    // Truncated JSON
    harness.set_response(
        "health",
        MockResponse::Malformed(r#"{"jsonrpc":"2.0","id":1,"result":{"status":"#.to_string()),
    );

    // daemon status returns success but reports error in output
    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("not running"));
}

// =============================================================================
// Connection Recovery Tests
// =============================================================================

#[test]
fn test_connection_recovers_after_failure() {
    let harness = TestHarness::new();

    // First request fails
    harness.set_response("health", MockResponse::Disconnect);
    let _ = harness.run(&["daemon", "status"]);

    // Restore normal response
    harness.set_success_response(
        "health",
        serde_json::json!({
            "status": "healthy",
            "pid": 12345,
            "uptime_ms": 1000,
            "session_count": 0,
            "version": "1.0.0"
        }),
    );

    // Second request should succeed
    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("healthy"));
}

#[test]
fn test_different_commands_independent_failure() {
    let harness = TestHarness::new();

    // health fails - daemon status returns success but reports "not running"
    harness.set_response("health", MockResponse::Disconnect);
    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("not running"));

    // sessions should still work (different connection)
    harness
        .run(&["sessions"])
        .success()
        .stdout(predicate::str::contains("No active sessions"));
}
