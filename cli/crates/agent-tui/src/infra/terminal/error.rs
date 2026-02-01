use crate::common::error_codes::{self, ErrorCategory};
use crate::usecases::ports::PtyError as PortPtyError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PtyError {
    #[error("Failed to open PTY: {0}")]
    Open(String),
    #[error("Failed to spawn process: {0}")]
    Spawn(String),
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
            PtyError::Spawn(reason) => {
                if reason.contains("not found") || reason.contains("No such file") {
                    "Command not found. Check if the command exists and is in PATH.".to_string()
                } else if reason.contains("Permission denied") {
                    "Permission denied. Check file permissions.".to_string()
                } else {
                    "Process spawn failed. Check command syntax and permissions.".to_string()
                }
            }
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
            PtyError::Spawn(_) => "spawn",
            PtyError::Write(_) => "write",
            PtyError::Read(_) => "read",
            PtyError::Resize(_) => "resize",
        }
    }

    pub fn reason(&self) -> &str {
        match self {
            PtyError::Open(r)
            | PtyError::Spawn(r)
            | PtyError::Write(r)
            | PtyError::Read(r)
            | PtyError::Resize(r) => r,
        }
    }
}

impl PtyError {
    /// Convert this infra error to the port error type.
    /// This keeps the dependency direction correct (infra -> usecases).
    pub fn to_port_error(self) -> PortPtyError {
        match self {
            PtyError::Open(reason) => PortPtyError::Open(reason),
            PtyError::Spawn(reason) => PortPtyError::Spawn(reason),
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
        let err = PtyError::Spawn("command not found".into());
        let ctx = err.context();
        assert_eq!(ctx.operation, "spawn");
        assert_eq!(ctx.reason, "command not found");
    }

    #[test]
    fn test_pty_error_suggestion_not_found() {
        let err = PtyError::Spawn("No such file or directory".into());
        assert!(err.suggestion().contains("not found"));
    }

    #[test]
    fn test_pty_error_suggestion_permission() {
        let err = PtyError::Spawn("Permission denied".into());
        assert!(err.suggestion().contains("Permission"));
    }

    #[test]
    fn test_pty_error_is_retryable() {
        assert!(PtyError::Read("timeout".into()).is_retryable());
        assert!(PtyError::Write("broken pipe".into()).is_retryable());
        assert!(!PtyError::Open("failed".into()).is_retryable());
        assert!(!PtyError::Spawn("not found".into()).is_retryable());
    }

    #[test]
    fn test_pty_error_operation() {
        assert_eq!(PtyError::Open("x".into()).operation(), "open");
        assert_eq!(PtyError::Spawn("x".into()).operation(), "spawn");
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
