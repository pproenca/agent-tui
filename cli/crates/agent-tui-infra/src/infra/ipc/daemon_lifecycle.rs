//! Daemon lifecycle helpers.

use std::path::Path;
use std::time::Duration;
use std::time::Instant;

use crate::infra::ipc::DaemonClient;
use crate::infra::ipc::DaemonClientConfig;
use crate::infra::ipc::error::ClientError;
use crate::infra::ipc::polling;
use crate::infra::ipc::process::ProcessController;
use crate::infra::ipc::process::ProcessIdentity;
use crate::infra::ipc::process::ProcessStatus;
use crate::infra::ipc::process::Signal;
use crate::infra::ipc::process::check_expected_process;

const RPC_EXIT_GRACE: Duration = Duration::from_millis(250);
const FORCE_EXIT_TIMEOUT: Duration = Duration::from_secs(2);
const SHUTDOWN_RPC_TIMEOUT: Duration = Duration::from_secs(15);

pub struct StopResult {
    pub pid: u32,
    pub warnings: Vec<String>,
}

pub fn stop_daemon<P: ProcessController>(
    controller: &P,
    expected: ProcessIdentity,
    socket_path: &Path,
    force: bool,
) -> Result<StopResult, ClientError> {
    let mut warnings = Vec::new();
    let pid = expected.pid;

    match check_process_status(controller, expected)? {
        ProcessStatus::NotFound => {
            cleanup_daemon_files_with_warnings(socket_path, &mut warnings);
            return Err(ClientError::DaemonNotRunning);
        }
        ProcessStatus::NoPermission => {
            return Err(ClientError::SignalFailed {
                pid,
                message: "Permission denied".to_string(),
                source: None,
            });
        }
        ProcessStatus::Running => {}
    }

    let signal = if force { Signal::Kill } else { Signal::Term };
    controller.send_signal(pid, signal).map_err(|e| {
        let message = e.to_string();
        ClientError::SignalFailed {
            pid,
            message,
            source: Some(e),
        }
    })?;

    let mut status = wait_for_socket_removal_or_process_exit(controller, expected, socket_path)?;
    if matches!(status, ProcessStatus::Running) {
        let grace = if force {
            FORCE_EXIT_TIMEOUT
        } else {
            RPC_EXIT_GRACE
        };
        status = wait_for_process_exit(controller, expected, grace)?;
    }

    if matches!(status, ProcessStatus::Running) && !force {
        warnings
            .push("Graceful shutdown timed out; forcing daemon shutdown with SIGKILL".to_string());
        controller.send_signal(pid, Signal::Kill).map_err(|e| {
            let message = e.to_string();
            ClientError::SignalFailed {
                pid,
                message,
                source: Some(e),
            }
        })?;
        status = wait_for_process_exit(controller, expected, FORCE_EXIT_TIMEOUT)?;
    }

    match status {
        ProcessStatus::Running => {
            return Err(ClientError::SignalFailed {
                pid,
                message: "Daemon did not shut down".to_string(),
                source: None,
            });
        }
        ProcessStatus::NoPermission => {
            return Err(ClientError::SignalFailed {
                pid,
                message: "Permission denied".to_string(),
                source: None,
            });
        }
        ProcessStatus::NotFound => {}
    }

    if socket_path.exists() {
        cleanup_daemon_files_with_warnings(socket_path, &mut warnings);
    }

    Ok(StopResult { pid, warnings })
}

pub fn stop_daemon_via_rpc(
    client: &mut impl DaemonClient,
    socket_path: &Path,
) -> Result<StopResult, ClientError> {
    let mut warnings = Vec::new();

    let config = DaemonClientConfig::default()
        .with_read_timeout(SHUTDOWN_RPC_TIMEOUT)
        .with_write_timeout(SHUTDOWN_RPC_TIMEOUT)
        .with_max_retries(0);

    let result = client.call_with_config("shutdown", None, &config)?;

    let acknowledged = result
        .get("acknowledged")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    if !acknowledged {
        return Err(ClientError::UnexpectedResponse {
            message: "Shutdown was not acknowledged by daemon".to_string(),
        });
    }

    wait_for_socket_removal(socket_path);

    if socket_path.exists() {
        cleanup_daemon_files_with_warnings(socket_path, &mut warnings);
    }

    Ok(StopResult { pid: 0, warnings })
}

pub fn stop_daemon_graceful<F, P, C>(
    client_factory: F,
    controller: &P,
    expected: ProcessIdentity,
    socket_path: &Path,
    force: bool,
) -> Result<StopResult, ClientError>
where
    F: Fn() -> Result<C, ClientError>,
    P: ProcessController,
    C: DaemonClient,
{
    if force {
        return stop_daemon(controller, expected, socket_path, true);
    }
    let pid = expected.pid;

    let rpc_stop_result = match client_factory() {
        Ok(mut client) => stop_daemon_via_rpc(&mut client, socket_path).ok(),
        Err(_) => None,
    };

    if let Some(mut result) = rpc_stop_result {
        result.pid = pid;
        match wait_for_process_exit(controller, expected, RPC_EXIT_GRACE)? {
            ProcessStatus::NotFound => return Ok(result),
            ProcessStatus::NoPermission => {
                return Err(ClientError::SignalFailed {
                    pid,
                    message: "Permission denied".to_string(),
                    source: None,
                });
            }
            ProcessStatus::Running => {
                result.warnings.push(
                    "RPC shutdown was acknowledged but the daemon was still running; sent SIGTERM."
                        .to_string(),
                );
                let signal_result = stop_daemon(controller, expected, socket_path, false)?;
                result.warnings.extend(signal_result.warnings);
                return Ok(result);
            }
        }
    }

    stop_daemon(controller, expected, socket_path, false)
}

