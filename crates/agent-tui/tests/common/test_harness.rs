//! Test harness for E2E tests with mock daemon
//!
//! Provides a synchronous interface to the async MockDaemon,
//! bridging async infrastructure with sync `#[test]` functions.

#![allow(dead_code)]

use super::mock_daemon::{MockDaemon, MockResponse, RecordedRequest};
use assert_cmd::Command;
use serde_json::Value;
use tokio::runtime::Runtime;

/// Test harness that manages a mock daemon for E2E CLI tests.
///
/// The harness wraps an async tokio runtime and MockDaemon, providing
/// a synchronous API for use in standard `#[test]` functions.
///
/// ## Example
///
/// ```ignore
/// #[test]
/// fn test_health_command() {
///     let harness = TestHarness::new();
///     harness.run(&["status"])
///         .success()
///         .stdout(predicate::str::contains("healthy"));
/// }
/// ```
pub struct TestHarness {
    daemon: MockDaemon,
    runtime: Runtime,
}

impl TestHarness {
    /// Create a new test harness with a running mock daemon.
    ///
    /// This starts a tokio runtime and initializes the mock daemon,
    /// which listens on a temporary Unix socket.
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        let daemon = runtime.block_on(MockDaemon::start());
        Self { daemon, runtime }
    }

    /// Get a Command configured to run agent-tui with environment
    /// variables pointing to the mock daemon.
    pub fn cli_command(&self) -> Command {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("agent-tui"));
        for (key, value) in self.daemon.env_vars() {
            cmd.env(key, value);
        }
        cmd
    }

    /// Run the CLI with the given arguments and return the assertion.
    ///
    /// This is a convenience method that combines `cli_command()` with
    /// argument passing and assertion.
    pub fn run(&self, args: &[&str]) -> assert_cmd::assert::Assert {
        self.cli_command().args(args).assert()
    }

    /// Get all requests recorded by the mock daemon.
    pub fn get_requests(&self) -> Vec<RecordedRequest> {
        self.daemon.get_requests()
    }

    /// Get the last request for a specific method.
    pub fn last_request_for(&self, method: &str) -> Option<RecordedRequest> {
        self.daemon.last_request_for(method)
    }

    /// Set a custom response for a specific method.
    ///
    /// This allows tests to configure error responses, specific data,
    /// or edge cases like disconnections and timeouts.
    pub fn set_response(&self, method: &str, response: MockResponse) {
        self.daemon.set_response(method, response);
    }

    /// Clear all recorded requests.
    ///
    /// Useful when testing multiple commands in sequence and you want
    /// to verify only the most recent requests.
    pub fn clear_requests(&self) {
        self.daemon.clear_requests();
    }

    /// Set a successful JSON response for a method.
    ///
    /// Convenience method for the common case of returning success data.
    pub fn set_success_response(&self, method: &str, result: Value) {
        self.set_response(method, MockResponse::Success(result));
    }

    /// Set an error response for a method.
    ///
    /// Convenience method for testing error handling.
    pub fn set_error_response(&self, method: &str, code: i32, message: &str) {
        self.set_response(
            method,
            MockResponse::Error {
                code,
                message: message.to_string(),
            },
        );
    }

    /// Verify that a specific method was called.
    pub fn assert_method_called(&self, method: &str) {
        let requests = self.get_requests();
        assert!(
            requests.iter().any(|r| r.method == method),
            "Expected method '{}' to be called, but it wasn't. Calls: {:?}",
            method,
            requests.iter().map(|r| &r.method).collect::<Vec<_>>()
        );
    }

    /// Verify that a specific method was called with expected params.
    pub fn assert_method_called_with(&self, method: &str, expected_params: Value) {
        let request = self
            .last_request_for(method)
            .unwrap_or_else(|| panic!("Expected method '{}' to be called", method));

        let actual_params = request.params.clone().unwrap_or(Value::Null);

        // For object params, check that expected fields are present and match
        if expected_params.is_object() && actual_params.is_object() {
            let expected_obj = expected_params.as_object().unwrap();
            let actual_obj = actual_params.as_object().unwrap();

            for (key, expected_value) in expected_obj {
                let actual_value = actual_obj.get(key).unwrap_or_else(|| {
                    panic!(
                        "Expected param '{}' not found in request. Actual params: {:?}",
                        key, actual_params
                    )
                });
                assert_eq!(
                    actual_value, expected_value,
                    "Param '{}' mismatch. Expected: {:?}, Actual: {:?}",
                    key, expected_value, actual_value
                );
            }
        } else {
            assert_eq!(
                actual_params, expected_params,
                "Params mismatch for method '{}'",
                method
            );
        }
    }

    /// Get the number of times a method was called.
    pub fn call_count(&self, method: &str) -> usize {
        self.get_requests()
            .iter()
            .filter(|r| r.method == method)
            .count()
    }
}

impl Default for TestHarness {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use predicates::prelude::*;

    #[test]
    fn test_harness_runs_cli() {
        let harness = TestHarness::new();
        harness
            .run(&["--help"])
            .success()
            .stdout(predicate::str::contains("agent-tui"));
    }

    #[test]
    fn test_harness_records_requests() {
        let harness = TestHarness::new();

        // Run sessions --status command which should contact the daemon
        let _ = harness.run(&["sessions", "--status"]);

        // Verify the health method was called
        let requests = harness.get_requests();
        assert!(
            requests.iter().any(|r| r.method == "health"),
            "health method should have been called"
        );
    }

    #[test]
    fn test_harness_custom_response() {
        let harness = TestHarness::new();

        // Set custom health response
        harness.set_success_response(
            "health",
            serde_json::json!({
                "status": "degraded",
                "pid": 99999,
                "uptime_ms": 1000,
                "session_count": 5,
                "version": "2.0.0-custom"
            }),
        );

        // Run sessions --status command which calls health
        harness
            .run(&["sessions", "--status"])
            .success()
            .stdout(predicate::str::contains("degraded"));
    }
}
