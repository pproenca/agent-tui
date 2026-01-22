//! Real daemon test harness for E2E workflow tests
//!
//! Unlike TestHarness (mock-based), RealTestHarness runs tests against the
//! actual daemon. Use for workflow verification where real PTY interaction
//! is required.

use serde_json::Value;
use std::cell::RefCell;
use std::process::Command;

/// Test harness for E2E tests with real daemon.
///
/// Tracks spawned sessions for automatic cleanup on drop.
pub struct RealTestHarness {
    sessions: RefCell<Vec<String>>,
}

impl RealTestHarness {
    pub fn new() -> Self {
        Self {
            sessions: RefCell::new(Vec::new()),
        }
    }

    /// Get CLI command configured for testing.
    pub fn cli(&self) -> Command {
        Command::new(env!("CARGO_BIN_EXE_agent-tui"))
    }

    /// CLI with JSON output format.
    pub fn cli_json(&self) -> Command {
        let mut cmd = self.cli();
        cmd.args(["-f", "json"]);
        cmd
    }

    /// Spawn bash, wait for prompt, return session_id.
    pub fn spawn_bash(&self) -> String {
        let output = self
            .cli_json()
            .args(["spawn", "bash"])
            .output()
            .expect("spawn failed");

        assert!(
            output.status.success(),
            "spawn failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let json: Value =
            serde_json::from_slice(&output.stdout).expect("failed to parse spawn output as JSON");
        let session_id = json["session_id"]
            .as_str()
            .expect("no session_id in response")
            .to_string();

        // Wait for shell prompt
        let wait_status = self
            .cli()
            .args(["-s", &session_id, "wait", "-t", "5000", "$"])
            .status()
            .expect("wait command failed");

        assert!(wait_status.success(), "wait for bash prompt failed");

        self.sessions.borrow_mut().push(session_id.clone());
        session_id
    }

    /// Spawn demo TUI, wait for title, return session_id.
    pub fn spawn_demo(&self) -> String {
        let output = self
            .cli_json()
            .args(["demo"])
            .output()
            .expect("spawn demo failed");

        assert!(
            output.status.success(),
            "demo spawn failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let json: Value =
            serde_json::from_slice(&output.stdout).expect("failed to parse demo output as JSON");
        let session_id = json["session_id"]
            .as_str()
            .expect("no session_id in response")
            .to_string();

        // Wait for demo UI title
        let wait_status = self
            .cli()
            .args(["-s", &session_id, "wait", "-t", "5000", "agent-tui Demo"])
            .status()
            .expect("wait command failed");

        assert!(wait_status.success(), "wait for demo UI failed");

        self.sessions.borrow_mut().push(session_id.clone());
        session_id
    }

    /// Kill a specific session (errors are ignored during cleanup).
    pub fn kill_session(&self, session_id: &str) {
        // Ignore errors - session may already be dead or daemon not running
        let _ = self.cli().args(["-s", session_id, "kill"]).status();
    }

    /// Cleanup all tracked sessions.
    pub fn cleanup(&self) {
        for session_id in self.sessions.borrow().iter() {
            self.kill_session(session_id);
        }
        self.sessions.borrow_mut().clear();
    }
}

impl Default for RealTestHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RealTestHarness {
    fn drop(&mut self) {
        self.cleanup();
    }
}
