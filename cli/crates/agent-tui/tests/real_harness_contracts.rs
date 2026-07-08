#![expect(
    clippy::expect_used,
    reason = "Test-only assertions use expect for clarity."
)]

mod common;

mod real_harness_contracts {
    use crate::common::real_test_harness::DaemonOutputCapture;
    use crate::common::real_test_harness::OutputStream;
    use crate::common::real_test_harness::format_command_failure;
    use crate::common::real_test_harness::runtime_env_vars;
    use std::collections::BTreeMap;
    use std::path::Path;
    use std::process::ExitStatus;
    use std::process::Output;
    use tempfile::TempDir;

    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;

    #[test]
    fn command_failure_diagnostic_includes_context_and_daemon_output() {
        let daemon_output = DaemonOutputCapture::default();
        daemon_output.push(OutputStream::Stdout, b"daemon booting\n");
        daemon_output.push(OutputStream::Stderr, b"daemon warning\n");

        let output = Output {
            status: failing_status(),
            stdout: b"command stdout\n".to_vec(),
            stderr: b"command stderr\n".to_vec(),
        };

        let diagnostic = format_command_failure(
            Path::new("/tmp/agent-tui-test/agent-tui.sock"),
            &["--format", "json", "sessions"],
            &output,
            &daemon_output,
        );

        assert!(diagnostic.contains("command: agent-tui --format json sessions"));
        assert!(diagnostic.contains("status: exit status: 1"));
        assert!(diagnostic.contains("command stdout"));
        assert!(diagnostic.contains("command stderr"));
        assert!(diagnostic.contains("/tmp/agent-tui-test/agent-tui.sock"));
        assert!(diagnostic.contains("daemon booting"));
        assert!(diagnostic.contains("daemon warning"));
    }

    #[test]
    fn runtime_env_vars_are_scoped_to_harness_directory() {
        let first = TempDir::new().expect("first temp dir");
        let second = TempDir::new().expect("second temp dir");

        let first_env = env_map(runtime_env_vars(
            first.path(),
            &first.path().join("agent-tui.sock"),
        ));
        let second_env = env_map(runtime_env_vars(
            second.path(),
            &second.path().join("agent-tui.sock"),
        ));

        for key in [
            "AGENT_TUI_SOCKET",
            "AGENT_TUI_SESSION_STORE",
            "AGENT_TUI_WS_STATE",
            "AGENT_TUI_UI_STATE",
        ] {
            let first_value = first_env.get(key).expect("first env value");
            let second_value = second_env.get(key).expect("second env value");
            assert!(first_value.starts_with(&first.path().to_string_lossy().to_string()));
            assert!(second_value.starts_with(&second.path().to_string_lossy().to_string()));
            assert_ne!(first_value, second_value);
        }
    }

    fn env_map(env_vars: Vec<(String, String)>) -> BTreeMap<String, String> {
        env_vars.into_iter().collect()
    }

    #[cfg(unix)]
    fn failing_status() -> ExitStatus {
        ExitStatus::from_raw(1 << 8)
    }
}
