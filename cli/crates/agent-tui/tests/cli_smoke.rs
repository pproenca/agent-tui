mod common;

use common::{MockResponse, TEST_COLS, TEST_ROWS, TEST_SESSION_ID, TestHarness};
use predicates::prelude::*;
use serde_json::json;

use assert_cmd::Command;
use std::path::PathBuf;
use tempfile::TempDir;

/// LSB exit code 3: program is not running
const EXIT_NOT_RUNNING: i32 = 3;

struct NoDaemonTestEnv {
    _temp_dir: TempDir,
    socket_path: PathBuf,
}

impl NoDaemonTestEnv {
    fn new() -> Self {
        let temp_dir = TempDir::new_in("/tmp").expect("Failed to create temp dir");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ =
                std::fs::set_permissions(temp_dir.path(), std::fs::Permissions::from_mode(0o777));
        }
        let socket_path = temp_dir.path().join("no-daemon.sock");

        Self {
            _temp_dir: temp_dir,
            socket_path,
        }
    }

    fn cli_command(&self) -> Command {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("agent-tui"));
        cmd.env("AGENT_TUI_SOCKET", &self.socket_path);
        cmd
    }

    fn run(&self, args: &[&str]) -> assert_cmd::assert::Assert {
        self.cli_command().args(args).assert()
    }
}

#[test]
fn smoke_daemon_status_text() {
    let harness = TestHarness::new();

    harness
        .run(&["daemon", "status"])
        .success()
        .stdout(predicate::str::contains("Daemon status:"))
        .stdout(predicate::str::contains("healthy"));
}

#[test]
fn smoke_daemon_status_json() {
    let harness = TestHarness::new();

    let output = harness.run(&["-f", "json", "daemon", "status"]).success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON output");

    assert_eq!(json["running"], true);
    assert_eq!(json["status"], "healthy");
    assert!(json["cli_version"].is_string());
}

#[test]
fn smoke_no_autostart_daemon_status() {
    let env = NoDaemonTestEnv::new();

    env.run(&["daemon", "status"])
        .code(EXIT_NOT_RUNNING)
        .stdout(predicate::str::contains("Daemon is not running"));
}

#[test]
fn smoke_env_without_daemon() {
    let env = NoDaemonTestEnv::new();

    env.run(&["env"])
        .success()
        .stdout(predicate::str::contains("Environment"));
}

#[test]
fn smoke_sessions_empty() {
    let harness = TestHarness::new();

    harness
        .run(&["sessions"])
        .success()
        .stdout(predicate::str::contains("No active sessions"));
}

#[test]
fn smoke_screenshot_with_elements() {
    let harness = TestHarness::new();

    harness.set_success_response(
        "snapshot",
        json!({
            "session_id": TEST_SESSION_ID,
            "screenshot": "Test screen\n",
            "elements": [
                {
                    "ref": "@btn1",
                    "type": "button",
                    "label": "Submit",
                    "position": { "row": 5, "col": 10 },
                    "focused": true,
                    "selected": false
                }
            ],
            "cursor": { "row": 0, "col": 0, "visible": true },
            "size": { "cols": TEST_COLS, "rows": TEST_ROWS }
        }),
    );

    harness
        .run(&["screenshot", "-e"])
        .success()
        .stdout(predicate::str::contains("@btn1"));
}

#[test]
fn smoke_action_click_success_and_error() {
    let harness = TestHarness::new();

    harness.set_success_response("click", json!({"success": true, "message": null}));

    harness
        .run(&["action", "@btn1", "click"])
        .success()
        .stdout(predicate::str::contains("Clicked"));

    harness.set_response(
        "click",
        MockResponse::StructuredError {
            code: -32003,
            message: "Element not found: @missing".to_string(),
            category: Some("not_found".to_string()),
            retryable: Some(false),
            context: Some(json!({"element_ref": "@missing"})),
            suggestion: Some("Use 'screenshot -e' to see available elements".to_string()),
        },
    );

    harness
        .run(&["action", "@missing", "click"])
        .failure()
        .stderr(predicate::str::contains("Element not found"));
}

#[test]
fn smoke_run_spawns_session() {
    let harness = TestHarness::new();

    harness.run(&["run", "bash"]).success();
    harness.assert_method_called("spawn");
}
