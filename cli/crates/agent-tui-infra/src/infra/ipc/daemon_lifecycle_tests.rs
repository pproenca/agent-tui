use super::*;
use crate::common::mutex_lock_or_recover;
use crate::test_support::MockProcessController;
use tempfile::tempdir;

fn identity(pid: u32) -> ProcessIdentity {
    ProcessIdentity {
        pid,
        started_at: None,
    }
}

#[test]
fn test_stop_daemon_not_running() {
    let mock = MockProcessController::new();
    let dir = tempdir().expect("temp dir should be created");
    let socket = dir.path().join("test.sock");

    let result = stop_daemon(&mock, identity(1234), &socket, false);
    assert!(matches!(result, Err(ClientError::DaemonNotRunning)));
}

#[test]
fn test_stop_daemon_success() {
    let mock = MockProcessController::new()
        .with_process(1234, ProcessStatus::Running)
        .with_signal_kills_process();
    let dir = tempdir().expect("temp dir should be created");
    let socket = dir.path().join("test.sock");

    let result = stop_daemon(&mock, identity(1234), &socket, false);
    assert!(result.is_ok());
    let stop_result = result.expect("stop_daemon should succeed");
    assert_eq!(stop_result.pid, 1234);
    assert!(stop_result.warnings.is_empty());
    assert_eq!(mock.signals_sent(), vec![(1234, Signal::Term)]);
}

#[test]
fn test_stop_daemon_force() {
    let mock = MockProcessController::new()
        .with_process(1234, ProcessStatus::Running)
        .with_signal_kills_process_on(Signal::Kill);
    let dir = tempdir().expect("temp dir should be created");
    let socket = dir.path().join("test.sock");

    let result = stop_daemon(&mock, identity(1234), &socket, true);
    assert!(result.is_ok());
    assert_eq!(mock.signals_sent(), vec![(1234, Signal::Kill)]);
}

#[test]
fn test_stop_daemon_returns_error_if_process_still_running() {
    let mock = MockProcessController::new().with_process(1234, ProcessStatus::Running);
    let dir = tempdir().expect("temp dir should be created");
    let socket = dir.path().join("missing.sock");

    let result = stop_daemon(&mock, identity(1234), &socket, false);

    assert!(matches!(
        result,
        Err(ClientError::SignalFailed {
            pid: 1234,
            message,
            ..
        }) if message.contains("did not shut down")
    ));
}

#[test]
fn test_stop_daemon_escalates_to_kill_after_graceful_timeout() {
    let mock = MockProcessController::new()
        .with_process(1234, ProcessStatus::Running)
        .with_signal_kills_process_on(Signal::Kill);
    let dir = tempdir().expect("temp dir should be created");
    let socket = dir.path().join("missing.sock");

    let result =
        stop_daemon(&mock, identity(1234), &socket, false).expect("stop_daemon should escalate");

    assert_eq!(result.pid, 1234);
    assert_eq!(
        result.warnings,
        vec!["Graceful shutdown timed out; forcing daemon shutdown with SIGKILL".to_string()]
    );
    assert_eq!(
        mock.signals_sent(),
        vec![(1234, Signal::Term), (1234, Signal::Kill)]
    );
}

#[test]
fn test_stop_daemon_no_permission() {
    let mock = MockProcessController::new().with_process(1234, ProcessStatus::NoPermission);
    let dir = tempdir().expect("temp dir should be created");
    let socket = dir.path().join("test.sock");

    let result = stop_daemon(&mock, identity(1234), &socket, false);
    assert!(matches!(
        result,
        Err(ClientError::SignalFailed {
            pid: 1234,
            message,
            ..
        }) if message.contains("Permission denied")
    ));
}

#[test]
fn test_stop_daemon_does_not_signal_reused_pid() {
    let mock =
        MockProcessController::new().with_process_identity(1234, ProcessStatus::Running, Some(99));
    let dir = tempdir().expect("temp dir should be created");
    let socket = dir.path().join("test.sock");

    let result = stop_daemon(
        &mock,
        ProcessIdentity {
            pid: 1234,
            started_at: Some(42),
        },
        &socket,
        false,
    );

    assert!(matches!(result, Err(ClientError::DaemonNotRunning)));
    assert!(
        mock.signals_sent().is_empty(),
        "stop_daemon must not signal a reused pid"
    );
}

#[test]
fn test_stop_daemon_cleans_stale_socket() {
    let mock = MockProcessController::new();
    let dir = tempdir().expect("temp dir should be created");
    let socket = dir.path().join("test.sock");
    let lock = socket.with_extension("lock");

    std::fs::write(&socket, "stale").expect("stale socket should be written");
    std::fs::write(&lock, "1234").expect("stale lock should be written");

    let result = stop_daemon(&mock, identity(1234), &socket, false);
    assert!(matches!(result, Err(ClientError::DaemonNotRunning)));

    assert!(!socket.exists());
    assert!(!lock.exists());
}

