//! Domain errors for daemon operations.
//!
//! These errors are mapped to specific JSON-RPC error codes and include
//! structured context for AI agents to handle programmatically.

use agent_tui_ipc::error_codes::{self, ErrorCategory};
use serde_json::{Value, json};

use crate::session::SessionError;

/// Domain-specific errors with semantic codes and structured context.
#[derive(Debug)]
pub enum DomainError {
    /// Session with given ID does not exist
    SessionNotFound { session_id: String },
    /// No session is currently active (none specified and no default)
    NoActiveSession,
    /// Element with given ref not found in current screen
    ElementNotFound {
        element_ref: String,
        session_id: Option<String>,
    },
    /// Element exists but is wrong type for operation
    WrongElementType {
        element_ref: String,
        actual: String,
        expected: String,
    },
    /// Invalid key name provided for keystroke
    InvalidKey { key: String },
    /// Maximum session limit reached
    SessionLimitReached { max: usize },
    /// Failed to acquire session lock within timeout
    LockTimeout { session_id: Option<String> },
    /// PTY communication error
    PtyError { operation: String, reason: String },
    /// Wait condition not met within timeout
    WaitTimeout {
        condition: String,
        elapsed_ms: u64,
        timeout_ms: u64,
    },
    /// Command to spawn not found
    CommandNotFound { command: String },
    /// Permission denied when spawning command
    PermissionDenied { command: String },
    /// Generic error (for backwards compatibility)
    Generic { message: String },
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainError::SessionNotFound { session_id } => {
                write!(f, "Session not found: {}", session_id)
            }
            DomainError::NoActiveSession => {
                write!(f, "No active session")
            }
            DomainError::ElementNotFound { element_ref, .. } => {
                write!(f, "Element not found: {}", element_ref)
            }
            DomainError::WrongElementType {
                element_ref,
                actual,
                expected,
            } => {
                write!(
                    f,
                    "Element {} is a {} not a {}",
                    element_ref, actual, expected
                )
            }
            DomainError::InvalidKey { key } => {
                write!(f, "Invalid key: {}", key)
            }
            DomainError::SessionLimitReached { max } => {
                write!(f, "Session limit reached: maximum {} sessions allowed", max)
            }
            DomainError::LockTimeout { session_id } => match session_id {
                Some(id) => write!(f, "Lock timeout for session: {}", id),
                None => write!(f, "Lock timeout"),
            },
            DomainError::PtyError { operation, reason } => {
                write!(f, "PTY error during {}: {}", operation, reason)
            }
            DomainError::WaitTimeout { condition, .. } => {
                write!(f, "Timeout waiting for: {}", condition)
            }
            DomainError::CommandNotFound { command } => {
                write!(f, "Command not found: {}", command)
            }
            DomainError::PermissionDenied { command } => {
                write!(f, "Permission denied: {}", command)
            }
            DomainError::Generic { message } => {
                write!(f, "{}", message)
            }
        }
    }
}

impl std::error::Error for DomainError {}

impl DomainError {
    /// Returns the JSON-RPC error code for this error.
    pub fn code(&self) -> i32 {
        match self {
            DomainError::SessionNotFound { .. } => error_codes::SESSION_NOT_FOUND,
            DomainError::NoActiveSession => error_codes::NO_ACTIVE_SESSION,
            DomainError::ElementNotFound { .. } => error_codes::ELEMENT_NOT_FOUND,
            DomainError::WrongElementType { .. } => error_codes::WRONG_ELEMENT_TYPE,
            DomainError::InvalidKey { .. } => error_codes::INVALID_KEY,
            DomainError::SessionLimitReached { .. } => error_codes::SESSION_LIMIT,
            DomainError::LockTimeout { .. } => error_codes::LOCK_TIMEOUT,
            DomainError::PtyError { .. } => error_codes::PTY_ERROR,
            DomainError::WaitTimeout { .. } => error_codes::WAIT_TIMEOUT,
            DomainError::CommandNotFound { .. } => error_codes::COMMAND_NOT_FOUND,
            DomainError::PermissionDenied { .. } => error_codes::PERMISSION_DENIED,
            DomainError::Generic { .. } => error_codes::GENERIC_ERROR,
        }
    }

