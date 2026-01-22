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