fn cleanup_daemon_files_with_warnings(socket: &Path, warnings: &mut Vec<String>) {
    match std::fs::remove_file(socket) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            warnings.push(format!("Failed to remove socket: {e}"));
        }
        _ => {}
    }
    let lock = socket.with_extension("lock");
    match std::fs::remove_file(&lock) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            warnings.push(format!("Failed to remove lock file: {e}"));
        }
        _ => {}
    }
}

fn wait_for_socket_removal(socket: &Path) {
    let start = Instant::now();
    let mut delay = polling::INITIAL_POLL_INTERVAL;

    while socket.exists() && start.elapsed() < polling::SHUTDOWN_TIMEOUT {
        std::thread::park_timeout(delay);
        delay = (delay * 2).min(polling::MAX_POLL_INTERVAL);
    }
}

fn check_process_status<P: ProcessController>(
    controller: &P,
    expected: ProcessIdentity,
) -> Result<ProcessStatus, ClientError> {
    check_expected_process(controller, expected).map_err(|e| {
        let message = e.to_string();
        ClientError::SignalFailed {
            pid: expected.pid,
            message,
            source: Some(e),
        }
    })
}

fn wait_for_process_exit<P: ProcessController>(
    controller: &P,
    expected: ProcessIdentity,
    timeout: Duration,
) -> Result<ProcessStatus, ClientError> {
    let start = Instant::now();
    let mut delay = polling::INITIAL_POLL_INTERVAL;

    loop {
        let status = check_process_status(controller, expected)?;
        if status != ProcessStatus::Running || start.elapsed() >= timeout {
            return Ok(status);
        }

        std::thread::park_timeout(delay);
        delay = (delay * 2).min(polling::MAX_POLL_INTERVAL);
    }
}

fn wait_for_socket_removal_or_process_exit<P: ProcessController>(
    controller: &P,
    expected: ProcessIdentity,
    socket: &Path,
) -> Result<ProcessStatus, ClientError> {
    let start = Instant::now();
    let mut delay = polling::INITIAL_POLL_INTERVAL;

    loop {
        let status = check_process_status(controller, expected)?;
        if !socket.exists()
            || status != ProcessStatus::Running
            || start.elapsed() >= polling::SHUTDOWN_TIMEOUT
        {
            return Ok(status);
        }

        std::thread::park_timeout(delay);
        delay = (delay * 2).min(polling::MAX_POLL_INTERVAL);
    }
}

pub fn restart_daemon<P, F, S>(
    controller: &P,
    get_pid: F,
    socket_path: &Path,
    start_fn: S,
) -> Result<Vec<String>, ClientError>
where
    P: ProcessController,
    F: Fn() -> Option<ProcessIdentity>,
    S: Fn() -> Result<(), ClientError>,
{
    let mut all_warnings = Vec::new();

    if let Some(expected) = get_pid() {
        match stop_daemon(controller, expected, socket_path, false) {
            Ok(result) => all_warnings.extend(result.warnings),
            Err(ClientError::DaemonNotRunning) => {}
            Err(e) => return Err(e),
        }
    }

    std::thread::park_timeout(Duration::from_millis(500));

    start_fn()?;

    Ok(all_warnings)
}

#[cfg(test)]
mod tests {
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

        let result = stop_daemon(&mock, identity(1234), &socket, false)
            .expect("stop_daemon should escalate");

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
        let mock = MockProcessController::new().with_process_identity(
            1234,
            ProcessStatus::Running,
            Some(99),
        );
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
    use serde_json::json;
    use std::sync::Mutex;

    struct MockDaemonClient {
        shutdown_response: Mutex<Option<Result<Value, ClientError>>>,
        calls: Mutex<Vec<String>>,
    }

    impl MockDaemonClient {
        fn new() -> Self {
            Self {
                shutdown_response: Mutex::new(Some(Ok(json!({ "acknowledged": true })))),
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
    fn test_stop_daemon_via_rpc_not_acknowledged() {
        let mut client =
            MockDaemonClient::new().with_shutdown_response(Ok(json!({ "acknowledged": false })));
        let dir = tempdir().expect("temp dir should be created");
        let socket = dir.path().join("test.sock");

        let result = stop_daemon_via_rpc(&mut client, &socket);

        assert!(matches!(
            result,
            Err(ClientError::UnexpectedResponse { message }) if message.contains("not acknowledged")
        ));
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
}