    /// Returns the error category for programmatic handling.
    pub fn category(&self) -> ErrorCategory {
        error_codes::category_for_code(self.code())
    }

    /// Returns structured context about the error for debugging.
    pub fn context(&self) -> Value {
        match self {
            DomainError::SessionNotFound { session_id } => {
                json!({ "session_id": session_id })
            }
            DomainError::NoActiveSession => json!({}),
            DomainError::ElementNotFound {
                element_ref,
                session_id,
            } => {
                let mut ctx = json!({ "element_ref": element_ref });
                if let Some(sid) = session_id {
                    ctx["session_id"] = json!(sid);
                }
                ctx
            }
            DomainError::WrongElementType {
                element_ref,
                actual,
                expected,
            } => {
                json!({
                    "element_ref": element_ref,
                    "actual_type": actual,
                    "expected_type": expected
                })
            }
            DomainError::InvalidKey { key } => {
                json!({ "key": key })
            }
            DomainError::SessionLimitReached { max } => {
                json!({ "max_sessions": max })
            }
            DomainError::LockTimeout { session_id } => match session_id {
                Some(id) => json!({ "session_id": id }),
                None => json!({}),
            },
            DomainError::PtyError { operation, reason } => {
                json!({
                    "operation": operation,
                    "reason": reason
                })
            }
            DomainError::WaitTimeout {
                condition,
                elapsed_ms,
                timeout_ms,
            } => {
                json!({
                    "condition": condition,
                    "elapsed_ms": elapsed_ms,
                    "timeout_ms": timeout_ms
                })
            }
            DomainError::CommandNotFound { command } => {
                json!({ "command": command })
            }
            DomainError::PermissionDenied { command } => {
                json!({ "command": command })
            }
            DomainError::Generic { message } => {
                json!({ "message": message })
            }
        }
    }

    /// Returns a helpful suggestion for resolving the error.
    pub fn suggestion(&self) -> String {
        match self {
            DomainError::SessionNotFound { .. } | DomainError::NoActiveSession => {
                "Run 'sessions' to list active sessions or 'spawn <cmd>' to start a new one."
                    .to_string()
            }
            DomainError::ElementNotFound { element_ref, .. } => {
                format!(
                    "Element '{}' not found. Run 'snapshot -i' to see current elements and their refs.",
                    element_ref
                )
            }
            DomainError::WrongElementType {
                element_ref,
                actual,
                ..
            } => suggest_command_for_type(actual, element_ref),
            DomainError::InvalidKey { .. } => {
                "Supported keys: Enter, Tab, Escape, Backspace, Delete, ArrowUp/Down/Left/Right, Home, End, PageUp/Down, F1-F12. Modifiers: Ctrl+, Alt+, Shift+".to_string()
            }
            DomainError::SessionLimitReached { .. } => {
                "Kill unused sessions with 'kill <session_id>' or increase limit with AGENT_TUI_MAX_SESSIONS env var.".to_string()
            }
            DomainError::LockTimeout { .. } => {
                "Session is busy. Try again in a moment, or run 'sessions' to check session status."
                    .to_string()
            }
            DomainError::PtyError { .. } => {
                "Terminal communication error. The session may have ended. Run 'sessions' to check status.".to_string()
            }
            DomainError::WaitTimeout { condition, .. } => {
                format!(
                    "Condition '{}' not met. The app may still be loading. Try 'wait --stable' or increase timeout with '-t'.",
                    condition
                )
            }
            DomainError::CommandNotFound { command } => {
                format!(
                    "Command '{}' not found. Check if the command exists and is in PATH.",
                    command
                )
            }
            DomainError::PermissionDenied { command } => {
                format!(
                    "Cannot execute '{}'. Check file permissions.",
                    command
                )
            }
            DomainError::Generic { .. } => {
                "Run 'snapshot -i' to see current screen state.".to_string()
            }
        }
    }
}

