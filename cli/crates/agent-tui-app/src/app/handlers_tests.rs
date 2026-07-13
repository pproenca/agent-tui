use super::*;
use crate::adapters::presenter::Presenter;
use crate::app::commands::OutputFormat;
use crate::infra::ipc::MockClient;
use crate::infra::ipc::ProcessStatus;
use crate::test_support::env_lock;
use std::fs;
use std::sync::Mutex;
use tempfile::TempDir;

struct EnvVarGuard {
    key: &'static str,
    prev: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: impl Into<String>) -> Self {
        let prev = std::env::var(key).ok();
        // SAFETY: test-only environment mutation under env_lock.
        unsafe {
            std::env::set_var(key, value.into());
        }
        Self { key, prev }
    }

    fn remove(key: &'static str) -> Self {
        let prev = std::env::var(key).ok();
        // SAFETY: test-only environment mutation under env_lock.
        unsafe {
            std::env::remove_var(key);
        }
        Self { key, prev }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        // SAFETY: test-only environment restoration under env_lock.
        unsafe {
            if let Some(prev) = self.prev.take() {
                std::env::set_var(self.key, prev);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }
}

struct StopUiController {
    status: Mutex<ProcessStatus>,
    signals: Mutex<Vec<Signal>>,
    kill_on_term: bool,
    kill_on_kill: bool,
    started_at: Option<u64>,
}

impl StopUiController {
    fn new(
        status: ProcessStatus,
        kill_on_term: bool,
        kill_on_kill: bool,
        started_at: Option<u64>,
    ) -> Self {
        Self {
            status: Mutex::new(status),
            signals: Mutex::new(Vec::new()),
            kill_on_term,
            kill_on_kill,
            started_at,
        }
    }

    fn signals(&self) -> Vec<Signal> {
        self.signals
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl ProcessController for StopUiController {
    fn check_process(&self, _pid: u32) -> std::io::Result<crate::infra::ipc::ProcessStatus> {
        Ok(*self
            .status
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner))
    }

    fn send_signal(&self, _pid: u32, signal: Signal) -> std::io::Result<()> {
        self.signals
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(signal);
        let should_stop = match signal {
            Signal::Term => self.kill_on_term,
            Signal::Kill => self.kill_on_kill,
        };
        if should_stop {
            *self
                .status
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = ProcessStatus::NotFound;
        }
        Ok(())
    }

    fn process_identity(&self, pid: u32) -> std::io::Result<Option<ProcessIdentity>> {
        Ok(Some(ProcessIdentity {
            pid,
            started_at: self.started_at,
        }))
    }
}

#[test]
fn handler_context_selects_presenter_from_output_format() {
    let mut client = MockClient::new();
    let context = HandlerContext::new(&mut client, None, OutputFormat::Json, false);

    assert_eq!(context.presenter(), &Presenter::Json);
}

#[test]
fn handle_spawn_uses_invocation_cwd_for_local_transport_when_cwd_omitted() {
    let _env = env_lock();
    let _transport_guard = EnvVarGuard::set("AGENT_TUI_TRANSPORT", "unix");
    let temp_dir = TempDir::new_in("/tmp").expect("tempdir");
    let expected_cwd = fs::canonicalize(temp_dir.path()).expect("canonical temp dir");

    let mut client = MockClient::new();
    client.set_response(
        "spawn",
        serde_json::json!({
            "session_id": "session-1",
            "pid": 42
        }),
    );

    let mut ctx = HandlerContext {
        client: &mut client,
        session: None,
        format: OutputFormat::Json,
        no_input: false,
        presenter: Presenter::Json,
        current_dir_override: Some(expected_cwd.clone()),
    };

    handle_spawn(
        &mut ctx,
        "bash".to_string(),
        Vec::new(),
        None,
        None,
        120,
        40,
    )
    .expect("spawn should succeed");

    let params = client
        .last_call("spawn")
        .and_then(|(_, params)| params)
        .expect("spawn params");
    assert_eq!(params["cwd"], expected_cwd.display().to_string());
}

#[test]
fn handle_spawn_omits_default_cwd_for_websocket_transport() {
    let _env = env_lock();
    let _transport_guard = EnvVarGuard::set("AGENT_TUI_TRANSPORT", "ws");

    let mut client = MockClient::new();
    client.set_response(
        "spawn",
        serde_json::json!({
            "session_id": "session-1",
            "pid": 42
        }),
    );

    let mut ctx = HandlerContext {
        client: &mut client,
        session: None,
        format: OutputFormat::Json,
        no_input: false,
        presenter: Presenter::Json,
        current_dir_override: None,
    };

    handle_spawn(
        &mut ctx,
        "bash".to_string(),
        Vec::new(),
        None,
        None,
        120,
        40,
    )
    .expect("spawn should succeed");

    let params = client
        .last_call("spawn")
        .and_then(|(_, params)| params)
        .expect("spawn params");
    assert!(
        params.get("cwd").is_none(),
        "cwd should be omitted for ws transport"
    );
}

#[test]
fn handle_spawn_rejects_invalid_terminal_size_before_rpc() {
    let mut client = MockClient::new();
    let mut ctx = HandlerContext {
        client: &mut client,
        session: None,
        format: OutputFormat::Json,
        no_input: false,
        presenter: Presenter::Json,
        current_dir_override: None,
    };

    let err = handle_spawn(&mut ctx, "bash".to_string(), Vec::new(), None, None, 9, 40)
        .expect_err("invalid size should fail");
    let cli_error = err.downcast::<CliError>().expect("cli error");
    assert!(cli_error.message.contains("Invalid terminal size"));
    assert!(
        client.last_call("spawn").is_none(),
        "spawn RPC should not be sent for invalid sizes"
    );
}

#[test]
fn handle_spawn_forwards_env_overrides() {
    let mut client = MockClient::new();
    client.set_response(
        "spawn",
        serde_json::json!({
            "session_id": "session-1",
            "pid": 42
        }),
    );

    let mut ctx = HandlerContext {
        client: &mut client,
        session: None,
        format: OutputFormat::Json,
        no_input: false,
        presenter: Presenter::Json,
        current_dir_override: None,
    };

    let env = HashMap::from([
        ("FOO".to_string(), "bar".to_string()),
        ("EMPTY".to_string(), String::new()),
    ]);

    handle_spawn(
        &mut ctx,
        "bash".to_string(),
        Vec::new(),
        None,
        Some(env),
        120,
        40,
    )
    .expect("spawn should succeed");

    let params = client
        .last_call("spawn")
        .and_then(|(_, params)| params)
        .expect("spawn params");
    assert_eq!(
        params["env"],
        serde_json::json!({
            "FOO": "bar",
            "EMPTY": "",
        })
    );
}

#[test]
fn test_assert_condition_parsing_text() {
    let condition = "text:Submit";
    let (kind, value) = condition.split_once(':').expect("expected separator");
    assert_eq!(kind, "text");
    assert_eq!(value, "Submit");
}

#[test]
fn test_assert_condition_parsing_session() {
    let condition = "session:my-session";
    let (kind, value) = condition.split_once(':').expect("expected separator");
    assert_eq!(kind, "session");
    assert_eq!(value, "my-session");
}

#[test]
fn test_assert_condition_parsing_with_colon_in_value() {
    let condition = "text:URL: https://example.com";
    let (kind, value) = condition.split_once(':').expect("expected separator");
    assert_eq!(kind, "text");
    assert_eq!(value, "URL: https://example.com");
}

#[test]
fn test_assert_condition_parsing_invalid() {
    let condition = "invalid_format";
    assert!(condition.split_once(':').is_none());
}

#[test]
fn test_wait_condition_stable() {
    let params = WaitParams {
        stable: true,
        ..Default::default()
    };
    let cond = resolve_wait_condition(&params);
    assert_eq!(cond, Some("stable".to_string()));
}

#[test]
fn test_wait_condition_text_gone() {
    let params = WaitParams {
        text: Some("Loading...".to_string()),
        gone: true,
        ..Default::default()
    };
    let cond = resolve_wait_condition(&params);
    assert_eq!(cond, Some("text_gone".to_string()));
}

#[test]
fn test_wait_condition_none() {
    let params = WaitParams::default();
    let cond = resolve_wait_condition(&params);
    assert_eq!(cond, None);
}

#[cfg(unix)]
#[test]
fn daemon_start_standalone_recovers_from_stale_local_socket() {
    use std::os::unix::net::UnixListener;
    use std::sync::atomic::Ordering;

    let _env = env_lock();
    let temp = TempDir::new_in("/tmp").expect("tempdir");
    let socket = temp.path().join("daemon.sock");
    let _socket_guard = EnvVarGuard::set("AGENT_TUI_SOCKET", socket.display().to_string());

    let listener = UnixListener::bind(&socket).expect("bind stale socket");
    drop(listener);

    crate::infra::ipc::transport::USE_DAEMON_START_STUB.store(true, Ordering::SeqCst);
    let result = handle_daemon_start_standalone(OutputFormat::Json);
    crate::infra::ipc::transport::USE_DAEMON_START_STUB.store(false, Ordering::SeqCst);
    crate::infra::ipc::transport::clear_test_listener();

    assert!(
        result.is_ok(),
        "daemon start should recover from stale socket"
    );
}

#[test]
fn restart_daemon_core_errors_on_invalid_pid_lock() {
    let _env = env_lock();
    let temp = TempDir::new_in("/tmp").expect("tempdir");
    let socket = temp.path().join("daemon.sock");
    let lock = socket.with_extension("lock");
    let _socket_guard = EnvVarGuard::set("AGENT_TUI_SOCKET", socket.display().to_string());
    fs::write(&lock, "not-a-pid").expect("write invalid lock");

    let err = restart_daemon_core().expect_err("restart should fail on invalid pid lock");
    let client_error = err
        .downcast_ref::<ClientError>()
        .expect("restart error should preserve client error");

    assert!(matches!(
        client_error,
        ClientError::DaemonStateInvalid { path, message }
        if path.ends_with("daemon.lock")
            && message.contains("not a valid daemon identity payload")
    ));
}

#[test]
fn ws_state_path_ignores_deprecated_api_state_alias() {
    let _env = env_lock();
    let temp = TempDir::new_in("/tmp").expect("tempdir");
    let home = temp.path().join("home");
    fs::create_dir_all(&home).expect("create temp home");
    let _home_guard = EnvVarGuard::set("HOME", home.display().to_string());
    let _ws_state_guard = EnvVarGuard::remove("AGENT_TUI_WS_STATE");
    let _api_state_guard = EnvVarGuard::set("AGENT_TUI_API_STATE", "/tmp/deprecated-state.json");

    let expected = home.join(".agent-tui").join("api.json");
    assert_eq!(ws_state_path(), expected);
}

#[test]
fn stop_ui_server_escalates_to_sigkill_and_cleans_state() {
    let _env = env_lock();
    let temp = TempDir::new_in("/tmp").expect("tempdir");
    let state_path = temp.path().join("ui.json");
    let identity = crate::infra::ipc::current_process_identity();
    fs::write(
        &state_path,
        format!(
            r#"{{"pid":42,"url":"http://127.0.0.1:7777/ui","port":7777,"process_started_at":{}}}"#,
            identity.started_at.unwrap_or(42)
        ),
    )
    .expect("write ui state");
    let _state_guard = EnvVarGuard::set("AGENT_TUI_UI_STATE", state_path.display().to_string());
    let _external_guard = EnvVarGuard::set("AGENT_TUI_UI_URL", "");

    let controller = StopUiController::new(
        ProcessStatus::Running,
        false,
        true,
        identity.started_at.or(Some(42)),
    );
    let result = stop_ui_server_with_controller_and_timeouts(
        &controller,
        Duration::from_millis(5),
        Duration::from_millis(5),
    )
    .expect("ui stop should escalate and succeed");

    assert!(matches!(result, StopUiResult::Stopped));
    assert_eq!(controller.signals(), vec![Signal::Term, Signal::Kill]);
    assert!(
        !state_path.exists(),
        "state file should be removed when stop succeeds"
    );
}

#[test]
fn read_ws_state_running_removes_stale_identity_file() {
    let _env = env_lock();
    let temp = TempDir::new_in("/tmp").expect("tempdir");
    let state_path = temp.path().join("api.json");
    let data = serde_json::json!({
        "pid": std::process::id(),
        "ws_url": "ws://127.0.0.1:43210/ws",
        "ui_url": "http://127.0.0.1:43210/ui",
        "listen": "127.0.0.1:43210",
        "started_at": 1735689600u64,
        "process_started_at": 1u64,
    });
    fs::write(
        &state_path,
        serde_json::to_string_pretty(&data).expect("serialize ws state"),
    )
    .expect("write ws state");

    let state = read_ws_state_running(&state_path);
    assert!(state.is_none(), "stale ws state should be rejected");
    assert!(
        !state_path.exists(),
        "stale ws state file should be removed after rejection"
    );
}

#[test]
fn build_ui_url_allows_same_origin_absolute_override() {
    let state = WsState {
        pid: 1,
        ws_url: "ws://127.0.0.1:43210/ws?token=secret".to_string(),
        ui_url: Some("http://127.0.0.1:43210/ui".to_string()),
        listen: "127.0.0.1:43210".to_string(),
        started_at: None,
        process_started_at: None,
        http_url: None,
    };

    let url = build_ui_url("http://127.0.0.1:43210/custom?theme=warm", &state)
        .expect("same-origin override should be accepted");
    let parsed = url::Url::parse(&url).expect("url");

    assert_eq!(parsed.scheme(), "http");
    assert_eq!(parsed.host_str(), Some("127.0.0.1"));
    assert_eq!(parsed.port_or_known_default(), Some(43210));
    assert_eq!(parsed.path(), "/custom");
    assert_eq!(
        parsed
            .query_pairs()
            .find(|(key, _)| key == "theme")
            .map(|(_, value)| value.into_owned()),
        Some("warm".to_string())
    );
    assert_eq!(
        parsed
            .query_pairs()
            .find(|(key, _)| key == "ws")
            .map(|(_, value)| value.into_owned()),
        Some("ws://127.0.0.1:43210/ws?token=secret".to_string())
    );
}

#[test]
fn build_ui_url_rejects_cross_origin_override() {
    let state = WsState {
        pid: 1,
        ws_url: "ws://127.0.0.1:43210/ws?token=secret".to_string(),
        ui_url: Some("http://127.0.0.1:43210/ui".to_string()),
        listen: "127.0.0.1:43210".to_string(),
        started_at: None,
        process_started_at: None,
        http_url: None,
    };

    let err = build_ui_url("https://example.com/flightdeck", &state)
        .expect_err("cross-origin override should be rejected");
    assert!(
        err.to_string()
            .contains("AGENT_TUI_UI_URL must use the same origin"),
        "{err}"
    );
}

#[test]
fn build_ui_url_resolves_relative_override_against_daemon_origin() {
    let state = WsState {
        pid: 1,
        ws_url: "ws://127.0.0.1:43210/ws?token=secret".to_string(),
        ui_url: Some("http://127.0.0.1:43210/ui".to_string()),
        listen: "127.0.0.1:43210".to_string(),
        started_at: None,
        process_started_at: None,
        http_url: None,
    };

    let url = build_ui_url("/preview?theme=warm#panel", &state).expect("relative override");
    assert_eq!(
        url,
        "http://127.0.0.1:43210/preview?theme=warm&ws=ws%3A%2F%2F127.0.0.1%3A43210%2Fws%3Ftoken%3Dsecret&session=active&auto=1#panel"
    );
}

#[test]
fn stop_ui_server_rejects_legacy_pid_only_state() {
    let _env = env_lock();
    let temp = TempDir::new_in("/tmp").expect("tempdir");
    let state_path = temp.path().join("ui.json");
    fs::write(
        &state_path,
        format!(
            r#"{{"pid":{},"url":"http://127.0.0.1:7777/ui","port":7777}}"#,
            std::process::id()
        ),
    )
    .expect("write legacy ui state");
    let _state_guard = EnvVarGuard::set("AGENT_TUI_UI_STATE", state_path.display().to_string());
    let _external_guard = EnvVarGuard::set("AGENT_TUI_UI_URL", "");

    let controller = StopUiController::new(ProcessStatus::Running, false, true, Some(42));
    let result = stop_ui_server_with_controller_and_timeouts(
        &controller,
        Duration::from_millis(5),
        Duration::from_millis(5),
    )
    .expect("legacy state should be treated as already stopped");

    assert!(matches!(result, StopUiResult::AlreadyStopped));
    assert!(
        controller.signals().is_empty(),
        "legacy pid-only ui state must not trigger signals"
    );
    assert!(
        !state_path.exists(),
        "legacy ui state file should be removed"
    );
}
