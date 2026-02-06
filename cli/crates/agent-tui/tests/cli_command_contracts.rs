#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only assertions use unwrap/expect for clarity."
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

    fn write_api_state(&self) {
        let data = json!({
            "pid": std::process::id(),
            "ws_url": "ws://127.0.0.1:43210/ws",
            "ui_url": "http://127.0.0.1:43210/ui",
            "listen": "127.0.0.1:43210",
            "started_at": 1735689600
        });
        fs::write(
            &self.ws_state_path,
            serde_json::to_string_pretty(&data).expect("serialize api state"),
        )
        .expect("write api state");
    }

    fn stop_daemon_best_effort(&self) {
        let _ = StdCommand::new(assert_cmd::cargo::cargo_bin!("agent-tui"))
            .env("AGENT_TUI_SOCKET", &self.socket_path)
            .env("AGENT_TUI_WS_STATE", &self.ws_state_path)
            .args(["daemon", "stop", "--force"])
            .output();
    }
}

impl Drop for StandaloneEnv {
    fn drop(&mut self) {
        self.stop_daemon_best_effort();
    }
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
        "daemon start".to_string(),
        "daemon stop".to_string(),
        "env".to_string(),
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
            args: &["restart"],
            expected_method: "restart",
            setup: no_setup,
        },
        CommandCase {
            args: &["press", "Enter"],
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
            args: &["wait", "done"],
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
            args: &["kill"],
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
            args: &["sessions", "cleanup"],
            expected_method: "sessions",
            setup: setup_mixed_sessions,
        },
        CommandCase {
            args: &["sessions", "cleanup", "--all"],
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
    harness.run(&["sessions", "cleanup"]).success();
    harness.assert_method_called("kill");
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
    harness.run(&["wait", "--assert", "never"]).code(1);
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

    for shell in ["bash", "zsh", "fish", "powershell", "elvish"] {
        env.run(&["completions", "--print", shell])
            .success()
            .stdout(predicate::str::is_empty().not());
    }

    env.run(&["completions", "--print", "bash", "--install"])
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn standalone_daemon_commands_contract() {
    let env = StandaloneEnv::new();

    env.run(&["daemon", "start"])
        .success()
        .stdout(predicate::str::contains("Daemon started in background"));

    env.run(&["daemon", "stop", "--force"]).success().stdout(
        predicate::str::contains("Daemon stopped").or(predicate::str::contains("already stopped")),
    );

    env.run(&["daemon", "restart"])
        .success()
        .stdout(predicate::str::contains("Daemon restarted"));

    env.run(&["daemon", "stop", "--force"]).success();
}

#[test]
fn live_start_alias_and_deprecated_flags_contract() {
    let harness = TestHarness::new();
    let env = StandaloneEnv::new();
    env.write_api_state();

    harness
        .cli_command()
        .env("AGENT_TUI_WS_STATE", &env.ws_state_path)
        .args([
            "--format",
            "json",
            "live",
            "start",
            "--listen",
            "127.0.0.1:0",
            "--allow-remote",
            "--max-viewers",
            "12",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "Live preview is now served by the daemon WebSocket server",
        ))
        .stdout(predicate::str::contains("\"running\": true"));

    harness
        .cli_command()
        .env("AGENT_TUI_WS_STATE", &env.ws_state_path)
        .args(["--format", "json", "live", "info"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"running\": true"));
}

#[test]
fn standalone_live_status_and_stop_contract() {
    let env = StandaloneEnv::new();
    env.write_api_state();

    env.run(&["--format", "json", "live", "status"])
        .success()
        .stdout(predicate::str::contains("\"running\": true"));

    env.run(&["live", "stop"])
        .success()
        .stdout(predicate::str::contains(
            "Live preview is served by the daemon; run 'agent-tui daemon stop' to stop.",
        ));
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
        &["type", "--help"],
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