fn suggest_command_for_type(element_type: &str, element_ref: &str) -> String {
    let hint = match element_type {
        "button" | "menuitem" | "listitem" => format!("Try: click {}", element_ref),
        "checkbox" | "radio" => format!("Try: toggle {} or click {}", element_ref, element_ref),
        "input" => format!("Try: fill {} <value>", element_ref),
        "select" => format!("Try: select {} <option>", element_ref),
        _ => "Run 'snapshot -i' to see element types.".to_string(),
    };
    hint
}

impl From<SessionError> for DomainError {
    fn from(err: SessionError) -> Self {
        match err {
            SessionError::NotFound(id) => DomainError::SessionNotFound { session_id: id },
            SessionError::NoActiveSession => DomainError::NoActiveSession,
            SessionError::ElementNotFound(element_ref) => DomainError::ElementNotFound {
                element_ref,
                session_id: None,
            },
            SessionError::InvalidKey(key) => DomainError::InvalidKey { key },
            SessionError::LimitReached(max) => DomainError::SessionLimitReached { max },
            SessionError::Pty(pty_err) => DomainError::PtyError {
                operation: "pty".to_string(),
                reason: pty_err.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_not_found_code() {
        let err = DomainError::SessionNotFound {
            session_id: "abc123".into(),
        };
        assert_eq!(err.code(), error_codes::SESSION_NOT_FOUND);
    }

    #[test]
    fn test_element_not_found_category() {
        let err = DomainError::ElementNotFound {
            element_ref: "@btn1".into(),
            session_id: None,
        };
        assert_eq!(err.category(), ErrorCategory::NotFound);
    }

    #[test]
    fn test_lock_timeout_is_retryable() {
        let err = DomainError::LockTimeout {
            session_id: Some("abc".into()),
        };
        assert!(error_codes::is_retryable(err.code()));
    }

    #[test]
    fn test_element_not_found_not_retryable() {
        let err = DomainError::ElementNotFound {
            element_ref: "@btn1".into(),
            session_id: None,
        };
        assert!(!error_codes::is_retryable(err.code()));
    }

    #[test]
    fn test_context_includes_element_ref() {
        let err = DomainError::ElementNotFound {
            element_ref: "@btn5".into(),
            session_id: Some("sess1".into()),
        };
        let ctx = err.context();
        assert_eq!(ctx["element_ref"], "@btn5");
        assert_eq!(ctx["session_id"], "sess1");
    }

    #[test]
    fn test_wrong_element_type_context() {
        let err = DomainError::WrongElementType {
            element_ref: "@el1".into(),
            actual: "button".into(),
            expected: "input".into(),
        };
        let ctx = err.context();
        assert_eq!(ctx["element_ref"], "@el1");
        assert_eq!(ctx["actual_type"], "button");
        assert_eq!(ctx["expected_type"], "input");
    }

    #[test]
    fn test_suggestion_for_button() {
        let err = DomainError::WrongElementType {
            element_ref: "@btn1".into(),
            actual: "button".into(),
            expected: "input".into(),
        };
        assert!(err.suggestion().contains("click @btn1"));
    }

    #[test]
    fn test_from_session_error() {
        let session_err = SessionError::NotFound("test123".into());
        let domain_err: DomainError = session_err.into();
        assert_eq!(domain_err.code(), error_codes::SESSION_NOT_FOUND);
    }

    #[test]
    fn test_display_session_not_found() {
        let err = DomainError::SessionNotFound {
            session_id: "abc".into(),
        };
        assert_eq!(err.to_string(), "Session not found: abc");
    }

    #[test]
    fn test_display_wrong_element_type() {
        let err = DomainError::WrongElementType {
            element_ref: "@el1".into(),
            actual: "button".into(),
            expected: "input".into(),
        };
        assert_eq!(err.to_string(), "Element @el1 is a button not a input");
    }
}
