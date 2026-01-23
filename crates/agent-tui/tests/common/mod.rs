//! Common test utilities for agent-tui CLI tests
//!
//! This module provides:
//! - MockDaemon: A simulated daemon that responds to JSON-RPC requests
//! - TestHarness: Sync wrapper for E2E tests with mock daemon
//! - Test helpers for CLI invocation
//! - Response fixtures for deterministic testing

#![allow(dead_code)]
#![allow(deprecated)]
#![allow(unused_imports)]

pub mod mock_daemon;
pub mod real_test_harness;
pub mod test_harness;

#[allow(unused_imports)]
pub use mock_daemon::{MockDaemon, MockResponse, RecordedRequest};
pub use real_test_harness::RealTestHarness;
pub use test_harness::TestHarness;

use assert_cmd::Command;
use std::path::PathBuf;

/// Get a Command configured to run agent-tui binary
pub fn agent_tui_cmd() -> Command {
    Command::cargo_bin("agent-tui").unwrap()
}

/// Get the path to the test fixtures directory
pub fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Standard test session ID for mock responses
pub const TEST_SESSION_ID: &str = "test-session-abc123";

/// Standard test PID for mock responses
pub const TEST_PID: u32 = 12345;

/// Standard terminal size for tests
pub const TEST_COLS: u16 = 120;
pub const TEST_ROWS: u16 = 40;

// ============================================================
// Timeout utilities
// ============================================================

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Execute a function with a timeout, returning None if it doesn't complete in time.
///
/// Spawns the function on a background thread and waits for completion
/// up to the specified duration. Useful for preventing test hangs.
///
/// # Example
/// ```ignore
/// let result = with_timeout(Duration::from_secs(5), || {
///     some_potentially_slow_operation()
/// });
/// assert!(result.is_some(), "Operation timed out");
/// ```
pub fn with_timeout<F, T>(duration: Duration, f: F) -> Option<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = f();
        let _ = tx.send(result);
    });
    rx.recv_timeout(duration).ok()
}

// ============================================================
// Error assertion helpers
// ============================================================

use serde_json::Value;
use std::time::Instant;

/// Assert structured error data from a JSON error response.
///
/// Validates that the error contains the expected code, category, and retryable flag.
pub fn assert_error_data(
    json: &Value,
    expected_code: Option<i32>,
    expected_category: Option<&str>,
    expected_retryable: Option<bool>,
) {
    if let Some(code) = expected_code {
        assert_eq!(
            json.get("code").and_then(|v| v.as_i64()),
            Some(code as i64),
            "Error code mismatch. JSON: {}",
            json
        );
    }
    if let Some(category) = expected_category {
        assert_eq!(
            json.get("category").and_then(|v| v.as_str()),
            Some(category),
            "Error category mismatch. JSON: {}",
            json
        );
    }
    if let Some(retryable) = expected_retryable {
        assert_eq!(
            json.get("retryable").and_then(|v| v.as_bool()),
            Some(retryable),
            "Error retryable flag mismatch. JSON: {}",
            json
        );
    }
}

/// Execute a function and measure its duration.
///
/// Returns a tuple of (result, duration).
pub fn timed<F, T>(f: F) -> (T, Duration)
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = f();
    (result, start.elapsed())
}

/// Assert that a duration is within expected bounds (with tolerance).
pub fn assert_duration_between(duration: Duration, min: Duration, max: Duration, context: &str) {
    assert!(
        duration >= min,
        "{}: duration {:?} is less than minimum {:?}",
        context,
        duration,
        min
    );
    assert!(
        duration <= max,
        "{}: duration {:?} exceeds maximum {:?}",
        context,
        duration,
        max
    );
}
