#![expect(
    clippy::expect_used,
    reason = "Test-only assertions use expect for clarity."
)]

//! Command contract tests for full CLI surface coverage.

mod common;

use agent_tui::cli_command;
use assert_cmd::Command;
use common::MockResponse;
use common::TestHarness;
use predicates::prelude::*;
use serde_json::Value;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command as StdCommand;
use sysinfo::Pid;
use sysinfo::ProcessRefreshKind;
use sysinfo::ProcessesToUpdate;
use sysinfo::System;
use tempfile::TempDir;

struct CommandCase {
    args: &'static [&'static str],
    expected_method: &'static str,
    setup: fn(&TestHarness),
}

fn no_setup(_: &TestHarness) {}

fn setup_running_session(harness: &TestHarness) {
    harness.set_success_response(
        "sessions",
        json!({
            "sessions": [{
                "id": "session-1",
                "command": "bash",
                "pid": 12345,
                "running": true,
                "created_at": "2026-01-01T00:00:00Z",
                "size": { "cols": 120, "rows": 40 }
            }],
            "active_session": "session-1"
        }),
    );
}

fn setup_mixed_sessions(harness: &TestHarness) {
    harness.set_success_response(
        "sessions",
        json!({
            "sessions": [
                {
                    "id": "running-1",
                    "command": "bash",
                    "pid": 11111,
                    "running": true,
                    "created_at": "2026-01-01T00:00:00Z",
                    "size": { "cols": 120, "rows": 40 }
                },
                {
                    "id": "stopped-1",
                    "command": "bash",
                    "pid": 22222,
                    "running": false,
                    "created_at": "2026-01-01T00:00:00Z",
                    "size": { "cols": 120, "rows": 40 }
                }
            ],
            "active_session": "running-1"
        }),
    );
}

struct StandaloneEnv {
    _temp_dir: TempDir,
    socket_path: PathBuf,
    ws_state_path: PathBuf,
    session_store_path: PathBuf,
    ui_state_path: PathBuf,
}

impl StandaloneEnv {
    fn new() -> Self {
        let temp_dir = TempDir::new_in("/tmp").expect("Failed to create temp dir");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ =
                std::fs::set_permissions(temp_dir.path(), std::fs::Permissions::from_mode(0o777));
        }
        Self {
            socket_path: temp_dir.path().join("daemon.sock"),
            ws_state_path: temp_dir.path().join("api.json"),
            session_store_path: temp_dir.path().join("sessions.jsonl"),
            ui_state_path: temp_dir.path().join("ui.json"),
            _temp_dir: temp_dir,
        }
    }

    fn cli_command(&self) -> Command {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("agent-tui"));
        cmd.env("AGENT_TUI_SOCKET", &self.socket_path)
            .env("AGENT_TUI_WS_STATE", &self.ws_state_path)
            .env("AGENT_TUI_SESSION_STORE", &self.session_store_path)
            .env("AGENT_TUI_UI_STATE", &self.ui_state_path)
            .env("NO_COLOR", "1");
        cmd
    }

    fn run(&self, args: &[&str]) -> assert_cmd::assert::Assert {
        self.cli_command().args(args).assert()
    }

    fn write_ws_state(&self) {
        let process_started_at = current_process_started_at();
        let data = json!({
            "pid": std::process::id(),
            "ws_url": "ws://127.0.0.1:43210/ws",
            "ui_url": "http://127.0.0.1:43210/ui",
            "listen": "127.0.0.1:43210",
            "started_at": 1735689600,
            "process_started_at": process_started_at,
        });
        fs::write(
            &self.ws_state_path,
            serde_json::to_string_pretty(&data).expect("serialize ws state"),
        )
        .expect("write ws state");
    }

    fn stop_daemon_best_effort(&self) {
        let _ = StdCommand::new(assert_cmd::cargo::cargo_bin!("agent-tui"))
            .env("AGENT_TUI_SOCKET", &self.socket_path)
            .env("AGENT_TUI_WS_STATE", &self.ws_state_path)
            .args(["daemon", "stop", "--force", "--yes"])
            .output();
    }
}

