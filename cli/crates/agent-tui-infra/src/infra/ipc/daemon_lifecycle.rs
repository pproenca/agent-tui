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

    let config = DaemonClientConfig {
        read_timeout: SHUTDOWN_RPC_TIMEOUT,
        write_timeout: SHUTDOWN_RPC_TIMEOUT,
        max_retries: 0,
        ..DaemonClientConfig::default()
    };

    client.call_with_config("shutdown", None, &config)?;

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
#[path = "daemon_lifecycle_tests.rs"]
mod tests;
