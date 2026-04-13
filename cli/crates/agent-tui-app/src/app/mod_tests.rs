use super::*;

mod daemon_standalone_tests {
    use super::*;
    use crate::app::commands::Cli;
    use crate::app::commands::Commands;
    use crate::app::commands::DaemonCommand;
    use crate::app::commands::LiveCommand;
    use crate::app::commands::OutputFormat;
    use crate::test_support::env_lock;
    use std::env;
    use std::path::Path;
    use tempfile::TempDir;

    struct EnvVarGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prev = env::var(key).ok();
            // SAFETY: Test-only environment override.
            unsafe {
                env::set_var(key, value);
            }
            Self { key, prev }
        }

        fn set_path(key: &'static str, value: &Path) -> Self {
            let prev = env::var(key).ok();
            // SAFETY: Test-only environment override.
            unsafe {
                env::set_var(key, value);
            }
            Self { key, prev }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(prev) = self.prev.take() {
                // SAFETY: Test-only environment restoration.
                unsafe {
                    env::set_var(self.key, prev);
                }
            } else {
                // SAFETY: Test-only environment cleanup.
                unsafe {
                    env::remove_var(self.key);
                }
            }
        }
    }

    fn make_cli(command: Commands) -> Cli {
        Cli {
            command,
            session: None,
            format: OutputFormat::Text,
            json: false,
            no_color: true,
            no_input: false,
        }
    }

    #[test]
    fn handle_standalone_commands_routes_daemon_stop() {
        let _env_lock = env_lock();
        // Isolate from any real daemon by pointing socket to a temp path.
        let tmp = TempDir::new().expect("temp dir");
        let socket_path = tmp.path().join("agent-tui-test.sock");
        let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);

        let app = Application::new();
        let cli = make_cli(Commands::Daemon(DaemonCommand::Stop {
            force: false,
            dry_run: false,
            yes: true,
        }));

        // When daemon is not running, should succeed (idempotent semantics)
        // The result should be Ok(true), indicating the command was handled
        let result = app.handle_standalone_commands(&cli);
        assert!(
            result.is_ok(),
            "daemon stop should succeed when daemon not running (idempotent)"
        );
        assert!(
            matches!(result, Ok(true)),
            "daemon stop should be handled as standalone"
        );
    }

    #[test]
    fn handle_standalone_commands_routes_daemon_start() {
        let _env_lock = env_lock();
        // Isolate from any real daemon by pointing socket to a temp path.
        let tmp = TempDir::new().expect("temp dir");
        let socket_path = tmp.path().join("agent-tui-test.sock");
        let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);

        // Use stub to prevent spawning a real daemon process.
        crate::infra::ipc::transport::USE_DAEMON_START_STUB
            .store(true, std::sync::atomic::Ordering::SeqCst);

        let app = Application::new();
        let cli = make_cli(Commands::Daemon(DaemonCommand::Start {}));

        let result = app.handle_standalone_commands(&cli);
        // Error is acceptable (daemon may fail to start), but it was handled
        if let Ok(handled) = result {
            assert!(handled, "daemon start should be handled as standalone");
        }

        // Clean up stub state.
        crate::infra::ipc::transport::USE_DAEMON_START_STUB
            .store(false, std::sync::atomic::Ordering::SeqCst);
        crate::infra::ipc::transport::clear_test_listener();
    }

    #[test]
    fn handle_standalone_commands_routes_daemon_restart() {
        let _env_lock = env_lock();
        // Isolate from any real daemon by pointing socket to a temp path.
        let tmp = TempDir::new().expect("temp dir");
        let socket_path = tmp.path().join("agent-tui-test.sock");
        let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);

        // Use stub to prevent spawning a real daemon process.
        crate::infra::ipc::transport::USE_DAEMON_START_STUB
            .store(true, std::sync::atomic::Ordering::SeqCst);

        let app = Application::new();
        let cli = make_cli(Commands::Daemon(DaemonCommand::Restart {
            dry_run: false,
            yes: true,
        }));

        // Restart should be handled as standalone (may error if start fails)
        let result = app.handle_standalone_commands(&cli);
        if let Ok(handled) = result {
            assert!(handled, "daemon restart should be handled as standalone");
        }

        // Clean up stub state.
        crate::infra::ipc::transport::USE_DAEMON_START_STUB
            .store(false, std::sync::atomic::Ordering::SeqCst);
        crate::infra::ipc::transport::clear_test_listener();
    }

    #[test]
    fn handle_standalone_commands_routes_daemon_status_without_autostart() {
        let _env_lock = env_lock();
        let tmp = TempDir::new().expect("temp dir");
        let socket_path = tmp.path().join("agent-tui-test.sock");
        let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);

        let app = Application::new();
        let cli = make_cli(Commands::Daemon(DaemonCommand::Status));

        let result = app.handle_standalone_commands(&cli);
        let err = result.expect_err("daemon status should report not running");
        assert!(
            err.downcast_ref::<DaemonNotRunningError>().is_some(),
            "daemon status should map to daemon-not-running exit handling"
        );
        assert!(
            !socket_path.exists(),
            "daemon status must not autostart daemon or create socket"
        );
    }

    #[test]
    fn handle_standalone_commands_routes_daemon_status_locally_when_ws_transport_selected() {
        let _env_lock = env_lock();
        let tmp = TempDir::new().expect("temp dir");
        let socket_path = tmp.path().join("agent-tui-test.sock");
        let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);
        let _transport_guard = EnvVarGuard::set("AGENT_TUI_TRANSPORT", "ws");
        let _ws_addr_guard = EnvVarGuard::set("AGENT_TUI_WS_ADDR", "ws://127.0.0.1:9/ws");

        let app = Application::new();
        let cli = make_cli(Commands::Daemon(DaemonCommand::Status));

        let result = app.handle_standalone_commands(&cli);
        let err = result.expect_err("daemon status should still inspect the local daemon");
        assert!(
            err.downcast_ref::<DaemonNotRunningError>().is_some(),
            "daemon status should ignore websocket transport selection"
        );
        assert!(
            !socket_path.exists(),
            "daemon status must not create a local socket when daemon is not running"
        );
    }

    #[test]
    fn handle_standalone_commands_routes_live_status_without_autostart() {
        let _env_lock = env_lock();
        let tmp = TempDir::new().expect("temp dir");
        let socket_path = tmp.path().join("agent-tui-test.sock");
        let ws_state = tmp.path().join("api.json");
        let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);
        let _ws_guard = EnvVarGuard::set_path("AGENT_TUI_WS_STATE", &ws_state);

        let app = Application::new();
        let cli = make_cli(Commands::Live {
            command: Some(LiveCommand::Status),
        });

        let result = app.handle_standalone_commands(&cli);
        assert!(result.is_ok(), "live status should be handled");
        assert!(
            matches!(result, Ok(true)),
            "live status should be standalone"
        );
        assert!(
            !socket_path.exists(),
            "live status must not autostart daemon or create socket"
        );
    }

    #[test]
    fn handle_standalone_commands_routes_live_start_locally_when_ws_transport_selected() {
        let _env_lock = env_lock();
        let tmp = TempDir::new().expect("temp dir");
        let socket_path = tmp.path().join("agent-tui-test.sock");
        let ws_state = tmp.path().join("api.json");
        std::fs::write(
            &ws_state,
            format!(
                r#"{{"pid":{},"ws_url":"ws://127.0.0.1:43210/ws","ui_url":"http://127.0.0.1:43210/ui","listen":"127.0.0.1:43210","started_at":1735689600}}"#,
                std::process::id()
            ),
        )
        .expect("write ws state");
        let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);
        let _ws_guard = EnvVarGuard::set_path("AGENT_TUI_WS_STATE", &ws_state);
        let _transport_guard = EnvVarGuard::set("AGENT_TUI_TRANSPORT", "ws");
        let _ws_addr_guard = EnvVarGuard::set("AGENT_TUI_WS_ADDR", "ws://127.0.0.1:9/ws");

        let app = Application::new();
        let cli = make_cli(Commands::Live { command: None });

        let result = app.handle_standalone_commands(&cli);
        assert!(result.is_ok(), "live start should be handled");
        assert!(
            matches!(result, Ok(true)),
            "live start should be standalone"
        );
        assert!(
            !socket_path.exists(),
            "live start should not use the selected remote websocket transport"
        );
    }

    #[test]
    fn handle_standalone_commands_routes_live_stop_without_autostart() {
        let _env_lock = env_lock();
        let tmp = TempDir::new().expect("temp dir");
        let socket_path = tmp.path().join("agent-tui-test.sock");
        let ws_state = tmp.path().join("api.json");
        let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);
        let _ws_guard = EnvVarGuard::set_path("AGENT_TUI_WS_STATE", &ws_state);

        let app = Application::new();
        let cli = make_cli(Commands::Live {
            command: Some(LiveCommand::Stop),
        });

        let result = app.handle_standalone_commands(&cli);
        assert!(result.is_ok(), "live stop should be handled");
        assert!(matches!(result, Ok(true)), "live stop should be standalone");
        assert!(
            !socket_path.exists(),
            "live stop must not autostart daemon or create socket"
        );
    }

    #[test]
    fn handle_standalone_commands_daemon_stop_stale_lock_is_idempotent() {
        let _env_lock = env_lock();
        let tmp = TempDir::new().expect("temp dir");
        let socket_path = tmp.path().join("agent-tui-test.sock");
        let lock_path = socket_path.with_extension("lock");
        std::fs::write(&lock_path, "999999").expect("write stale lock");
        let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);

        let app = Application::new();
        let cli = make_cli(Commands::Daemon(DaemonCommand::Stop {
            force: false,
            dry_run: false,
            yes: true,
        }));

        let result = app.handle_standalone_commands(&cli);
        assert!(
            matches!(result, Ok(true)),
            "daemon stop should be idempotent with stale lock"
        );
        assert!(
            !lock_path.exists(),
            "stale lock file should be cleaned after stop"
        );
    }

    #[test]
    fn handle_standalone_commands_daemon_force_stop_stale_lock_is_idempotent() {
        let _env_lock = env_lock();
        let tmp = TempDir::new().expect("temp dir");
        let socket_path = tmp.path().join("agent-tui-test.sock");
        let lock_path = socket_path.with_extension("lock");
        std::fs::write(&lock_path, "999999").expect("write stale lock");
        let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);

        let app = Application::new();
        let cli = make_cli(Commands::Daemon(DaemonCommand::Stop {
            force: true,
            dry_run: false,
            yes: true,
        }));

        let result = app.handle_standalone_commands(&cli);
        assert!(
            matches!(result, Ok(true)),
            "daemon stop --force should be idempotent with stale lock"
        );
        assert!(
            !lock_path.exists(),
            "stale lock file should be cleaned after forced stop"
        );
    }

    #[test]
    fn handle_standalone_commands_daemon_stop_removes_stale_ws_state() {
        let _env_lock = env_lock();
        let tmp = TempDir::new().expect("temp dir");
        let socket_path = tmp.path().join("agent-tui-test.sock");
        let ws_state = tmp.path().join("api.json");
        std::fs::write(&ws_state, r#"{"pid":1}"#).expect("write ws state");
        let _socket_guard = EnvVarGuard::set_path("AGENT_TUI_SOCKET", &socket_path);
        let _ws_guard = EnvVarGuard::set_path("AGENT_TUI_WS_STATE", &ws_state);

        let app = Application::new();
        let cli = make_cli(Commands::Daemon(DaemonCommand::Stop {
            force: true,
            dry_run: false,
            yes: true,
        }));

        let result = app.handle_standalone_commands(&cli);
        assert!(
            matches!(result, Ok(true)),
            "daemon stop should succeed when daemon is already stopped"
        );
        assert!(
            !ws_state.exists(),
            "WS state file should be cleaned on successful stop path"
        );
    }

    #[test]
    fn handle_error_returns_not_running_exit_code() {
        let app = Application::new();
        let exit_code = app.handle_error(anyhow::Error::new(DaemonNotRunningError));
        assert_eq!(exit_code, exit_codes::NOT_RUNNING);
    }

    #[test]
    fn daemon_start_requests_foreground_accepts_truthy_values() {
        let _env_lock = env_lock();
        let _foreground_guard = EnvVarGuard::set("AGENT_TUI_DAEMON_FOREGROUND", "true");
        assert!(daemon_start_requests_foreground());
    }
}
