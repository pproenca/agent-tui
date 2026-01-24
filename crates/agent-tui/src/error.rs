//! CLI errors with structured context for AI agents.
//!
//! These errors provide semantic codes, categories, and actionable suggestions
//! to enable programmatic error handling and UNIX sysexits.h-compliant exit codes.

use std::io;

use crate::ipc::error_codes::{self, ErrorCategory};
use serde_json::{Value, json};
use thiserror::Error;

/// Attach mode errors with structured context.
#[derive(Error, Debug)]
pub enum AttachError {
    #[error("Terminal error: {0}")]
    Terminal(#[from] io::Error),

    #[error("PTY write failed: {0}")]
    PtyWrite(String),

    #[error("PTY read failed: {0}")]
    PtyRead(String),

    #[error("Event read failed")]
    EventRead,
}

impl AttachError {
    /// Returns the JSON-RPC error code for this error.
    pub fn code(&self) -> i32 {
        match self {
            AttachError::Terminal(_) => error_codes::PTY_ERROR,
            AttachError::PtyWrite(_) => error_codes::PTY_ERROR,
            AttachError::PtyRead(_) => error_codes::PTY_ERROR,
            AttachError::EventRead => error_codes::PTY_ERROR,
        }
    }

    /// Returns the error category for programmatic handling.
    pub fn category(&self) -> ErrorCategory {
        ErrorCategory::External
    }

    /// Returns structured context about the error for debugging.
    pub fn context(&self) -> Value {
        match self {
            AttachError::Terminal(e) => json!({
                "operation": "terminal",
                "reason": e.to_string()
            }),
            AttachError::PtyWrite(reason) => json!({
                "operation": "pty_write",
                "reason": reason
            }),
            AttachError::PtyRead(reason) => json!({
                "operation": "pty_read",
                "reason": reason
            }),
            AttachError::EventRead => json!({
                "operation": "event_read",
                "reason": "Failed to read terminal events"
            }),
        }
    }

    /// Returns a helpful suggestion for resolving the error.
    pub fn suggestion(&self) -> String {
        match self {
            AttachError::Terminal(_) => {
                "Terminal mode error. Try restarting your terminal.".to_string()
            }
            AttachError::PtyWrite(_) => {
                "Failed to send input to session. The session may have ended. Run 'sessions' to check status."
                    .to_string()
            }
            AttachError::PtyRead(_) => {
                "Failed to read from session. The session may have ended. Run 'sessions' to check status."
                    .to_string()
            }
            AttachError::EventRead => {
                "Failed to read terminal events. Try restarting your terminal.".to_string()
            }
        }
    }

    /// Returns whether this error is potentially transient and may succeed on retry.
    pub fn is_retryable(&self) -> bool {
        matches!(self, AttachError::PtyWrite(_) | AttachError::PtyRead(_))
    }

    /// Converts to UNIX sysexits.h-compliant exit code.
    pub fn exit_code(&self) -> i32 {
        match self.category() {
            ErrorCategory::InvalidInput => 64, // EX_USAGE
            ErrorCategory::NotFound => 69,     // EX_UNAVAILABLE
            ErrorCategory::Busy => 73,         // EX_CANTCREAT
            ErrorCategory::External => 74,     // EX_IOERR
            ErrorCategory::Internal => 74,     // EX_IOERR
            ErrorCategory::Timeout => 75,      // EX_TEMPFAIL
        }
    }

    /// Returns structured JSON representation of this error.
    pub fn to_json(&self) -> Value {
        json!({
            "code": self.code(),
            "message": self.to_string(),
            "category": self.category().as_str(),
            "retryable": self.is_retryable(),
            "context": self.context(),
            "suggestion": self.suggestion()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attach_error_code() {
        let err = AttachError::PtyWrite("broken pipe".into());
        assert_eq!(err.code(), error_codes::PTY_ERROR);

        let err = AttachError::PtyRead("timeout".into());
        assert_eq!(err.code(), error_codes::PTY_ERROR);

        let err = AttachError::EventRead;
        assert_eq!(err.code(), error_codes::PTY_ERROR);
    }

    #[test]
    fn test_attach_error_category() {
        let err = AttachError::PtyWrite("x".into());
        assert_eq!(err.category(), ErrorCategory::External);

        let err = AttachError::EventRead;
        assert_eq!(err.category(), ErrorCategory::External);
    }

    #[test]
    fn test_attach_error_context() {
        let err = AttachError::PtyWrite("broken pipe".into());
        let ctx = err.context();
        assert_eq!(ctx["operation"], "pty_write");
        assert_eq!(ctx["reason"], "broken pipe");

        let err = AttachError::PtyRead("timeout".into());
        let ctx = err.context();
        assert_eq!(ctx["operation"], "pty_read");
        assert_eq!(ctx["reason"], "timeout");
    }

    #[test]
    fn test_attach_error_suggestion() {
        let err = AttachError::PtyWrite("x".into());
        assert!(err.suggestion().contains("session"));

        let err = AttachError::EventRead;
        assert!(err.suggestion().contains("terminal"));
    }

    #[test]
    fn test_attach_error_is_retryable() {
        assert!(AttachError::PtyWrite("x".into()).is_retryable());
        assert!(AttachError::PtyRead("x".into()).is_retryable());
        assert!(!AttachError::EventRead.is_retryable());
    }

    #[test]
    fn test_attach_error_exit_code() {
        let err = AttachError::PtyWrite("x".into());
        assert_eq!(err.exit_code(), 74); // EX_IOERR
    }

    #[test]
    fn test_attach_error_to_json() {
        let err = AttachError::PtyRead("connection reset".into());
        let json = err.to_json();
        assert_eq!(json["code"], error_codes::PTY_ERROR);
        assert_eq!(json["category"], "external");
        assert_eq!(json["retryable"], true);
        assert!(
            json["context"]["operation"]
                .as_str()
                .unwrap()
                .contains("pty_read")
        );
    }
}
