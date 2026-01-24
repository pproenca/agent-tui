//! Daemon lifecycle management use cases.
//!
//! This module provides functions for managing daemon lifecycle:
//! - `stop_daemon`: Stop the running daemon
//! - `restart_daemon`: Stop (if running) and start the daemon

use std::path::Path;
use std::time::{Duration, Instant};

use crate::ipc::client::polling;
use crate::ipc::error::ClientError;
use crate::ipc::process::{ProcessController, ProcessStatus, Signal};

/// Result of stopping the daemon.
pub struct StopResult {
    /// PID of the stopped daemon.
    pub pid: u32,
    /// Warnings encountered during cleanup (non-fatal).
    pub warnings: Vec<String>,
}

/// Stop the daemon process.
///
/// # Arguments
/// * `controller` - Process controller for sending signals
/// * `pid` - PID of the daemon to stop
/// * `socket_path` - Path to the daemon socket file
/// * `force` - If true, send SIGKILL; otherwise send SIGTERM
pub fn stop_daemon<P: ProcessController>(
    controller: &P,
    pid: u32,
    socket_path: &Path,
    force: bool,
) -> Result<StopResult, ClientError> {
    let mut warnings = Vec::new();

    // 1. Verify process exists
    match controller
        .check_process(pid)
        .map_err(|e| ClientError::SignalFailed {
            pid,
            message: e.to_string(),
        })? {
        ProcessStatus::NotFound => {
            cleanup_daemon_files_with_warnings(socket_path, &mut warnings);
            return Err(ClientError::DaemonNotRunning);
        }
        ProcessStatus::NoPermission => {
            return Err(ClientError::SignalFailed {
                pid,
                message: "Permission denied".to_string(),
            });
        }
        ProcessStatus::Running => {}
    }

    // 2. Send signal
    let signal = if force { Signal::Kill } else { Signal::Term };
    controller
        .send_signal(pid, signal)
        .map_err(|e| ClientError::SignalFailed {
            pid,
            message: e.to_string(),
        })?;

    // 3. Wait for socket removal with exponential backoff
    wait_for_socket_removal(socket_path);

    // 4. Cleanup if socket still exists
    if socket_path.exists() {
        cleanup_daemon_files_with_warnings(socket_path, &mut warnings);
    }

    Ok(StopResult { pid, warnings })
}

/// Clean up daemon socket and lock files, collecting warnings.
fn cleanup_daemon_files_with_warnings(socket: &Path, warnings: &mut Vec<String>) {
    if let Err(e) = std::fs::remove_file(socket) {
        if e.kind() != std::io::ErrorKind::NotFound {
            warnings.push(format!("Failed to remove socket: {}", e));
        }
    }
    let lock = socket.with_extension("lock");
    if let Err(e) = std::fs::remove_file(&lock) {
        if e.kind() != std::io::ErrorKind::NotFound {
            warnings.push(format!("Failed to remove lock file: {}", e));
        }
    }
}

/// Wait for socket file to be removed with exponential backoff.
fn wait_for_socket_removal(socket: &Path) {
    let start = Instant::now();
    let mut delay = polling::INITIAL_POLL_INTERVAL;

    while socket.exists() && start.elapsed() < polling::SHUTDOWN_TIMEOUT {
        std::thread::sleep(delay);
        delay = (delay * 2).min(polling::MAX_POLL_INTERVAL);
    }
}

