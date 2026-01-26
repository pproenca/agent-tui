#![allow(dead_code)]

use super::mock_daemon::{MockDaemon, MockResponse, RecordedRequest};
use assert_cmd::Command;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::time::Duration;
use tokio::runtime::Runtime;

// Shared runtime keeps the mock daemon server running while CLI processes execute.
static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    let worker_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .min(4);
    tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .enable_io()
        .worker_threads(worker_threads)
        .build()
        .expect("Failed to create tokio runtime")
});

pub struct TestHarness {
    daemon: MockDaemon,
}

impl TestHarness {
    pub fn new() -> Self {
        let daemon = RUNTIME.block_on(MockDaemon::start());
        Self { daemon }
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

    pub fn set_delayed_response(&self, method: &str, delay: Duration, next: MockResponse) {
        self.set_response(method, MockResponse::Delayed(delay, Box::new(next)));
    }

    pub fn advance_time(&self, duration: Duration) {
        self.daemon.advance_time(duration);
    }

    pub fn wait_for_request_count(&self, count: usize) {
        self.daemon.wait_for_request_count(count);
    }

    pub fn wait_for_pending_delays(&self, count: usize) {
        self.daemon.wait_for_pending_delays(count);
    }

    pub fn set_junk_then_response(&self, method: &str, junk: &str, next: MockResponse) {
        self.set_response(
            method,
            MockResponse::JunkThen(Box::new(next), junk.to_string()),
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

    pub fn assert_param(&self, param_name: &str, expected_value: Value) {
        let requests = self.get_requests();
        let last_request = requests.last().expect("No requests recorded");
        let params = last_request.params.as_ref().expect("Request has no params");
        let actual = params
            .get(param_name)
            .unwrap_or_else(|| panic!("Param '{}' not found. Params: {:?}", param_name, params));
        assert_eq!(
            actual, &expected_value,
            "Param '{}' mismatch. Expected: {:?}, Actual: {:?}",
            param_name, expected_value, actual
        );
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

        let _ = harness.run(&["sessions", "status"]);

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
            .run(&["sessions", "status"])
            .success()
            .stdout(predicate::str::contains("degraded"));
    }
}