impl Drop for StandaloneEnv {
    fn drop(&mut self) {
        self.stop_daemon_best_effort();
    }
}

fn current_process_started_at() -> Option<u64> {
    let pid = Pid::from_u32(std::process::id());
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing(),
    );
    let process = system.process(pid)?;
    let started_at = process.start_time();
    (started_at > 0).then(|| System::boot_time().saturating_add(started_at))
}

fn collect_command_paths(
    command: &clap::Command,
    prefix: Option<&str>,
    out: &mut BTreeSet<String>,
) {
    for sub in command.get_subcommands() {
        let name = sub.get_name();
        if name == "help" {
            continue;
        }
        let path = match prefix {
            Some(parent) => format!("{parent} {name}"),
            None => name.to_string(),
        };
        out.insert(path.clone());
        collect_command_paths(sub, Some(&path), out);
    }
}

#[test]
fn command_paths_match_expected_matrix() {
    let command = cli_command();
    let mut discovered = BTreeSet::new();
    collect_command_paths(&command, None, &mut discovered);

    let expected = BTreeSet::from([
        "completions".to_string(),
        "daemon".to_string(),
        "daemon restart".to_string(),
        "daemon run".to_string(),
        "daemon start".to_string(),
        "daemon status".to_string(),
        "daemon stop".to_string(),
        "action".to_string(),
        "env".to_string(),
        "input".to_string(),
        "kill".to_string(),
        "live".to_string(),
        "live start".to_string(),
        "live status".to_string(),
        "live stop".to_string(),
        "press".to_string(),
        "resize".to_string(),
        "restart".to_string(),
        "run".to_string(),
        "screenshot".to_string(),
        "scroll".to_string(),
        "scroll-into-view".to_string(),
        "sessions".to_string(),
        "sessions attach".to_string(),
        "sessions cleanup".to_string(),
        "sessions list".to_string(),
        "sessions show".to_string(),
        "sessions switch".to_string(),
        "type".to_string(),
        "version".to_string(),
        "wait".to_string(),
    ]);

    assert_eq!(
        discovered, expected,
        "CLI command paths changed. Update command contracts and docs accordingly."
    );
}

