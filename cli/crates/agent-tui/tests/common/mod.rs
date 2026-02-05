#![expect(dead_code, reason = "Test harness helpers are used selectively.")]
#![expect(unused_imports, reason = "Test harness re-exports vary by test.")]

//! Test harness exports.

pub mod interactive_pty;
pub mod mock_daemon;
pub mod real_test_harness;
pub mod test_harness;

#[allow(unused_imports)]
pub use interactive_pty::InteractivePtyRunner;
#[allow(unused_imports)]
pub use mock_daemon::{MockDaemon, MockResponse, RecordedRequest};
pub use real_test_harness::RealTestHarness;
pub use test_harness::TestHarness;

use assert_cmd::Command;
use std::path::PathBuf;

pub fn agent_tui_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("agent-tui"))
}

pub fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

pub const TEST_SESSION_ID: &str = "test-session-abc123";

pub const TEST_PID: u32 = 12345;

pub const TEST_COLS: u16 = 120;
pub const TEST_ROWS: u16 = 40;

use serde_json::Value;

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
