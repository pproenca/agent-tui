//! Tests for daemon status/stop commands NOT auto-starting the daemon.
//!
//! These tests verify that `daemon status` and `daemon stop` do not auto-start
//! the daemon when it's not running. They run WITHOUT the mock daemon.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;
use tempfile::TempDir;

/// Creates a test environment with a unique socket path where no daemon is running.
struct NoDaemonTestEnv {
    _temp_dir: TempDir,
    socket_path: PathBuf,
}

impl NoDaemonTestEnv {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let socket_path = temp_dir.path().join("no-daemon.sock");

        Self {
            _temp_dir: temp_dir,
            socket_path,
        }
    }

    fn cli_command(&self) -> Command {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("agent-tui"));
        // Point to our non-existent socket
        cmd.env("AGENT_TUI_SOCKET", &self.socket_path);
        cmd
    }

    fn run(&self, args: &[&str]) -> assert_cmd::assert::Assert {
        self.cli_command().args(args).assert()
    }
}

/// LSB exit code 3: program is not running
const EXIT_NOT_RUNNING: i32 = 3;

#[test]
fn daemon_status_shows_not_running_when_daemon_stopped() {
    let env = NoDaemonTestEnv::new();

    // Should show "not running" message and exit with LSB code 3
    env.run(&["daemon", "status"])
        .code(EXIT_NOT_RUNNING)
        .stdout(predicate::str::contains("Daemon is not running"));
}

#[test]
fn daemon_status_json_shows_running_false_when_daemon_stopped() {
    let env = NoDaemonTestEnv::new();

    // JSON output should also exit with LSB code 3
    env.run(&["--json", "daemon", "status"])
        .code(EXIT_NOT_RUNNING)
        .stdout(predicate::str::contains("\"running\":false"))
        .stdout(predicate::str::contains("\"cli_version\""));
}

#[test]
fn daemon_status_does_not_create_socket_file() {
    let env = NoDaemonTestEnv::new();

    // Verify socket doesn't exist before
    assert!(!env.socket_path.exists());

    // Run daemon status (expect exit code 3 for "not running")
    env.run(&["daemon", "status"]).code(EXIT_NOT_RUNNING);

    // Verify socket still doesn't exist (daemon was NOT auto-started)
    assert!(
        !env.socket_path.exists(),
        "Socket file should not exist - daemon should not have been auto-started"
    );
}

#[test]
fn daemon_stop_returns_error_when_daemon_not_running() {
    let env = NoDaemonTestEnv::new();

    env.run(&["daemon", "stop"])
        .failure()
        .stderr(predicate::str::contains("Daemon not running"));
}

#[test]
fn daemon_stop_does_not_create_socket_file() {
    let env = NoDaemonTestEnv::new();

    // Verify socket doesn't exist before
    assert!(!env.socket_path.exists());

    // Run daemon stop (will fail since daemon not running)
    let _ = env.run(&["daemon", "stop"]);

    // Verify socket still doesn't exist (daemon was NOT auto-started)
    assert!(
        !env.socket_path.exists(),
        "Socket file should not exist - daemon should not have been auto-started"
    );
}
