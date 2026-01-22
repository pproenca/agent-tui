//! Real daemon test harness for E2E workflow tests
//!
//! Each test spawns an isolated daemon instance on a unique socket,
//! allowing tests to run in parallel without state conflicts.

use serde_json::Value;
use std::cell::RefCell;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// Test harness for E2E tests with isolated daemon per test.
///
/// Spawns a daemon on a unique socket path and tracks sessions for cleanup.
pub struct RealTestHarness {
    sessions: RefCell<Vec<String>>,
    daemon_socket: PathBuf,
    daemon_process: RefCell<Option<Child>>,
}

impl RealTestHarness {
    pub fn new() -> Self {
        let daemon_socket =
            PathBuf::from(format!("/tmp/agent-tui-test-{}.sock", uuid::Uuid::new_v4()));

        let daemon_process = Command::new(env!("CARGO_BIN_EXE_agent-tui"))
            .arg("daemon")
            .env("AGENT_TUI_SOCKET", &daemon_socket)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to spawn test daemon");

        for _ in 0..50 {
            if daemon_socket.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        assert!(
            daemon_socket.exists(),
            "Daemon socket not created at {}",
            daemon_socket.display()
        );

        Self {
            sessions: RefCell::new(Vec::new()),
            daemon_socket,
            daemon_process: RefCell::new(Some(daemon_process)),
        }
    }

    /// Get CLI command configured for this test's isolated daemon.
    pub fn cli(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_agent-tui"));
        cmd.env("AGENT_TUI_SOCKET", &self.daemon_socket);
        cmd
    }

    /// CLI with JSON output format.
    pub fn cli_json(&self) -> Command {
        let mut cmd = self.cli();
        cmd.args(["-f", "json"]);
        cmd
    }

    /// Get the socket path for this test's daemon.
    pub fn socket_path(&self) -> &std::path::Path {
        &self.daemon_socket
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

        let wait_status = self
            .cli()
            .args(["-s", &session_id, "wait", "-t", "5000", "agent-tui Demo"])
            .status()
            .expect("wait command failed");

        assert!(wait_status.success(), "wait for demo UI failed");

        self.sessions.borrow_mut().push(session_id.clone());
        session_id
    }

    /// Kill a specific session with logging on failure.
    pub fn kill_session(&self, session_id: &str) {
        match self.cli().args(["-s", session_id, "kill"]).status() {
            Ok(status) if status.success() => {}
            Ok(status) => {
                eprintln!(
                    "Warning: Session {} kill failed with exit code {:?}",
                    session_id,
                    status.code()
                );
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to execute kill for session {}: {}",
                    session_id, e
                );
            }
        }
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

        if let Some(mut daemon) = self.daemon_process.borrow_mut().take() {
            if let Err(e) = daemon.kill() {
                eprintln!("Warning: Failed to kill daemon process: {}", e);
            }
            match daemon.wait() {
                Ok(status) if !status.success() => {
                    eprintln!("Warning: Daemon exited with status: {:?}", status.code());
                }
                Err(e) => {
                    eprintln!("Warning: Failed to wait for daemon: {}", e);
                }
                _ => {}
            }
        }

        if self.daemon_socket.exists() {
            if let Err(e) = std::fs::remove_file(&self.daemon_socket) {
                eprintln!(
                    "Warning: Failed to remove socket {}: {}",
                    self.daemon_socket.display(),
                    e
                );
            }
        }

        let lock_path = self.daemon_socket.with_extension("lock");
        if lock_path.exists() {
            if let Err(e) = std::fs::remove_file(&lock_path) {
                eprintln!(
                    "Warning: Failed to remove lock file {}: {}",
                    lock_path.display(),
                    e
                );
            }
        }
    }
}