#[test]
fn rpc_contract_matrix_covers_full_working_surface() {
    let harness = TestHarness::new();
    let cases = [
        CommandCase {
            args: &["run", "bash"],
            expected_method: "spawn",
            setup: no_setup,
        },
        CommandCase {
            args: &["screenshot"],
            expected_method: "snapshot",
            setup: no_setup,
        },
        CommandCase {
            args: &["resize", "--cols", "88", "--rows", "22"],
            expected_method: "resize",
            setup: no_setup,
        },
        CommandCase {
            args: &["restart", "--yes"],
            expected_method: "restart",
            setup: no_setup,
        },
        CommandCase {
            args: &["press", "Enter"],
            expected_method: "keystroke",
            setup: no_setup,
        },
        CommandCase {
            args: &["action", "@submit", "click"],
            expected_method: "keystroke",
            setup: no_setup,
        },
        CommandCase {
            args: &["press", "Shift", "--hold"],
            expected_method: "keydown",
            setup: no_setup,
        },
        CommandCase {
            args: &["press", "Shift", "--release"],
            expected_method: "keyup",
            setup: no_setup,
        },
        CommandCase {
            args: &["type", "hello"],
            expected_method: "type",
            setup: no_setup,
        },
        CommandCase {
            args: &["input", "hello"],
            expected_method: "type",
            setup: no_setup,
        },
        CommandCase {
            args: &["scroll", "down", "3"],
            expected_method: "keystroke",
            setup: no_setup,
        },
        CommandCase {
            args: &["wait", "done"],
            expected_method: "wait",
            setup: no_setup,
        },
        CommandCase {
            args: &["wait", "-e", "@ready"],
            expected_method: "wait",
            setup: no_setup,
        },
        CommandCase {
            args: &["wait", "--stable"],
            expected_method: "wait",
            setup: no_setup,
        },
        CommandCase {
            args: &["wait", "Loading", "--gone"],
            expected_method: "wait",
            setup: no_setup,
        },
        CommandCase {
            args: &["kill", "--yes"],
            expected_method: "kill",
            setup: no_setup,
        },
        CommandCase {
            args: &["sessions"],
            expected_method: "sessions",
            setup: no_setup,
        },
        CommandCase {
            args: &["sessions", "list"],
            expected_method: "sessions",
            setup: no_setup,
        },
        CommandCase {
            args: &["sessions", "ls"],
            expected_method: "sessions",
            setup: no_setup,
        },
        CommandCase {
            args: &["sessions", "show", "session-1"],
            expected_method: "sessions",
            setup: setup_running_session,
        },
        CommandCase {
            args: &["sessions", "switch", "session-1"],
            expected_method: "attach",
            setup: no_setup,
        },
        CommandCase {
            args: &["sessions", "select", "session-1"],
            expected_method: "attach",
            setup: no_setup,
        },
        CommandCase {
            args: &["sessions", "attach", "-T"],
            expected_method: "attach",
            setup: setup_running_session,
        },
        CommandCase {
            args: &["sessions", "cleanup", "--yes"],
            expected_method: "sessions",
            setup: setup_mixed_sessions,
        },
        CommandCase {
            args: &["sessions", "cleanup", "--all", "--yes"],
            expected_method: "sessions",
            setup: setup_mixed_sessions,
        },
    ];

    for case in cases {
        harness.clear_requests();
        (case.setup)(&harness);
        harness
            .run(case.args)
            .success()
            .stderr(predicate::str::contains("Error").not());
        harness.assert_method_called(case.expected_method);
    }
}

#[test]
fn sessions_cleanup_kills_stopped_sessions() {
    let harness = TestHarness::new();
    setup_mixed_sessions(&harness);
    harness.run(&["sessions", "cleanup", "--yes"]).success();
    harness.assert_method_called("kill");
}

#[test]
fn type_dash_reads_from_stdin() {
    let harness = TestHarness::new();

    harness
        .cli_command()
        .args(["type", "-"])
        .write_stdin("hello from stdin")
        .assert()
        .success();

    harness.assert_method_called_with(
        "type",
        json!({
            "text": "hello from stdin"
        }),
    );
}

#[test]
fn legacy_screenshot_element_flag_preserves_json_stdout() {
    let harness = TestHarness::new();

    let output = harness
        .cli_command()
        .args(["--format", "json", "screenshot", "-e"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "agent-tui screenshot -e is deprecated; use `agent-tui screenshot` instead. It will be deprecated in the next major release.",
        ))
        .get_output()
        .clone();

    let stdout_text = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout_text.contains("deprecated"),
        "deprecation notice must not be written to JSON stdout: {stdout_text}"
    );
    let parsed: Value = serde_json::from_slice(&output.stdout).expect("valid JSON stdout");
    assert_eq!(parsed["screenshot"], "Test screen content\n");

    harness.assert_method_called_with(
        "snapshot",
        json!({
            "retain_ansi": true,
            "include_render": true
        }),
    );
}

#[test]
fn legacy_screenshot_element_flag_returns_text_compat_output() {
    let harness = TestHarness::new();

    harness
        .run(&["screenshot", "-e"])
        .success()
        .stdout(predicate::str::contains("Screenshot:"))
        .stdout(predicate::str::contains("Test screen content"))
        .stderr(predicate::str::contains(
            "agent-tui screenshot -e is deprecated; use `agent-tui screenshot` instead. It will be deprecated in the next major release.",
        ));

    harness.assert_method_called("snapshot");
}

