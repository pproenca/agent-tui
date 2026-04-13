//! Daemon adapter error types.

use crate::common::error_codes;
use crate::common::error_codes::ErrorCategory;
use crate::usecases::SpawnError;
use crate::usecases::ports::SessionError;
use crate::usecases::ports::SpawnErrorKind;
use crate::usecases::ports::TerminalError;
use serde_json::Value;
use serde_json::json;
use thiserror::Error;

/// Adapter-layer trait for presenting errors to external consumers.
///
/// This trait keeps presentation concerns (error codes, categories, suggestions)
/// in the adapter layer rather than extending inner-layer types with inherent methods.
pub trait ErrorPresentation {
    fn code(&self) -> i32;
    fn category(&self) -> ErrorCategory;
    fn context(&self) -> Value;
    fn suggestion(&self) -> String;
    fn is_retryable(&self) -> bool;
}

impl ErrorPresentation for SessionError {
    fn code(&self) -> i32 {
        match self {
            SessionError::NotFound(_) => error_codes::SESSION_NOT_FOUND,
            SessionError::NotRunning { .. } => error_codes::SESSION_NOT_FOUND,
            SessionError::AlreadyExists(_) => error_codes::SESSION_ALREADY_EXISTS,
            SessionError::NoActiveSession => error_codes::NO_ACTIVE_SESSION,
            SessionError::InvalidKey(_) => error_codes::INVALID_KEY,
            SessionError::InvalidInput { .. } => error_codes::INVALID_INPUT,
            SessionError::LimitReached(_) => error_codes::SESSION_LIMIT,
            SessionError::Terminal(_) => error_codes::PTY_ERROR,
            SessionError::Persistence { .. } => error_codes::PERSISTENCE_ERROR,
        }
    }

    fn category(&self) -> ErrorCategory {
        error_codes::category_for_code(self.code())
    }

    fn context(&self) -> Value {
        match self {
            SessionError::NotFound(id) => json!({ "session_id": id }),
            SessionError::NotRunning { session_id } => {
                json!({ "session_id": session_id, "state": "not_running" })
            }
            SessionError::AlreadyExists(id) => json!({ "session_id": id }),
            SessionError::NoActiveSession => json!({}),
            SessionError::InvalidKey(key) => json!({ "key": key }),
            SessionError::InvalidInput { field, reason } => {
                json!({ "field": field, "reason": reason })
            }
            SessionError::LimitReached(max) => json!({ "max_sessions": max }),
            SessionError::Terminal(terminal_err) => json!({
                "operation": terminal_err.operation(),
                "reason": terminal_err.reason()
            }),
            SessionError::Persistence {
                operation, reason, ..
            } => {
                json!({ "operation": operation, "reason": reason })
            }
        }
    }

    fn suggestion(&self) -> String {
        match self {
            SessionError::NotFound(_)
            | SessionError::NotRunning { .. }
            | SessionError::AlreadyExists(_)
            | SessionError::NoActiveSession => {
                if matches!(self, SessionError::NotRunning { .. }) {
                    "Run 'sessions' to inspect the stopped session, or 'restart <session_id>' to start it again."
                        .to_string()
                } else {
                    "Run 'sessions' to list active sessions or 'spawn <cmd>' to start a new one."
                        .to_string()
                }
            }
            SessionError::InvalidKey(_) => {
                "Supported keys: Enter, Tab, Escape, Backspace, Delete, ArrowUp/Down/Left/Right, Home, End, PageUp/Down, F1-F12. Modifiers: Ctrl+, Alt+, Shift+".to_string()
            }
            SessionError::InvalidInput { .. } => {
                "Adjust the invalid input and retry the command.".to_string()
            }
            SessionError::LimitReached(_) => {
                "Kill unused sessions with 'kill <session_id>' or increase limit with AGENT_TUI_MAX_SESSIONS env var.".to_string()
            }
            SessionError::Terminal(terminal_err) => match terminal_err {
                TerminalError::Open { .. } => {
                    "Terminal allocation failed. Check system resource limits (ulimit -n) or try restarting."
                        .to_string()
                }
                TerminalError::Spawn { kind, .. } => match kind {
                    SpawnErrorKind::NotFound => {
                        "Command not found. Check if the command exists and is in PATH."
                            .to_string()
                    }
                    SpawnErrorKind::PermissionDenied => {
                        "Permission denied. Check file permissions.".to_string()
                    }
                    SpawnErrorKind::Other => {
                        "Process spawn failed. Check command syntax and permissions.".to_string()
                    }
                },
                TerminalError::Write { .. } => {
                    "Failed to send input to terminal. The session may have ended. Run 'sessions' to check status."
                        .to_string()
                }
                TerminalError::Read { .. } => {
                    "Failed to read terminal output. The session may have ended. Run 'sessions' to check status."
                        .to_string()
                }
                TerminalError::Resize { .. } => {
                    "Failed to resize terminal. Try again or restart the session.".to_string()
                }
            },
            SessionError::Persistence { .. } => {
                "Persistence error is non-fatal. Session continues to operate normally.".to_string()
            }
        }
    }

