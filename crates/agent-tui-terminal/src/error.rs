//! PTY errors with structured context for AI agents.
//!
//! These errors provide semantic codes, categories, and actionable suggestions
//! to enable programmatic error handling.

use agent_tui_common::error_codes::{self, ErrorCategory};
use serde_json::{Value, json};
use thiserror::Error;

/// PTY operation errors with structured context.
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

impl PtyError {
    /// Returns the JSON-RPC error code for this error.
    ///
    /// All PTY errors map to PTY_ERROR (-32008) since they're all
    /// external/terminal communication failures. The specific operation
    /// is available via `context()`.
    pub fn code(&self) -> i32 {
        error_codes::PTY_ERROR
    }

    /// Returns the error category for programmatic handling.
    pub fn category(&self) -> ErrorCategory {
        ErrorCategory::External
    }

    /// Returns structured context about the error for debugging.
    pub fn context(&self) -> Value {
        match self {
            PtyError::Open(reason) => json!({
                "operation": "open",
                "reason": reason
            }),
            PtyError::Spawn(reason) => json!({
                "operation": "spawn",
                "reason": reason
            }),
            PtyError::Write(reason) => json!({
                "operation": "write",
                "reason": reason
            }),
            PtyError::Read(reason) => json!({
                "operation": "read",
                "reason": reason
            }),
            PtyError::Resize(reason) => json!({
                "operation": "resize",
                "reason": reason
            }),
        }
    }

    /// Returns a helpful suggestion for resolving the error.
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

    /// Returns whether this error is potentially transient and may succeed on retry.
    pub fn is_retryable(&self) -> bool {
        matches!(self, PtyError::Read(_) | PtyError::Write(_))
    }

    /// Returns the operation that failed.
    pub fn operation(&self) -> &'static str {
        match self {
            PtyError::Open(_) => "open",
            PtyError::Spawn(_) => "spawn",
            PtyError::Write(_) => "write",
            PtyError::Read(_) => "read",
            PtyError::Resize(_) => "resize",
        }
    }

    /// Returns the underlying reason/message for the error.
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
        assert_eq!(ctx["operation"], "spawn");
        assert_eq!(ctx["reason"], "command not found");
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