#[test]
fn legacy_screenshot_accessibility_flag_returns_text_compat_output() {
    let harness = TestHarness::new();

    harness
        .run(&["screenshot", "-a"])
        .success()
        .stdout(predicate::str::contains("Screenshot:"))
        .stdout(predicate::str::contains("Test screen content"))
        .stderr(predicate::str::contains(
            "agent-tui screenshot -a is deprecated; use `agent-tui screenshot` instead. It will be deprecated in the next major release.",
        ));

    harness.assert_method_called("snapshot");
}

#[test]
fn legacy_screenshot_interactive_only_flag_returns_text_compat_output() {
    let harness = TestHarness::new();

    harness
        .run(&["screenshot", "--interactive-only"])
        .success()
        .stdout(predicate::str::contains("Screenshot:"))
        .stdout(predicate::str::contains("Test screen content"))
        .stderr(predicate::str::contains(
            "agent-tui screenshot --interactive-only is deprecated; use `agent-tui screenshot` instead. It will be deprecated in the next major release.",
        ));

    harness.assert_method_called("snapshot");
}

#[test]
fn legacy_screenshot_accessibility_flags_preserve_json_stdout() {
    let harness = TestHarness::new();

    let output = harness
        .cli_command()
        .args(["--format", "json", "screenshot", "-a", "--interactive-only"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "agent-tui screenshot -a is deprecated; use `agent-tui screenshot` instead. It will be deprecated in the next major release.",
        ))
        .stderr(predicate::str::contains(
            "agent-tui screenshot --interactive-only is deprecated; use `agent-tui screenshot` instead. It will be deprecated in the next major release.",
        ))
        .get_output()
        .clone();

    let stdout_text = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout_text.contains("deprecated"),
        "deprecation notice must not be written to JSON stdout: {stdout_text}"
    );
    let parsed: Value = serde_json::from_slice(&output.stdout).expect("valid JSON stdout");
    assert_eq!(parsed["screenshot"], "Test screen content\n");

    harness.assert_method_called("snapshot");
}

#[test]
fn legacy_input_alias_types_text_and_warns_to_stderr() {
    let harness = TestHarness::new();

    harness
        .run(&["input", "hello legacy"])
        .success()
        .stdout(predicate::str::contains("Text typed"))
        .stderr(predicate::str::contains(
            "agent-tui input is deprecated; use `agent-tui type` instead. It will be deprecated in the next major release.",
        ));

    harness.assert_method_called_with(
        "type",
        json!({
            "text": "hello legacy"
        }),
    );
}

#[test]
fn legacy_input_json_stdout_stays_valid_and_notice_stays_on_stderr() {
    let harness = TestHarness::new();

    let output = harness
        .cli_command()
        .args(["--format", "json", "input", "hello json"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "agent-tui input is deprecated; use `agent-tui type` instead. It will be deprecated in the next major release.",
        ))
        .get_output()
        .clone();

    let stdout_text = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout_text.contains("deprecated"),
        "deprecation notice must not be written to JSON stdout: {stdout_text}"
    );
    let parsed: Value = serde_json::from_slice(&output.stdout).expect("valid JSON stdout");
    assert_eq!(parsed["success"], true);

    harness.assert_method_called_with(
        "type",
        json!({
            "text": "hello json"
        }),
    );
}

#[test]
fn legacy_input_no_input_mode_does_not_prompt() {
    let harness = TestHarness::new();

    harness
        .run(&["--no-input", "input", "hello automation"])
        .success()
        .stdout(predicate::str::contains("Text typed"))
        .stderr(predicate::str::contains(
            "agent-tui input is deprecated; use `agent-tui type` instead. It will be deprecated in the next major release.",
        ))
        .stderr(predicate::str::contains("Confirmation required").not());

    harness.assert_method_called_with(
        "type",
        json!({
            "text": "hello automation"
        }),
    );
}