#[test]
fn test_restart_daemon_not_running() {
    let mock = MockProcessController::new();
    let dir = tempdir().expect("temp dir should be created");
    let socket = dir.path().join("test.sock");
    let started = std::sync::atomic::AtomicBool::new(false);

    let result = restart_daemon(
        &mock,
        || None,
        &socket,
        || {
            started.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert!(started.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn test_restart_daemon_running() {
    let mock = MockProcessController::new()
        .with_process(1234, ProcessStatus::Running)
        .with_signal_kills_process();
    let dir = tempdir().expect("temp dir should be created");
    let socket = dir.path().join("test.sock");
    let started = std::sync::atomic::AtomicBool::new(false);

    let result = restart_daemon(
        &mock,
        || Some(identity(1234)),
        &socket,
        || {
            started.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert!(started.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(mock.signals_sent(), vec![(1234, Signal::Term)]);
}

#[test]
fn test_restart_daemon_start_fails_after_stop() {
    let mock = MockProcessController::new()
        .with_process(1234, ProcessStatus::Running)
        .with_signal_kills_process();
    let dir = tempdir().expect("temp dir should be created");
    let socket = dir.path().join("test.sock");

    let result = restart_daemon(
        &mock,
        || Some(identity(1234)),
        &socket,
        || Err(ClientError::DaemonNotRunning),
    );

    assert!(matches!(result, Err(ClientError::DaemonNotRunning)));

    assert_eq!(mock.signals_sent(), vec![(1234, Signal::Term)]);
}

#[test]
fn test_restart_daemon_start_fails_when_not_running() {
    let mock = MockProcessController::new();
    let dir = tempdir().expect("temp dir should be created");
    let socket = dir.path().join("test.sock");

    let result = restart_daemon(
        &mock,
        || None,
        &socket,
        || {
            Err(ClientError::ConnectionFailed(std::io::Error::other(
                "Failed to start daemon",
            )))
        },
    );

    assert!(matches!(result, Err(ClientError::ConnectionFailed(_))));

    assert!(mock.signals_sent().is_empty());
}

use serde_json::Value;
use std::sync::Mutex;

struct MockDaemonClient {
    shutdown_response: Mutex<Option<Result<Value, ClientError>>>,
    calls: Mutex<Vec<String>>,
}

impl MockDaemonClient {
    fn new() -> Self {
        Self {
            shutdown_response: Mutex::new(Some(Ok(Value::Null))),
            calls: Mutex::new(Vec::new()),
        }
    }

    fn with_shutdown_response(self, response: Result<Value, ClientError>) -> Self {
        *mutex_lock_or_recover(&self.shutdown_response) = Some(response);
        self
    }

    fn calls(&self) -> Vec<String> {
        mutex_lock_or_recover(&self.calls).clone()
    }
}

impl DaemonClient for MockDaemonClient {
    fn call(&mut self, method: &str, _params: Option<Value>) -> Result<Value, ClientError> {
        mutex_lock_or_recover(&self.calls).push(method.to_string());
        if method == "shutdown" {
            mutex_lock_or_recover(&self.shutdown_response)
                .take()
                .unwrap_or(Err(ClientError::InvalidResponse))
        } else {
            Err(ClientError::InvalidResponse)
        }
    }

    fn call_with_config(
        &mut self,
        method: &str,
        params: Option<Value>,
        _config: &DaemonClientConfig,
    ) -> Result<Value, ClientError> {
        self.call(method, params)
    }
}

#[test]
fn test_stop_daemon_via_rpc_success() {
    let mut client = MockDaemonClient::new();
    let dir = tempdir().expect("temp dir should be created");
    let socket = dir.path().join("test.sock");

    let result = stop_daemon_via_rpc(&mut client, &socket);

    assert!(result.is_ok());
    let stop_result = result.expect("stop_daemon_via_rpc should succeed");
    assert!(stop_result.warnings.is_empty());
    assert_eq!(client.calls(), vec!["shutdown"]);
}

#[test]
fn test_stop_daemon_via_rpc_connection_failed() {
    let mut client = MockDaemonClient::new().with_shutdown_response(Err(
        ClientError::ConnectionFailed(std::io::Error::other("connection refused")),
    ));
    let dir = tempdir().expect("temp dir should be created");
    let socket = dir.path().join("test.sock");

    let result = stop_daemon_via_rpc(&mut client, &socket);

    assert!(matches!(result, Err(ClientError::ConnectionFailed(_))));
}

#[test]
fn test_stop_daemon_graceful_falls_back_to_signal_when_rpc_leaves_process_running() {
    let mock = MockProcessController::new()
        .with_process(1234, ProcessStatus::Running)
        .with_signal_kills_process();
    let dir = tempdir().expect("temp dir should be created");
    let socket = dir.path().join("test.sock");

    let result = stop_daemon_graceful(
        || Ok(MockDaemonClient::new()),
        &mock,
        identity(1234),
        &socket,
        false,
    )
    .expect("graceful stop should fall back to signal");

    assert_eq!(result.pid, 1234);
    assert_eq!(
        result.warnings,
        vec![
            "RPC shutdown was acknowledged but the daemon was still running; sent SIGTERM."
                .to_string()
        ]
    );
    assert_eq!(mock.signals_sent(), vec![(1234, Signal::Term)]);
}