    fn is_retryable(&self) -> bool {
        match self {
            SessionError::Terminal(terminal_err) => terminal_err.is_retryable(),
            SessionError::Persistence { .. } => true,
            _ => error_codes::is_retryable(self.code()),
        }
    }
}

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },

    #[error("Session not running: {session_id}")]
    SessionNotRunning { session_id: String },

    #[error("Session already exists: {session_id}")]
    SessionAlreadyExists { session_id: String },

    #[error("No active session")]
    NoActiveSession,

    #[error("Invalid key: {key}")]
    InvalidKey { key: String },

    #[error("Invalid input for {field}: {reason}")]
    InvalidInput { field: String, reason: String },

    #[error("Session limit reached: maximum {max} sessions allowed")]
    SessionLimitReached { max: usize },

    #[error("Lock timeout{}", session_id.as_ref().map(|id| format!(" for session: {id}")).unwrap_or_default())]
    LockTimeout { session_id: Option<String> },

    #[error("Terminal error during {operation}: {reason}")]
    TerminalError { operation: String, reason: String },

    #[error("Timeout waiting for: {condition}")]
    WaitTimeout {
        condition: String,
        elapsed_ms: u64,
        timeout_ms: u64,
    },

    #[error("Command not found: {command}")]
    CommandNotFound { command: String },

    #[error("Permission denied: {command}")]
    PermissionDenied { command: String },

    #[error("Persistence error during {operation}: {reason}")]
    PersistenceError { operation: String, reason: String },

    #[error("{message}")]
    Generic { message: String },
}

impl DomainError {
    pub fn code(&self) -> i32 {
        match self {
            DomainError::SessionNotFound { .. } => error_codes::SESSION_NOT_FOUND,
            DomainError::SessionNotRunning { .. } => error_codes::SESSION_NOT_FOUND,
            DomainError::SessionAlreadyExists { .. } => error_codes::SESSION_ALREADY_EXISTS,
            DomainError::NoActiveSession => error_codes::NO_ACTIVE_SESSION,
            DomainError::InvalidKey { .. } => error_codes::INVALID_KEY,
            DomainError::InvalidInput { .. } => error_codes::INVALID_INPUT,
            DomainError::SessionLimitReached { .. } => error_codes::SESSION_LIMIT,
            DomainError::LockTimeout { .. } => error_codes::LOCK_TIMEOUT,
            DomainError::TerminalError { .. } => error_codes::PTY_ERROR,
            DomainError::WaitTimeout { .. } => error_codes::WAIT_TIMEOUT,
            DomainError::CommandNotFound { .. } => error_codes::COMMAND_NOT_FOUND,
            DomainError::PermissionDenied { .. } => error_codes::PERMISSION_DENIED,
            DomainError::PersistenceError { .. } => error_codes::PERSISTENCE_ERROR,
            DomainError::Generic { .. } => error_codes::GENERIC_ERROR,
        }
    }

    pub fn category(&self) -> ErrorCategory {
        error_codes::category_for_code(self.code())
    }