#[test]
fn legacy_action_click_routes_to_enter_and_warns_to_stderr() {
    let harness = TestHarness::new();

    harness
        .run(&["action", "@submit", "click"])
        .success()
        .stdout(predicate::str::contains("Key pressed"))
        .stderr(predicate::str::contains(
            "agent-tui action is deprecated; use `agent-tui press`, `agent-tui type`, or `agent-tui scroll` instead. It will be deprecated in the next major release.",
        ));

    harness.assert_method_called_with(
        "keystroke",
        json!({
            "key": "Enter"
        }),
    );
}

#[test]
fn legacy_action_fill_json_stdout_stays_valid_and_targets_session() {
    let harness = TestHarness::new();

    let output = harness
        .cli_command()
        .args([
            "--format",
            "json",
            "--session",
            "session-a",
            "action",
            "@name",
            "fill",
            "Pedro",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "agent-tui action is deprecated; use `agent-tui press`, `agent-tui type`, or `agent-tui scroll` instead. It will be deprecated in the next major release.",
        ))
        .get_output()
        .clone();

    let stdout_text = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout_text.contains("deprecated"),
        "deprecation notice must not be written to JSON stdout: {stdout_text}"
    );
    let parsed: Value = serde_json::from_slice(&output.stdout).expect("valid JSON stdout");
    assert_eq!(parsed["success"], true);

    harness.assert_method_called_with(
        "type",
        json!({
            "text": "Pedro",
            "session": "session-a"
        }),
    );
}

#[test]
fn legacy_action_selector_routing_is_session_scoped() {
    let harness = TestHarness::new();

    harness
        .run(&["--session", "session-a", "action", "@submit", "click"])
        .success();
    harness.assert_method_called_with(
        "keystroke",
        json!({
            "key": "Enter",
            "session": "session-a"
        }),
    );

    harness.clear_requests();
    harness
        .run(&["--session", "session-b", "action", "@submit", "click"])
        .success();
    harness.assert_method_called_with(
        "keystroke",
        json!({
            "key": "Enter",
            "session": "session-b"
        }),
    );
}

#[test]
fn legacy_action_unsupported_semantics_return_compat_error() {
    let harness = TestHarness::new();

    harness
        .run(&["action", "@checkbox", "toggle"])
        .code(64)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "Legacy action `toggle` for selector `@checkbox` is not supported.",
        ))
        .stderr(predicate::str::contains(
            "Use `agent-tui press`, `agent-tui type`, or `agent-tui scroll`.",
        ))
        .stderr(predicate::str::contains("unrecognized subcommand").not());
}

#[test]
fn legacy_action_unsupported_semantics_do_not_require_daemon() {
    let env = StandaloneEnv::new();

    env.run(&["action", "@checkbox", "toggle"])
        .code(64)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "Legacy action `toggle` for selector `@checkbox` is not supported.",
        ))
        .stderr(predicate::str::contains("Daemon is not running").not())
        .stderr(predicate::str::contains("unrecognized subcommand").not());
}

#[test]
fn legacy_action_missing_operation_returns_compat_error() {
    let env = StandaloneEnv::new();

    env.run(&["action", "@checkbox"])
        .code(64)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "Legacy action `missing` for selector `@checkbox` is not supported.",
        ))
        .stderr(predicate::str::contains("2 values required").not())
        .stderr(predicate::str::contains("Daemon is not running").not());
}

#[test]
fn legacy_wait_element_routes_to_literal_text_and_warns_to_stderr() {
    let harness = TestHarness::new();

    harness
        .run(&["wait", "-e", "@ready"])
        .success()
        .stdout(predicate::str::contains("Found after"))
        .stderr(predicate::str::contains(
            "agent-tui wait -e is deprecated; use `agent-tui wait <text>` instead. It will be deprecated in the next major release.",
        ));

    harness.assert_method_called_with(
        "wait",
        json!({
            "text": "@ready",
            "condition": "text"
        }),
    );
}

