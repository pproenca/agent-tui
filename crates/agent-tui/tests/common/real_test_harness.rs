//! Real daemon test harness for E2E workflow tests
//!
//! Each test spawns an isolated daemon instance on a unique socket,
//! allowing tests to run in parallel without state conflicts.

use serde_json::Value;
use std::cell::RefCell;
use std::io::Read;
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

        let mut daemon_process = Command::new(env!("CARGO_BIN_EXE_agent-tui"))
            .arg("daemon")
            .env("AGENT_TUI_SOCKET", &daemon_socket)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn test daemon");

        // Exponential backoff for faster daemon startup detection
        let backoff_intervals = [5, 10, 20, 40, 80, 160, 320];
        for ms in backoff_intervals {
            if daemon_socket.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(ms));
        }

        if !daemon_socket.exists() {
            let stderr_output = daemon_process
                .stderr
                .as_mut()
                .and_then(|stderr| {
                    let mut buf = String::new();
                    stderr.read_to_string(&mut buf).ok().map(|_| buf)
                })
                .unwrap_or_else(|| "<no output>".to_string());
            panic!(
                "Daemon socket not created at {}. Daemon stderr: {}",
                daemon_socket.display(),
                stderr_output
            );
        }

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
            .args(["run", "bash"])
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
            .args(["-s", &session_id, "wait", "-t", "1000", "$"])
            .status()
            .expect("wait command failed");

        assert!(wait_status.success(), "wait for bash prompt failed");

        self.sessions.borrow_mut().push(session_id.clone());
        session_id
    }

    /// Kill a specific session with logging on failure.
    pub fn kill_session(&self, session_id: &str) {
        match self.cli().args(["-s", session_id, "kill"]).status() {
            Ok(status) if status.success() => {}
            Ok(status) => {
                eprintln!(
                    "Warning: Session {} kill failed with exit code {:?}. \
                     The session may have already terminated or does not exist.",
                    session_id,
                    status.code()
                );
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to execute kill for session {}: {}. \
                     The daemon may have already terminated.",
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

fn remove_file_if_exists(path: &std::path::Path, description: &str) {
    if path.exists() {
        if let Err(e) = std::fs::remove_file(path) {
            eprintln!("Warning: Failed to remove {}: {}", description, e);
        }
    }
}

impl Drop for RealTestHarness {
    fn drop(&mut self) {
        self.cleanup();

        if let Some(mut daemon) = self.daemon_process.borrow_mut().take() {
            // Check if already exited before attempting kill
            match daemon.try_wait() {
                Ok(Some(_)) => {
                    // Already exited, no need to kill
                }
                Ok(None) => {
                    // Still running, kill it
                    if let Err(e) = daemon.kill() {
                        eprintln!(
                            "Warning: Failed to kill daemon process: {}. \
                             Process may have exited between try_wait and kill.",
                            e
                        );
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to check daemon status: {}. Attempting kill anyway.",
                        e
                    );
                    let _ = daemon.kill();
                }
            }

            // Always wait to reap the process and prevent zombies
            match daemon.wait() {
                Ok(status) if !status.success() => {
                    eprintln!(
                        "Warning: Daemon exited with status: {:?}. \
                         This may indicate an error during shutdown.",
                        status.code()
                    );
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to wait for daemon: {}. \
                         Process may become a zombie.",
                        e
                    );
                }
                _ => {}
            }
        }

        remove_file_if_exists(&self.daemon_socket, "socket");
        remove_file_if_exists(&self.daemon_socket.with_extension("lock"), "lock file");
    }
}
