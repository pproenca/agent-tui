mod common;

use common::{MockResponse, TestHarness, agent_tui_cmd};
use predicates::prelude::*;

#[test]
fn test_daemon_socket_not_exists() {
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

#[test]
fn test_daemon_disconnect_during_request() {
    let harness = TestHarness::new();

    harness.set_response("health", MockResponse::Disconnect);

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

#[test]
fn test_malformed_response_handling() {
    let harness = TestHarness::new();

    harness.set_response(
        "health",
        MockResponse::Malformed("not valid json".to_string()),
    );

    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("not running"));
}

#[test]
fn test_malformed_json_rpc_missing_result() {
    let harness = TestHarness::new();

    harness.set_response(
        "health",
        MockResponse::Malformed(r#"{"jsonrpc":"2.0","id":1}"#.to_string()),
    );

    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("not running"));
}

#[test]
fn test_malformed_json_rpc_wrong_version() {
    let harness = TestHarness::new();

    harness.set_response(
        "health",
        MockResponse::Malformed(r#"{"jsonrpc":"1.0","id":1,"result":{"status":"ok"}}"#.to_string()),
    );

    let _ = harness.run(&["daemon", "status"]);
}

#[test]
fn test_empty_response() {
    let harness = TestHarness::new();

    harness.set_response("health", MockResponse::Malformed(String::new()));

    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("not running"));
}

#[test]
fn test_partial_json_response() {
    let harness = TestHarness::new();

    harness.set_response(
        "health",
        MockResponse::Malformed(r#"{"jsonrpc":"2.0","id":1,"result":{"status":"#.to_string()),
    );

    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("not running"));
}

#[test]
fn test_connection_recovers_after_failure() {
    let harness = TestHarness::new();

    harness.set_response("health", MockResponse::Disconnect);
    let _ = harness.run(&["daemon", "status"]);

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

    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("healthy"));
}

#[test]
fn test_different_commands_independent_failure() {
    let harness = TestHarness::new();

    harness.set_response("health", MockResponse::Disconnect);
    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("not running"));

    harness
        .run(&["sessions"])
        .success()
        .stdout(predicate::str::contains("No active sessions"));
}
