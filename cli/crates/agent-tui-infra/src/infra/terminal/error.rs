//! Terminal error types.

use crate::common::error_codes;
use crate::common::error_codes::ErrorCategory;
use crate::usecases::ports::SpawnErrorKind;
use crate::usecases::ports::TerminalError as PortTerminalError;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PtyError {
    #[error("Failed to open PTY: {reason}")]
    Open {
        reason: String,
        #[source]
        source: Option<io::Error>,
    },
    #[error("Failed to spawn process: {reason}")]
    Spawn {
        reason: String,
        kind: SpawnErrorKind,
    },
    #[error("Failed to write to PTY: {reason}")]
    Write {
        reason: String,
        #[source]
        source: Option<io::Error>,
    },
    #[error("Failed to read from PTY: {reason}")]
    Read {
        reason: String,
        #[source]
        source: Option<io::Error>,
    },
    #[error("Failed to resize PTY: {reason}")]
    Resize {
        reason: String,
        #[source]
        source: Option<io::Error>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyErrorContext {
    pub operation: &'static str,
    pub reason: String,
}

impl PtyError {
    pub fn code(&self) -> i32 {
        error_codes::PTY_ERROR
    }

    pub fn category(&self) -> ErrorCategory {
        ErrorCategory::External
    }

    pub fn context(&self) -> PtyErrorContext {
        PtyErrorContext {
            operation: self.operation(),
            reason: self.reason().to_string(),
        }
    }

    pub fn suggestion(&self) -> String {
        match self {
            PtyError::Open { .. } => {
                "PTY allocation failed. Check system resource limits (ulimit -n) or try restarting."
                    .to_string()
            }
            PtyError::Spawn { kind, .. } => match kind {
                SpawnErrorKind::NotFound => {
                    "Command not found. Check if the command exists and is in PATH.".to_string()
                }
                SpawnErrorKind::PermissionDenied => {
                    "Permission denied. Check file permissions.".to_string()
                }
                SpawnErrorKind::Other => {
                    "Process spawn failed. Check command syntax and permissions.".to_string()
                }
            },
            PtyError::Write { .. } => {
                "Failed to send input to terminal. The session may have ended. Run 'sessions' to check status."
                    .to_string()
            }
            PtyError::Read { .. } => {
                "Failed to read terminal output. The session may have ended. Run 'sessions' to check status."
                    .to_string()
            }
            PtyError::Resize { .. } => {
                "Failed to resize terminal. Try again or restart the session.".to_string()
            }
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, PtyError::Read { .. } | PtyError::Write { .. })
    }

    pub fn operation(&self) -> &'static str {
        match self {
            PtyError::Open { .. } => "open",
            PtyError::Spawn { .. } => "spawn",
            PtyError::Write { .. } => "write",
            PtyError::Read { .. } => "read",
            PtyError::Resize { .. } => "resize",
        }
    }

    pub fn reason(&self) -> &str {
        match self {
            PtyError::Open { reason, .. }
            | PtyError::Write { reason, .. }
            | PtyError::Read { reason, .. }
            | PtyError::Resize { reason, .. } => reason,
            PtyError::Spawn { reason, .. } => reason,
        }
    }
}

impl PtyError {
    /// Convert this infra error to the port error type.
    /// This keeps the dependency direction correct (infra -> usecases).
    pub fn into_port_error(self) -> PortTerminalError {
        match self {
            PtyError::Open { reason, source } => PortTerminalError::Open {
                reason,
                source: source.map(|err| Box::new(err) as _),
            },
            PtyError::Spawn { reason, kind } => PortTerminalError::Spawn { reason, kind },
            PtyError::Write { reason, source } => PortTerminalError::Write {
                reason,
                source: source.map(|err| Box::new(err) as _),
            },
            PtyError::Read { reason, source } => PortTerminalError::Read {
                reason,
                source: source.map(|err| Box::new(err) as _),
            },
            PtyError::Resize { reason, source } => PortTerminalError::Resize {
                reason,
                source: source.map(|err| Box::new(err) as _),
            },
        }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
