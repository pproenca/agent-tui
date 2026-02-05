#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only assertions use unwrap/expect for clarity."
)]

//! End-to-end system tests.

mod common;

#[cfg(feature = "slow-tests")]
mod e2e {
    use crate::common::InteractivePtyRunner;
    use crate::common::RealTestHarness;
    use predicates::prelude::*;
    use serde_json::Value;
    use std::path::PathBuf;
    use std::time::Duration;

    fn run_json(harness: &RealTestHarness, args: &[&str]) -> Value {
        let output = harness
            .cli_command()
            .args(args)
            .output()
            .expect("failed to execute command");
        assert!(
            output.status.success(),
            "command failed: args={args:?}, status={:?}, stdout={}, stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).expect("command output must be valid JSON")
    }

    fn spawn_session(harness: &RealTestHarness, command: &str) -> String {
        let output = harness
            .cli_command()
            .args(["--format", "json", "run", command])
            .output()
            .expect("failed to run session");
        assert!(
            output.status.success(),
            "run command failed: stdout={}, stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let value: Value = serde_json::from_slice(&output.stdout).expect("run output must be JSON");
        value["session_id"]
            .as_str()
            .expect("run output must include session_id")
            .to_string()
    }

    #[test]
    fn e2e_core_runtime_commands_work_end_to_end() {
        let mut harness = RealTestHarness::new();

        harness
            .run(&["--no-color", "version"])
            .success()
            .stdout(predicate::str::contains("CLI version:"))
            .stdout(predicate::str::contains("Daemon version:"))
            .stdout(predicate::str::contains("unavailable").not());

        harness
            .run(&["--no-color", "sessions"])
            .success()
            .stdout(predicate::str::contains("No active sessions"));

        let session_1 = spawn_session(&harness, "bash");
        let session_2 = spawn_session(&harness, "sh");

        let sessions = run_json(&harness, &["--format", "json", "sessions"]);
        let listed = sessions["sessions"]
            .as_array()
            .expect("sessions array expected")
            .iter()
            .filter_map(|s| s["id"].as_str())
            .collect::<Vec<_>>();
        assert!(listed.iter().any(|id| *id == session_1));
        assert!(listed.iter().any(|id| *id == session_2));

        harness
            .run(&["--session", &session_1, "type", "echo core-runtime-marker"])
            .success();
        harness
            .run(&["--session", &session_1, "press", "Enter"])
            .success();
        harness
            .run(&[
                "--session",
                &session_1,
                "wait",
                "--assert",
                "core-runtime-marker",
            ])
            .success();
        harness
            .run(&["--session", &session_1, "screenshot", "--strip-ansi"])
            .success()
            .stdout(predicate::str::contains("core-runtime-marker"));

        harness
            .run(&["--no-color", "sessions", "show", &session_2])
            .success()
            .stdout(predicate::str::contains(&session_2));

        harness
            .run(&["--no-color", "sessions", "switch", &session_1])
            .success()
            .stdout(predicate::str::contains(&session_1));

        let switched = run_json(&harness, &["--format", "json", "sessions"]);
        assert_eq!(
            switched["active_session"].as_str(),
            Some(session_1.as_str())
        );

        let live_status = run_json(&harness, &["--format", "json", "live", "status"]);
        assert_eq!(live_status["running"].as_bool(), Some(true));

        harness.run(&["--session", &session_1, "kill"]).success();
        harness.run(&["--session", &session_2, "kill"]).success();

        harness
            .run(&["--no-color", "daemon", "stop"])
            .success()
            .stdout(
                predicate::str::contains("Daemon stopped")
                    .or(predicate::str::contains("already stopped")),
            );

        harness.wait_for_exit(Duration::from_secs(3));

        harness
            .run(&["--no-color", "sessions"])
            .code(69)
            .stderr(predicate::str::contains("Daemon not running"));

        harness.stop();
    }

    #[test]
    fn e2e_sessions_attach_interactive_default_detach_keys() {
        let harness = RealTestHarness::new();
        let session_id = spawn_session(&harness, "bash");

        harness
            .run(&["--session", &session_id, "type", "echo attach-default"])
            .success();
        harness
            .run(&["--session", &session_id, "press", "Enter"])
            .success();

        let binary = PathBuf::from(assert_cmd::cargo::cargo_bin!("agent-tui"));
        let mut env_vars = harness.env_vars();
        env_vars.push(("NO_COLOR".to_string(), "1".to_string()));

        let args = vec!["--session", session_id.as_str(), "sessions", "attach"];
        let mut runner =
            InteractivePtyRunner::spawn(&binary, &args, &env_vars).expect("spawn attach PTY");
        runner
            .read_until_contains("Connected!", Duration::from_secs(5))
            .expect("attach should print Connected");
        runner
            .read_until_contains("Press Ctrl-P Ctrl-Q to detach.", Duration::from_secs(5))
            .expect("attach should show default detach keys");
        runner.send_bytes(&[0x10]).expect("send Ctrl-P");
        runner.send_bytes(&[0x11]).expect("send Ctrl-Q");
        let status = runner
            .wait_for_exit(Duration::from_secs(6))
            .expect("attach command should exit");
        assert!(status.success(), "attach must exit successfully");
        assert!(
            runner.output_as_string().contains("Detached from session"),
            "attach output must include detach confirmation"
        );

        harness.run(&["--session", &session_id, "kill"]).success();
    }

    #[test]
    fn e2e_sessions_attach_interactive_custom_detach_keys() {
        let harness = RealTestHarness::new();
        let session_id = spawn_session(&harness, "bash");

        let binary = PathBuf::from(assert_cmd::cargo::cargo_bin!("agent-tui"));
        let mut env_vars = harness.env_vars();
        env_vars.push(("NO_COLOR".to_string(), "1".to_string()));

        let args = vec![
            "--session",
            session_id.as_str(),
            "sessions",
            "attach",
            "--detach-keys",
            "ctrl-p",
        ];
        let mut runner =
            InteractivePtyRunner::spawn(&binary, &args, &env_vars).expect("spawn attach PTY");
        runner
            .read_until_contains("Connected!", Duration::from_secs(5))
            .expect("attach should print Connected");
        runner
            .read_until_contains("Press Ctrl-P to detach.", Duration::from_secs(5))
            .expect("attach should show custom detach keys");
        runner.send_bytes(&[0x10]).expect("send Ctrl-P");
        let status = runner
            .wait_for_exit(Duration::from_secs(6))
            .expect("attach command should exit");
        assert!(status.success(), "attach must exit successfully");
        assert!(
            runner.output_as_string().contains("Detached from session"),
            "attach output must include detach confirmation"
        );

        harness.run(&["--session", &session_id, "kill"]).success();
    }
}