#[test]
fn legacy_wait_element_gone_json_stdout_stays_valid_and_targets_session() {
    let harness = TestHarness::new();

    let output = harness
        .cli_command()
        .args([
            "--format",
            "json",
            "--session",
            "session-a",
            "wait",
            "-e",
            "@ready",
            "--gone",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "agent-tui wait -e is deprecated; use `agent-tui wait <text>` instead. It will be deprecated in the next major release.",
        ))
        .get_output()
        .clone();

    let stdout_text = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout_text.contains("deprecated"),
        "deprecation notice must not be written to JSON stdout: {stdout_text}"
    );
    let parsed: Value = serde_json::from_slice(&output.stdout).expect("valid JSON stdout");
    assert_eq!(parsed["found"], true);

    harness.assert_method_called_with(
        "wait",
        json!({
            "text": "@ready",
            "condition": "text_gone",
            "session": "session-a"
        }),
    );
}

#[test]
fn legacy_wait_element_reference_routing_is_session_scoped() {
    let harness = TestHarness::new();

    harness
        .run(&["--session", "session-a", "wait", "-e", "@ready"])
        .success();
    harness.assert_method_called_with(
        "wait",
        json!({
            "text": "@ready",
            "session": "session-a"
        }),
    );

    harness.clear_requests();
    harness
        .run(&["--session", "session-b", "wait", "-e", "@ready"])
        .success();
    harness.assert_method_called_with(
        "wait",
        json!({
            "text": "@ready",
            "session": "session-b"
        }),
    );
}

#[test]
fn legacy_scroll_into_view_json_stdout_stays_valid_and_does_not_send_input() {
    let env = StandaloneEnv::new();

    let output = env
        .cli_command()
        .args(["--format", "json", "scroll-into-view", "@details"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "agent-tui scroll-into-view is deprecated; use `agent-tui scroll` or `agent-tui press` instead. It will be deprecated in the next major release.",
        ))
        .stderr(predicate::str::contains("Daemon is not running").not())
        .get_output()
        .clone();

    let stdout_text = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout_text.contains("deprecated"),
        "deprecation notice must not be written to JSON stdout: {stdout_text}"
    );
    let parsed: Value = serde_json::from_slice(&output.stdout).expect("valid JSON stdout");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["selector"], "@details");
    assert_eq!(parsed["scrolled"], false);
}

#[test]
fn legacy_scroll_into_view_unsupported_semantics_return_compat_error() {
    let env = StandaloneEnv::new();

    env.run(&["scroll-into-view", "@details", "--center"])
        .code(64)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "Legacy scroll-into-view option `--center` for selector `@details` is not supported.",
        ))
        .stderr(predicate::str::contains(
            "Use `agent-tui scroll <direction> [amount]` or `agent-tui press`.",
        ))
        .stderr(predicate::str::contains("Daemon is not running").not())
        .stderr(predicate::str::contains("unrecognized subcommand").not());
}

#[test]
fn wait_assert_returns_non_zero_on_timeout() {
    let harness = TestHarness::new();
    harness.set_response(
        "wait",
        MockResponse::Success(json!({
            "found": false,
            "elapsed_ms": 30000
        })),
    );
    harness.run(&["wait", "--assert", "never"]).code(75);
    harness.assert_method_called("wait");
}