/// Restart daemon: stop (if running) + start.
///
/// # Arguments
/// * `controller` - Process controller for sending signals
/// * `get_pid` - Function to get the current daemon PID
/// * `socket_path` - Path to the daemon socket file
/// * `start_fn` - Function to start the daemon
pub fn restart_daemon<P, F, S>(
    controller: &P,
    get_pid: F,
    socket_path: &Path,
    start_fn: S,
) -> Result<Vec<String>, ClientError>
where
    P: ProcessController,
    F: Fn() -> Option<u32>,
    S: Fn() -> Result<(), ClientError>,
{
    let mut all_warnings = Vec::new();

    // Stop if running
    if let Some(pid) = get_pid() {
        match stop_daemon(controller, pid, socket_path, false) {
            Ok(result) => all_warnings.extend(result.warnings),
            Err(ClientError::DaemonNotRunning) => {} // OK, continue
            Err(e) => return Err(e),
        }
    }

    // Brief delay for cleanup
    std::thread::sleep(Duration::from_millis(500));

    // Start
    start_fn()?;

    Ok(all_warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::process::mock::MockProcessController;
    use tempfile::tempdir;

    #[test]
    fn test_stop_daemon_not_running() {
        let mock = MockProcessController::new();
        let dir = tempdir().unwrap();
        let socket = dir.path().join("test.sock");

        let result = stop_daemon(&mock, 1234, &socket, false);
        assert!(matches!(result, Err(ClientError::DaemonNotRunning)));
    }

    #[test]
    fn test_stop_daemon_success() {
        let mock = MockProcessController::new().with_process(1234, ProcessStatus::Running);
        let dir = tempdir().unwrap();
        let socket = dir.path().join("test.sock");

        let result = stop_daemon(&mock, 1234, &socket, false);
        assert!(result.is_ok());
        let stop_result = result.unwrap();
        assert_eq!(stop_result.pid, 1234);
        assert!(stop_result.warnings.is_empty());
        assert_eq!(mock.signals_sent(), vec![(1234, Signal::Term)]);
    }

    #[test]
    fn test_stop_daemon_force() {
        let mock = MockProcessController::new().with_process(1234, ProcessStatus::Running);
        let dir = tempdir().unwrap();
        let socket = dir.path().join("test.sock");

        let result = stop_daemon(&mock, 1234, &socket, true);
        assert!(result.is_ok());
        assert_eq!(mock.signals_sent(), vec![(1234, Signal::Kill)]);
    }

    #[test]
    fn test_stop_daemon_no_permission() {
        let mock = MockProcessController::new().with_process(1234, ProcessStatus::NoPermission);
        let dir = tempdir().unwrap();
        let socket = dir.path().join("test.sock");

        let result = stop_daemon(&mock, 1234, &socket, false);
        assert!(matches!(
            result,
            Err(ClientError::SignalFailed {
                pid: 1234,
                message
            }) if message.contains("Permission denied")
        ));
    }

    #[test]
    fn test_stop_daemon_cleans_stale_socket() {
        let mock = MockProcessController::new();
        let dir = tempdir().unwrap();
        let socket = dir.path().join("test.sock");
        let lock = socket.with_extension("lock");

        // Create stale files
        std::fs::write(&socket, "stale").unwrap();
        std::fs::write(&lock, "1234").unwrap();

        let result = stop_daemon(&mock, 1234, &socket, false);
        assert!(matches!(result, Err(ClientError::DaemonNotRunning)));
        // Stale files should be cleaned up
        assert!(!socket.exists());
        assert!(!lock.exists());
    }

    #[test]
    fn test_restart_daemon_not_running() {
        let mock = MockProcessController::new();
        let dir = tempdir().unwrap();
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
        let mock = MockProcessController::new().with_process(1234, ProcessStatus::Running);
        let dir = tempdir().unwrap();
        let socket = dir.path().join("test.sock");
        let started = std::sync::atomic::AtomicBool::new(false);

        let result = restart_daemon(
            &mock,
            || Some(1234),
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
        let mock = MockProcessController::new().with_process(1234, ProcessStatus::Running);
        let dir = tempdir().unwrap();
        let socket = dir.path().join("test.sock");

        let result = restart_daemon(
            &mock,
            || Some(1234),
            &socket,
            || Err(ClientError::DaemonNotRunning), // Simulate start failure
        );

        // Error should propagate from start_fn
        assert!(matches!(result, Err(ClientError::DaemonNotRunning)));
        // Stop should have been called
        assert_eq!(mock.signals_sent(), vec![(1234, Signal::Term)]);
    }

    #[test]
    fn test_restart_daemon_start_fails_when_not_running() {
        let mock = MockProcessController::new();
        let dir = tempdir().unwrap();
        let socket = dir.path().join("test.sock");

        let result = restart_daemon(
            &mock,
            || None, // Daemon not running
            &socket,
            || {
                Err(ClientError::ConnectionFailed(std::io::Error::other(
                    "Failed to start daemon",
                )))
            },
        );

        // Error should propagate from start_fn
        assert!(matches!(result, Err(ClientError::ConnectionFailed(_))));
        // No stop signal should have been sent (daemon wasn't running)
        assert!(mock.signals_sent().is_empty());
    }
}
