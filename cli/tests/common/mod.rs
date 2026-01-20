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
pub mod test_harness;

#[allow(unused_imports)]
pub use mock_daemon::{MockDaemon, MockResponse, RecordedRequest};
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