#[test]
fn standalone_version_env_and_completions_contract() {
    let env = StandaloneEnv::new();

    env.run(&["--format", "json", "version"])
        .success()
        .stdout(predicate::str::contains("\"cli_version\""));
    env.run(&["--format", "json", "env"])
        .success()
        .stdout(predicate::str::contains("\"environment\""));

    for shell in ["bash", "zsh", "fish", "elvish"] {
        env.run(&["completions", "--print", shell])
            .success()
            .stdout(predicate::str::is_empty().not());
    }

    let printed = env
        .run(&["--format", "json", "completions", "--print", "bash"])
        .success()
        .get_output()
        .stdout
        .clone();
    let printed: Value = serde_json::from_slice(&printed).expect("valid completions json");
    assert_eq!(printed["action"], "print");
    assert_eq!(printed["shell"], "bash");
    assert!(
        printed["script"]
            .as_str()
            .is_some_and(|script| !script.is_empty())
    );

    let status = env
        .run(&["--format", "json", "completions", "bash"])
        .success()
        .get_output()
        .stdout
        .clone();
    let status: Value = serde_json::from_slice(&status).expect("valid completions status json");
    assert_eq!(status["action"], "status");
    assert_eq!(status["shell"], "bash");
    assert!(status["install_supported"].is_boolean());

    env.run(&["completions", "--print", "bash", "--install"])
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn standalone_completions_json_reports_shell_detection_failures_structurally() {
    let env = StandaloneEnv::new();
    let output = env
        .cli_command()
        .env_remove("SHELL")
        .args(["--format", "json", "completions", "--no-input"])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let parsed: Value = serde_json::from_slice(&output).expect("valid completions error json");
    assert_eq!(parsed["category"], "invalid_input");
    assert_eq!(parsed["code"], 64);
    assert!(
        parsed["message"]
            .as_str()
            .is_some_and(|message| message.contains("Shell not detected"))
    );
    assert_eq!(
        parsed["context"]["supported_shells"],
        json!(["bash", "zsh", "fish", "elvish"])
    );
}

#[test]
fn standalone_version_uses_local_daemon_when_ws_transport_selected() {
    let env = StandaloneEnv::new();
    env.run(&["daemon", "start"]).success();

    let output = env
        .cli_command()
        .env("AGENT_TUI_TRANSPORT", "ws")
        .env("AGENT_TUI_WS_ADDR", "ws://127.0.0.1:9/ws")
        .args(["--format", "json", "version"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: Value = serde_json::from_slice(&output).expect("valid version json");
    assert_ne!(parsed["daemon_version"], "unavailable");
    assert!(
        parsed.get("daemon_error").is_none(),
        "version should ignore remote ws transport for local daemon inspection"
    );

    env.run(&["daemon", "stop", "--force", "--yes"]).success();
}

#[test]
fn standalone_daemon_commands_contract() {
    let env = StandaloneEnv::new();

    env.run(&["daemon", "start"])
        .success()
        .stdout(predicate::str::contains("Daemon started"));

    env.run(&["--format", "json", "daemon", "start"])
        .success()
        .stdout(predicate::str::contains("\"running\": true"));

    env.run(&["--format", "json", "daemon", "status"])
        .success()
        .stdout(predicate::str::contains("\"running\": true"));

    env.run(&["daemon", "stop", "--force", "--yes"])
        .success()
        .stdout(
            predicate::str::contains("Daemon stopped")
                .or(predicate::str::contains("already stopped")),
        );

    env.run(&["daemon", "restart", "--yes"])
        .success()
        .stdout(predicate::str::contains("Daemon restarted"));

    env.run(&["daemon", "stop", "--force", "--yes"]).success();
}

#[test]
fn standalone_daemon_stop_uses_local_daemon_when_ws_transport_selected() {
    let env = StandaloneEnv::new();
    env.run(&["daemon", "start"]).success();

    env.cli_command()
        .env("AGENT_TUI_TRANSPORT", "ws")
        .env("AGENT_TUI_WS_ADDR", "ws://127.0.0.1:9/ws")
        .args(["daemon", "stop", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Daemon stopped"));

    env.run(&["daemon", "status"])
        .code(3)
        .stdout(predicate::str::contains("Daemon is not running"));
}

#[test]
fn live_start_alias_contract() {
    let env = StandaloneEnv::new();
    env.write_ws_state();

    env.cli_command()
        .env("AGENT_TUI_TRANSPORT", "ws")
        .env("AGENT_TUI_WS_ADDR", "ws://127.0.0.1:9/ws")
        .env("AGENT_TUI_WS_STATE", &env.ws_state_path)
        .args(["--format", "json", "live", "start"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"running\": true"));

    env.cli_command()
        .env("AGENT_TUI_TRANSPORT", "ws")
        .env("AGENT_TUI_WS_ADDR", "ws://127.0.0.1:9/ws")
        .env("AGENT_TUI_WS_STATE", &env.ws_state_path)
        .args(["--format", "json", "live", "info"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"running\": true"));
}

#[test]
fn standalone_live_status_and_stop_contract() {
    let env = StandaloneEnv::new();
    env.write_ws_state();

    env.run(&["--format", "json", "live", "status"])
        .success()
        .stdout(predicate::str::contains("\"running\": true"));

    env.run(&["live", "stop"])
        .success()
        .stdout(predicate::str::contains(
            "Live preview is served by the daemon; run 'agent-tui daemon stop --yes' to stop.",
        ));
}

#[test]
fn standalone_live_stop_ignores_legacy_ui_state_without_identity() {
    let env = StandaloneEnv::new();
    fs::write(
        &env.ui_state_path,
        r#"{"pid":1,"url":"http://127.0.0.1:43210/ui","port":43210}"#,
    )
    .expect("write ui state");

    env.run(&["live", "stop"])
        .success()
        .stdout(predicate::str::contains("UI server is not running."));
}

#[test]
fn help_entrypoints_remain_valid() {
    let env = StandaloneEnv::new();
    let help_cases: &[&[&str]] = &[
        &["--help"],
        &["run", "--help"],
        &["screenshot", "--help"],
        &["resize", "--help"],
        &["restart", "--help"],
        &["press", "--help"],
        &["action", "--help"],
        &["type", "--help"],
        &["scroll-into-view", "--help"],
        &["wait", "--help"],
        &["kill", "--help"],
        &["sessions", "--help"],
        &["sessions", "help"],
        &["live", "--help"],
        &["live", "help"],
        &["daemon", "--help"],
        &["daemon", "help"],
        &["version", "--help"],
        &["env", "--help"],
        &["completions", "--help"],
    ];

    for args in help_cases {
        env.run(args).success();
    }
}

#[test]
fn leaf_help_examples_contract() {
    let env = StandaloneEnv::new();
    let example_cases: &[&[&str]] = &[
        &["sessions", "list", "--help"],
        &["sessions", "show", "--help"],
        &["sessions", "attach", "--help"],
        &["sessions", "switch", "--help"],
        &["sessions", "cleanup", "--help"],
        &["live", "start", "--help"],
        &["live", "stop", "--help"],
        &["live", "status", "--help"],
        &["daemon", "--help"],
        &["daemon", "status", "--help"],
        &["daemon", "restart", "--help"],
    ];

    for args in example_cases {
        env.run(args)
            .success()
            .stdout(predicate::str::contains("EXAMPLES:"));
    }
}

#[test]
fn invalid_flag_errors_include_example_invocation() {
    let env = StandaloneEnv::new();
    env.run(&["version", "--definitely-not-a-real-flag-xyz123"])
        .failure()
        .code(2)
        .stderr(predicate::str::contains("Example:"))
        .stderr(predicate::str::contains("agent-tui version --help"));
}

#[test]
fn global_flags_contract() {
    let harness = TestHarness::new();

    harness.clear_requests();
    harness
        .run(&["--session", "custom-session", "run", "bash"])
        .success();
    harness.assert_method_called_with(
        "spawn",
        json!({
            "session": "custom-session"
        }),
    );

    let output = harness
        .run(&["--format", "text", "--json", "version"])
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Value = serde_json::from_slice(&output).expect("valid JSON output");
    assert!(parsed.get("cli_version").is_some());

    setup_running_session(&harness);
    let stdout = String::from_utf8_lossy(
        &harness
            .run(&["--no-color", "sessions"])
            .success()
            .get_output()
            .stdout,
    )
    .to_string();
    assert!(
        !stdout.contains("\u{1b}["),
        "no-color output must not include ANSI escapes"
    );
}
