use std::io;

use crate::app::commands::OutputFormat;
use crate::common::error_codes::{self, ErrorCategory};
use serde::Serialize;
use thiserror::Error;

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

#[derive(Debug)]
pub struct CliError {
    pub exit_code: i32,
    pub format: OutputFormat,
    pub message: String,
    pub json: Option<String>,
}

impl CliError {
    pub fn new(
        format: OutputFormat,
        message: impl Into<String>,
        json: Option<String>,
        exit_code: i32,
    ) -> Self {
        Self {
            exit_code,
            format,
            message: message.into(),
            json,
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CliError {}

impl AttachError {
    pub fn code(&self) -> i32 {
        match self {
            AttachError::Terminal(_) => error_codes::PTY_ERROR,
            AttachError::PtyWrite(_) => error_codes::PTY_ERROR,
            AttachError::PtyRead(_) => error_codes::PTY_ERROR,
            AttachError::EventRead => error_codes::PTY_ERROR,
        }
    }

    pub fn category(&self) -> ErrorCategory {
        ErrorCategory::External
    }

    pub fn context(&self) -> AttachErrorContext {
        match self {
            AttachError::Terminal(e) => AttachErrorContext {
                operation: "terminal",
                reason: e.to_string(),
            },
            AttachError::PtyWrite(reason) => AttachErrorContext {
                operation: "pty_write",
                reason: reason.clone(),
            },
            AttachError::PtyRead(reason) => AttachErrorContext {
                operation: "pty_read",
                reason: reason.clone(),
            },
            AttachError::EventRead => AttachErrorContext {
                operation: "event_read",
                reason: "Failed to read terminal events".to_string(),
            },
        }
    }

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

    pub fn is_retryable(&self) -> bool {
        matches!(self, AttachError::PtyWrite(_) | AttachError::PtyRead(_))
    }

    pub fn exit_code(&self) -> i32 {
        match self.category() {
            ErrorCategory::InvalidInput => 64,
            ErrorCategory::NotFound => 69,
            ErrorCategory::Busy => 73,
            ErrorCategory::External => 74,
            ErrorCategory::Internal => 74,
            ErrorCategory::Timeout => 75,
        }
    }

    pub fn to_payload(&self) -> AttachErrorPayload {
        AttachErrorPayload {
            code: self.code(),
            message: self.to_string(),
            category: self.category().as_str().to_string(),
            retryable: self.is_retryable(),
            context: self.context(),
            suggestion: self.suggestion(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AttachErrorContext {
    pub operation: &'static str,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttachErrorPayload {
    pub code: i32,
    pub message: String,
    pub category: String,
    pub retryable: bool,
    pub context: AttachErrorContext,
    pub suggestion: String,
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
        assert_eq!(ctx.operation, "pty_write");
        assert_eq!(ctx.reason, "broken pipe");

        let err = AttachError::PtyRead("timeout".into());
        let ctx = err.context();
        assert_eq!(ctx.operation, "pty_read");
        assert_eq!(ctx.reason, "timeout");
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
        assert_eq!(err.exit_code(), 74);
    }

    #[test]
    fn test_attach_error_to_payload() {
        let err = AttachError::PtyRead("connection reset".into());
        let payload = err.to_payload();
        assert_eq!(payload.code, error_codes::PTY_ERROR);
        assert_eq!(payload.category, "external");
        assert!(payload.retryable);
        assert_eq!(payload.context.operation, "pty_read");
    }
}
