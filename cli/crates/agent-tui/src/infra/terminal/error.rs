use crate::common::error_codes::{self, ErrorCategory};
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
mod tests {
    use super::*;

    #[test]
    fn test_pty_error_code() {
        let err = PtyError::Open {
            reason: "test".into(),
            source: None,
        };
        assert_eq!(err.code(), error_codes::PTY_ERROR);
    }

    #[test]
    fn test_pty_error_category() {
        let err = PtyError::Write {
            reason: "broken pipe".into(),
            source: None,
        };
        assert_eq!(err.category(), ErrorCategory::External);
    }

    #[test]
    fn test_pty_error_context() {
        let err = PtyError::Spawn {
            reason: "command not found".into(),
            kind: SpawnErrorKind::NotFound,
        };
        let ctx = err.context();
        assert_eq!(ctx.operation, "spawn");
        assert_eq!(ctx.reason, "command not found");
    }

    #[test]
    fn test_pty_error_suggestion_not_found() {
        let err = PtyError::Spawn {
            reason: "No such file or directory".into(),
            kind: SpawnErrorKind::NotFound,
        };
        assert!(err.suggestion().contains("not found"));
    }

    #[test]
    fn test_pty_error_suggestion_permission() {
        let err = PtyError::Spawn {
            reason: "Permission denied".into(),
            kind: SpawnErrorKind::PermissionDenied,
        };
        assert!(err.suggestion().contains("Permission"));
    }

    #[test]
    fn test_pty_error_is_retryable() {
        assert!(
            PtyError::Read {
                reason: "timeout".into(),
                source: None,
            }
            .is_retryable()
        );
        assert!(
            PtyError::Write {
                reason: "broken pipe".into(),
                source: None,
            }
            .is_retryable()
        );
        assert!(
            !PtyError::Open {
                reason: "failed".into(),
                source: None,
            }
            .is_retryable()
        );
        assert!(
            !PtyError::Spawn {
                reason: "not found".into(),
                kind: SpawnErrorKind::NotFound
            }
            .is_retryable()
        );
    }

    #[test]
    fn test_pty_error_operation() {
        assert_eq!(
            PtyError::Open {
                reason: "x".into(),
                source: None,
            }
            .operation(),
            "open"
        );
        assert_eq!(
            PtyError::Spawn {
                reason: "x".into(),
                kind: SpawnErrorKind::Other
            }
            .operation(),
            "spawn"
        );
        assert_eq!(
            PtyError::Write {
                reason: "x".into(),
                source: None,
            }
            .operation(),
            "write"
        );
        assert_eq!(
            PtyError::Read {
                reason: "x".into(),
                source: None,
            }
            .operation(),
            "read"
        );
        assert_eq!(
            PtyError::Resize {
                reason: "x".into(),
                source: None,
            }
            .operation(),
            "resize"
        );
    }

    #[test]
    fn test_pty_error_reason() {
        let err = PtyError::Open {
            reason: "allocation failed".into(),
            source: None,
        };
        assert_eq!(err.reason(), "allocation failed");
    }
}
