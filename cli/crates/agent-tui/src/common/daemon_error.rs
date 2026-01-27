use crate::common::error_codes::{self, ErrorCategory};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("Failed to bind socket: {0}")]
    SocketBind(String),
    #[error("Another daemon instance is already running")]
    AlreadyRunning,
    #[error("Failed to acquire lock: {0}")]
    LockFailed(String),
    #[error("Failed to setup signal handler: {0}")]
    SignalSetup(String),
    #[error("Failed to create thread pool: {0}")]
    ThreadPool(String),
}

impl DaemonError {
    pub fn code(&self) -> i32 {
        error_codes::DAEMON_ERROR
    }

    pub fn category(&self) -> ErrorCategory {
        ErrorCategory::External
    }

    pub fn suggestion(&self) -> String {
        match self {
            DaemonError::SocketBind(_) => {
                "Check if the socket directory is writable. Try: rm /tmp/agent-tui.sock".to_string()
            }
            DaemonError::AlreadyRunning => {
                "Another daemon is running. Use 'agent-tui sessions' to connect or kill existing daemon."
                    .to_string()
            }
            DaemonError::LockFailed(_) => {
                "Lock file issue. Try removing the lock file: rm /tmp/agent-tui.sock.lock".to_string()
            }
            DaemonError::SignalSetup(_) => {
                "Signal handler setup failed. Check system signal configuration.".to_string()
            }
            DaemonError::ThreadPool(_) => {
                "Thread pool creation failed. Check system thread limits (ulimit -u).".to_string()
            }
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, DaemonError::LockFailed(_))
    }
}
