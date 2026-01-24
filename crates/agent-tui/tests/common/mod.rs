#![allow(dead_code)]
#![allow(unused_imports)]

pub mod mock_daemon;
pub mod test_harness;

#[allow(unused_imports)]
pub use mock_daemon::{MockDaemon, MockResponse, RecordedRequest};
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

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

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

use serde_json::Value;
use std::time::Instant;

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

pub fn timed<F, T>(f: F) -> (T, Duration)
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = f();
    (result, start.elapsed())
}

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
