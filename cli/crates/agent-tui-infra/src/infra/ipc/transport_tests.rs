#![allow(
    clippy::expect_used,
    reason = "Test-only assertions use expect for clarity."
)]

use super::*;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;
use std::sync::atomic::Ordering;
use tempfile::TempDir;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct SharedLogBuffer {
    buffer: std::sync::Arc<Mutex<Vec<u8>>>,
}

struct SharedLogWriter {
    buffer: std::sync::Arc<Mutex<Vec<u8>>>,
}

impl SharedLogBuffer {
    fn contents(&self) -> String {
        let bytes = self
            .buffer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        String::from_utf8(bytes).expect("logs should be valid utf-8")
    }
}

impl<'a> MakeWriter<'a> for SharedLogBuffer {
    type Writer = SharedLogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        SharedLogWriter {
            buffer: std::sync::Arc::clone(&self.buffer),
        }
    }
}

impl std::io::Write for SharedLogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self
            .buffer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::io::Write::write_all(&mut *guard, buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct EnvGuard {
    key: &'static str,
    value: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: impl Into<String>) -> Self {
        let value = value.into();
        let prev = std::env::var(key).ok();
        // SAFETY: test-only environment mutation for isolated test setup.
        unsafe {
            std::env::set_var(key, &value);
        }
        Self { key, value: prev }
    }

    fn remove(key: &'static str) -> Self {
        let prev = std::env::var(key).ok();
        // SAFETY: test-only environment mutation for isolated test setup.
        unsafe {
            std::env::remove_var(key);
        }
        Self { key, value: prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // SAFETY: test-only environment restoration after mutation.
        unsafe {
            match self.value.take() {
                Some(prev) => std::env::set_var(self.key, prev),
                None => std::env::remove_var(self.key),
            }
        }
    }
}

fn env_lock() -> &'static Mutex<()> {
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_LOCK.get_or_init(|| Mutex::new(()))
}

fn acquire_env_lock() -> MutexGuard<'static, ()> {
    env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[test]
fn ws_state_path_ignores_deprecated_api_state_alias() {
    let _env_lock = acquire_env_lock();
    let temp_dir = TempDir::new_in("/tmp").expect("Failed to create temp dir");
    let home = temp_dir.path().join("home");
    std::fs::create_dir_all(&home).expect("create temp home");
    let _home_guard = EnvGuard::set("HOME", home.display().to_string());
    let _ws_state_guard = EnvGuard::remove("AGENT_TUI_WS_STATE");
    let _api_state_guard = EnvGuard::set("AGENT_TUI_API_STATE", "/tmp/deprecated-state.json");

    let expected = home.join(".agent-tui").join("api.json");
    assert_eq!(ws_state_path_from_env(), expected);
}

#[test]
fn redact_ws_url_for_log_strips_query_secrets() {
    let url = Url::parse("ws://127.0.0.1:7777/ws?token=super-secret&other=value")
        .expect("url should parse");

    let redacted = super::redact_ws_url_for_log(&url);

    assert_eq!(redacted, "ws://127.0.0.1:7777/ws?redacted");
    assert!(!redacted.contains("super-secret"));
}

#[test]
fn ws_transport_logs_redacted_websocket_address() {
    let logs = SharedLogBuffer::default();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(logs.clone())
        .with_ansi(false)
        .without_time()
        .finish();
    let transport = IpcTransport::WebSocket {
        address: Some(
            Url::parse("ws://127.0.0.1:1/ws?token=super-secret").expect("url should parse"),
        ),
    };

    tracing::subscriber::with_default(subscriber, || {
        let _ = transport.connect_connection();
    });

    let output = logs.contents();
    assert!(output.contains("Connecting to daemon websocket"));
    assert!(output.contains("ws://127.0.0.1:1/ws?redacted"));
    assert!(!output.contains("super-secret"));
}

#[test]
fn start_daemon_background_reaps_early_exit() {
    let _env_lock = acquire_env_lock();
    let temp_dir = TempDir::new_in("/tmp").expect("Failed to create temp dir");
    let socket_path = temp_dir.path().join("daemon.sock");
    let _socket_guard = EnvGuard::set("AGENT_TUI_SOCKET", socket_path.display().to_string());
    let _cmd_guard = EnvGuard::set("AGENT_TUI_DAEMON_START_TEST_CMD", "true");

    DAEMON_START_TEST_REAPED.store(false, Ordering::SeqCst);

    let result = start_daemon_background_impl();
    assert!(matches!(result, Err(ClientError::DaemonNotRunning)));
    assert!(DAEMON_START_TEST_REAPED.load(Ordering::SeqCst));
}

#[test]
fn start_daemon_background_impl_guards_against_recursive_spawn() {
    let _env_lock = acquire_env_lock();
    let temp_dir = TempDir::new_in("/tmp").expect("Failed to create temp dir");
    let socket_path = temp_dir.path().join("daemon.sock");
    let _socket_guard = EnvGuard::set("AGENT_TUI_SOCKET", socket_path.display().to_string());
    let _fg_guard = EnvGuard::set("AGENT_TUI_DAEMON_FOREGROUND", "1");

    let result = start_daemon_background_impl();
    assert!(
        matches!(result, Err(ClientError::DaemonNotRunning)),
        "should refuse to spawn when AGENT_TUI_DAEMON_FOREGROUND is set"
    );
}

#[cfg(unix)]
#[test]
fn reaper_failure_fallback_terminates_and_reaps_child() {
    use crate::infra::ipc::ProcessController;
    use crate::infra::ipc::ProcessStatus;
    use crate::infra::ipc::UnixProcessController;
    use std::process::Command;
    use std::time::Duration;

    let child = Command::new("sh")
        .arg("-c")
        .arg("sleep 30")
        .spawn()
        .expect("failed to spawn child");
    let pid = child.id();

    let (result_tx, result_rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let _ = result_tx.send(handle_reaper_spawn_failure(child));
    });
    let result = result_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("reaper fallback should return in bounded time");
    assert!(matches!(
        result,
        Err(ClientError::UnexpectedResponse { .. })
    ));

    let controller = UnixProcessController;
    let status = controller
        .check_process(pid)
        .expect("process check should succeed");
    assert!(
        matches!(status, ProcessStatus::NotFound),
        "child must be reaped after fallback, got {status:?}"
    );
}
