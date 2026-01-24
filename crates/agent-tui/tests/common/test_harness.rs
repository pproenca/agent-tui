#![allow(dead_code)]

use super::mock_daemon::{MockDaemon, MockResponse, RecordedRequest};
use assert_cmd::Command;
use serde_json::Value;
use tokio::runtime::Runtime;

pub struct TestHarness {
    daemon: MockDaemon,
    runtime: Runtime,
}

impl TestHarness {
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");
        let daemon = runtime.block_on(MockDaemon::start());
        Self { daemon, runtime }
    }

    pub fn cli_command(&self) -> Command {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("agent-tui"));
        for (key, value) in self.daemon.env_vars() {
            cmd.env(key, value);
        }
        cmd
    }

    pub fn run(&self, args: &[&str]) -> assert_cmd::assert::Assert {
        self.cli_command().args(args).assert()
    }

    pub fn get_requests(&self) -> Vec<RecordedRequest> {
        self.daemon.get_requests()
    }

    pub fn last_request_for(&self, method: &str) -> Option<RecordedRequest> {
        self.daemon.last_request_for(method)
    }

    pub fn set_response(&self, method: &str, response: MockResponse) {
        self.daemon.set_response(method, response);
    }

    pub fn clear_requests(&self) {
        self.daemon.clear_requests();
    }

    pub fn set_success_response(&self, method: &str, result: Value) {
        self.set_response(method, MockResponse::Success(result));
    }

    pub fn set_error_response(&self, method: &str, code: i32, message: &str) {
        self.set_response(
            method,
            MockResponse::Error {
                code,
                message: message.to_string(),
            },
        );
    }

    pub fn assert_method_called(&self, method: &str) {
        let requests = self.get_requests();
        assert!(
            requests.iter().any(|r| r.method == method),
            "Expected method '{}' to be called, but it wasn't. Calls: {:?}",
            method,
            requests.iter().map(|r| &r.method).collect::<Vec<_>>()
        );
    }

    pub fn assert_method_called_with(&self, method: &str, expected_params: Value) {
        let request = self
            .last_request_for(method)
            .unwrap_or_else(|| panic!("Expected method '{}' to be called", method));

        let actual_params = request.params.clone().unwrap_or(Value::Null);

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

        let _ = harness.run(&["sessions", "--status"]);

        let requests = harness.get_requests();
        assert!(
            requests.iter().any(|r| r.method == "health"),
            "health method should have been called"
        );
    }

    #[test]
    fn test_harness_custom_response() {
        let harness = TestHarness::new();

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

        harness
            .run(&["sessions", "--status"])
            .success()
            .stdout(predicate::str::contains("degraded"));
    }
}