    pub fn context(&self) -> Value {
        match self {
            DomainError::SessionNotFound { session_id } => {
                json!({ "session_id": session_id })
            }
            DomainError::SessionNotRunning { session_id } => {
                json!({ "session_id": session_id, "state": "not_running" })
            }
            DomainError::SessionAlreadyExists { session_id } => {
                json!({ "session_id": session_id })
            }
            DomainError::NoActiveSession => json!({}),
            DomainError::InvalidKey { key } => {
                json!({ "key": key })
            }
            DomainError::InvalidInput { field, reason } => {
                json!({ "field": field, "reason": reason })
            }
            DomainError::SessionLimitReached { max } => {
                json!({ "max_sessions": max })
            }
            DomainError::LockTimeout { session_id } => match session_id {
                Some(id) => json!({ "session_id": id }),
                None => json!({}),
            },
            DomainError::TerminalError { operation, reason } => {
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
            DomainError::PersistenceError { operation, reason } => {
                json!({ "operation": operation, "reason": reason })
            }
            DomainError::Generic { message } => {
                json!({ "message": message })
            }
        }
    }

    pub fn suggestion(&self) -> String {
        match self {
            DomainError::SessionNotFound { .. }
            | DomainError::SessionNotRunning { .. }
            | DomainError::SessionAlreadyExists { .. }
            | DomainError::NoActiveSession => {
                if matches!(self, DomainError::SessionNotRunning { .. }) {
                    "Run 'sessions' to inspect the stopped session, or 'restart <session_id>' to start it again."
                        .to_string()
                } else {
                    "Run 'sessions' to list active sessions or 'spawn <cmd>' to start a new one."
                        .to_string()
                }
            }
            DomainError::InvalidKey { .. } => {
                "Supported keys: Enter, Tab, Escape, Backspace, Delete, ArrowUp/Down/Left/Right, Home, End, PageUp/Down, F1-F12. Modifiers: Ctrl+, Alt+, Shift+".to_string()
            }
            DomainError::InvalidInput { .. } => {
                "Adjust the invalid input and retry the command.".to_string()
            }
            DomainError::SessionLimitReached { .. } => {
                "Kill unused sessions with 'kill <session_id>' or increase limit with AGENT_TUI_MAX_SESSIONS env var.".to_string()
            }
            DomainError::LockTimeout { .. } => {
                "Session is busy. Try again in a moment, or run 'sessions' to check session status."
                    .to_string()
            }
            DomainError::TerminalError { .. } => {
                "Terminal communication error. The session may have ended. Run 'sessions' to check status.".to_string()
            }
            DomainError::WaitTimeout { condition, .. } => {
                format!(
                    "Condition '{condition}' not met. The app may still be loading. Try 'wait --stable' or increase timeout with '-t'."
                )
            }
            DomainError::CommandNotFound { command } => {
                format!(
                    "Command '{command}' not found. Check if the command exists and is in PATH."
                )
            }
            DomainError::PermissionDenied { command } => {
                format!(
                    "Cannot execute '{command}'. Check file permissions."
                )
            }
            DomainError::PersistenceError { .. } => {
                "Persistence error is non-fatal. Session continues to operate normally."
                    .to_string()
            }
            DomainError::Generic { .. } => {
                "Run 'screenshot' to see current screen state.".to_string()
            }
        }
    }
}

impl From<SessionError> for DomainError {
    fn from(err: SessionError) -> Self {
        match err {
            SessionError::NotFound(id) => DomainError::SessionNotFound { session_id: id },
            SessionError::NotRunning { session_id } => {
                DomainError::SessionNotRunning { session_id }
            }
            SessionError::AlreadyExists(id) => DomainError::SessionAlreadyExists { session_id: id },
            SessionError::NoActiveSession => DomainError::NoActiveSession,
            SessionError::InvalidKey(key) => DomainError::InvalidKey { key },
            SessionError::InvalidInput { field, reason } => {
                DomainError::InvalidInput { field, reason }
            }
            SessionError::LimitReached(max) => DomainError::SessionLimitReached { max },
            SessionError::Terminal(terminal_err) => DomainError::TerminalError {
                operation: terminal_err.operation().to_string(),
                reason: terminal_err.reason().to_string(),
            },
            SessionError::Persistence {
                operation, reason, ..
            } => DomainError::PersistenceError { operation, reason },
        }
    }
}

impl From<SpawnError> for DomainError {
    fn from(err: SpawnError) -> Self {
        match err {
            SpawnError::SessionLimitReached { max } => DomainError::SessionLimitReached { max },
            SpawnError::SessionAlreadyExists { session_id } => {
                DomainError::SessionAlreadyExists { session_id }
            }
            SpawnError::CommandNotFound { command } => DomainError::CommandNotFound { command },
            SpawnError::PermissionDenied { command } => DomainError::PermissionDenied { command },
            SpawnError::TerminalError { operation, reason } => {
                DomainError::TerminalError { operation, reason }
            }
        }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
