use crate::common::error_codes::{self, ErrorCategory};
use crate::usecases::ports::PtyError as PortPtyError;
use crate::usecases::ports::SpawnErrorKind;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PtyError {
    #[error("Failed to open PTY: {0}")]
    Open(String),
    #[error("Failed to spawn process: {reason}")]
    Spawn {
        reason: String,
        kind: SpawnErrorKind,
    },
    #[error("Failed to write to PTY: {0}")]
    Write(String),
    #[error("Failed to read from PTY: {0}")]
    Read(String),
    #[error("Failed to resize PTY: {0}")]
    Resize(String),
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
            PtyError::Open(_) => {
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
            PtyError::Write(_) => {
                "Failed to send input to terminal. The session may have ended. Run 'sessions' to check status."
                    .to_string()
            }
            PtyError::Read(_) => {
                "Failed to read terminal output. The session may have ended. Run 'sessions' to check status."
                    .to_string()
            }
            PtyError::Resize(_) => {
                "Failed to resize terminal. Try again or restart the session.".to_string()
            }
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, PtyError::Read(_) | PtyError::Write(_))
    }

    pub fn operation(&self) -> &'static str {
        match self {
            PtyError::Open(_) => "open",
            PtyError::Spawn { .. } => "spawn",
            PtyError::Write(_) => "write",
            PtyError::Read(_) => "read",
            PtyError::Resize(_) => "resize",
        }
    }

    pub fn reason(&self) -> &str {
        match self {
            PtyError::Open(r) | PtyError::Write(r) | PtyError::Read(r) | PtyError::Resize(r) => r,
            PtyError::Spawn { reason, .. } => reason,
        }
    }
}

impl PtyError {
    /// Convert this infra error to the port error type.
    /// This keeps the dependency direction correct (infra -> usecases).
    pub fn to_port_error(self) -> PortPtyError {
        match self {
            PtyError::Open(reason) => PortPtyError::Open(reason),
            PtyError::Spawn { reason, kind } => PortPtyError::Spawn { reason, kind },
            PtyError::Write(reason) => PortPtyError::Write(reason),
            PtyError::Read(reason) => PortPtyError::Read(reason),
            PtyError::Resize(reason) => PortPtyError::Resize(reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pty_error_code() {
        let err = PtyError::Open("test".into());
        assert_eq!(err.code(), error_codes::PTY_ERROR);
    }

    #[test]
    fn test_pty_error_category() {
        let err = PtyError::Write("broken pipe".into());
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
        assert!(PtyError::Read("timeout".into()).is_retryable());
        assert!(PtyError::Write("broken pipe".into()).is_retryable());
        assert!(!PtyError::Open("failed".into()).is_retryable());
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
        assert_eq!(PtyError::Open("x".into()).operation(), "open");
        assert_eq!(
            PtyError::Spawn {
                reason: "x".into(),
                kind: SpawnErrorKind::Other
            }
            .operation(),
            "spawn"
        );
        assert_eq!(PtyError::Write("x".into()).operation(), "write");
        assert_eq!(PtyError::Read("x".into()).operation(), "read");
        assert_eq!(PtyError::Resize("x".into()).operation(), "resize");
    }

    #[test]
    fn test_pty_error_reason() {
        let err = PtyError::Open("allocation failed".into());
        assert_eq!(err.reason(), "allocation failed");
    }
}
