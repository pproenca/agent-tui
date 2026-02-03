use crate::common::error_codes::{self, ErrorCategory};
use crate::usecases::SpawnError;
use crate::usecases::ports::{LivePreviewError, PtyError, SessionError};
use serde_json::{Value, json};
use thiserror::Error;

impl SessionError {
    pub fn code(&self) -> i32 {
        match self {
            SessionError::NotFound(_) => error_codes::SESSION_NOT_FOUND,
            SessionError::AlreadyExists(_) => error_codes::SESSION_ALREADY_EXISTS,
            SessionError::NoActiveSession => error_codes::NO_ACTIVE_SESSION,
            SessionError::InvalidKey(_) => error_codes::INVALID_KEY,
            SessionError::LimitReached(_) => error_codes::SESSION_LIMIT,
            SessionError::Pty(_) => error_codes::PTY_ERROR,
            SessionError::Persistence { .. } => error_codes::PERSISTENCE_ERROR,
        }
    }

    pub fn category(&self) -> ErrorCategory {
        error_codes::category_for_code(self.code())
    }

    pub fn context(&self) -> Value {
        match self {
            SessionError::NotFound(id) => json!({ "session_id": id }),
            SessionError::AlreadyExists(id) => json!({ "session_id": id }),
            SessionError::NoActiveSession => json!({}),
            SessionError::InvalidKey(key) => json!({ "key": key }),
            SessionError::LimitReached(max) => json!({ "max_sessions": max }),
            SessionError::Pty(pty_err) => json!({
                "operation": pty_err.operation(),
                "reason": pty_err.reason()
            }),
            SessionError::Persistence { operation, reason } => {
                json!({ "operation": operation, "reason": reason })
            }
        }
    }

    pub fn suggestion(&self) -> String {
        match self {
            SessionError::NotFound(_)
            | SessionError::AlreadyExists(_)
            | SessionError::NoActiveSession => {
                "Run 'sessions' to list active sessions or 'spawn <cmd>' to start a new one."
                    .to_string()
            }
            SessionError::InvalidKey(_) => {
                "Supported keys: Enter, Tab, Escape, Backspace, Delete, ArrowUp/Down/Left/Right, Home, End, PageUp/Down, F1-F12. Modifiers: Ctrl+, Alt+, Shift+".to_string()
            }
            SessionError::LimitReached(_) => {
                "Kill unused sessions with 'kill <session_id>' or increase limit with AGENT_TUI_MAX_SESSIONS env var.".to_string()
            }
            SessionError::Pty(pty_err) => match pty_err {
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
            },
            SessionError::Persistence { .. } => {
                "Persistence error is non-fatal. Session continues to operate normally.".to_string()
            }
        }
    }

    pub fn is_retryable(&self) -> bool {
        match self {
            SessionError::Pty(pty_err) => pty_err.is_retryable(),
            SessionError::Persistence { .. } => true,
            _ => error_codes::is_retryable(self.code()),
        }
    }
}

impl LivePreviewError {
    pub fn code(&self) -> i32 {
        match self {
            LivePreviewError::Session(err) => err.code(),
            LivePreviewError::AlreadyRunning => error_codes::LIVE_PREVIEW_ALREADY_RUNNING,
            LivePreviewError::NotRunning => error_codes::LIVE_PREVIEW_NOT_RUNNING,
            LivePreviewError::InvalidListenAddress(_) => error_codes::LIVE_PREVIEW_INVALID_LISTEN,
            LivePreviewError::BindFailed { .. } => error_codes::LIVE_PREVIEW_BIND_FAILED,
        }
    }

    pub fn category(&self) -> ErrorCategory {
        error_codes::category_for_code(self.code())
    }

    pub fn context(&self) -> Value {
        match self {
            LivePreviewError::Session(err) => err.context(),
            LivePreviewError::AlreadyRunning => json!({}),
            LivePreviewError::NotRunning => json!({}),
            LivePreviewError::InvalidListenAddress(addr) => json!({ "listen": addr }),
            LivePreviewError::BindFailed { addr, reason } => {
                json!({ "listen": addr, "reason": reason })
            }
        }
    }

    pub fn suggestion(&self) -> String {
        match self {
            LivePreviewError::Session(err) => err.suggestion(),
            LivePreviewError::AlreadyRunning => {
                "Run 'live status' to see the API endpoints or 'daemon stop' to stop it."
                    .to_string()
            }
            LivePreviewError::NotRunning => {
                "Restart the daemon to start the live preview API.".to_string()
            }
            LivePreviewError::InvalidListenAddress(_) => {
                "Use a valid HOST:PORT or IP:PORT listen address via AGENT_TUI_API_LISTEN."
                    .to_string()
            }
            LivePreviewError::BindFailed { .. } => {
                "Check if the port is available or choose another AGENT_TUI_API_LISTEN.".to_string()
            }
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, LivePreviewError::BindFailed { .. })
    }
}

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },

    #[error("Session already exists: {session_id}")]
    SessionAlreadyExists { session_id: String },

    #[error("No active session")]
    NoActiveSession,

    #[error("Invalid key: {key}")]
    InvalidKey { key: String },

    #[error("Session limit reached: maximum {max} sessions allowed")]
    SessionLimitReached { max: usize },

    #[error("Lock timeout{}", session_id.as_ref().map(|id| format!(" for session: {}", id)).unwrap_or_default())]
    LockTimeout { session_id: Option<String> },

    #[error("PTY error during {operation}: {reason}")]
    PtyError { operation: String, reason: String },

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

    #[error("{message}")]
    Generic { message: String },
}

impl DomainError {
    pub fn code(&self) -> i32 {
        match self {
            DomainError::SessionNotFound { .. } => error_codes::SESSION_NOT_FOUND,
            DomainError::SessionAlreadyExists { .. } => error_codes::SESSION_ALREADY_EXISTS,
            DomainError::NoActiveSession => error_codes::NO_ACTIVE_SESSION,
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

    pub fn category(&self) -> ErrorCategory {
        error_codes::category_for_code(self.code())
    }

    pub fn context(&self) -> Value {
        match self {
            DomainError::SessionNotFound { session_id } => {
                json!({ "session_id": session_id })
            }
            DomainError::SessionAlreadyExists { session_id } => {
                json!({ "session_id": session_id })
            }
            DomainError::NoActiveSession => json!({}),
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

    pub fn suggestion(&self) -> String {
        match self {
            DomainError::SessionNotFound { .. }
            | DomainError::SessionAlreadyExists { .. }
            | DomainError::NoActiveSession => {
                "Run 'sessions' to list active sessions or 'spawn <cmd>' to start a new one."
                    .to_string()
            }
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
                "Run 'screenshot' to see current screen state.".to_string()
            }
        }
    }
}

impl From<SessionError> for DomainError {
    fn from(err: SessionError) -> Self {
        match err {
            SessionError::NotFound(id) => DomainError::SessionNotFound { session_id: id },
            SessionError::AlreadyExists(id) => DomainError::SessionAlreadyExists { session_id: id },
            SessionError::NoActiveSession => DomainError::NoActiveSession,
            SessionError::InvalidKey(key) => DomainError::InvalidKey { key },
            SessionError::LimitReached(max) => DomainError::SessionLimitReached { max },
            SessionError::Pty(pty_err) => DomainError::PtyError {
                operation: pty_err.operation().to_string(),
                reason: pty_err.reason().to_string(),
            },
            SessionError::Persistence { operation, reason } => DomainError::Generic {
                message: format!("Persistence error during {}: {}", operation, reason),
            },
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
            SpawnError::PtyError { operation, reason } => {
                DomainError::PtyError { operation, reason }
            }
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
    fn test_lock_timeout_is_retryable() {
        let err = DomainError::LockTimeout {
            session_id: Some("abc".into()),
        };
        assert!(error_codes::is_retryable(err.code()));
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
    fn test_session_error_not_found_code() {
        let err = SessionError::NotFound("abc123".into());
        assert_eq!(err.code(), error_codes::SESSION_NOT_FOUND);
    }

    #[test]
    fn test_session_error_no_active_session_code() {
        let err = SessionError::NoActiveSession;
        assert_eq!(err.code(), error_codes::NO_ACTIVE_SESSION);
    }

    #[test]
    fn test_session_error_invalid_key_code() {
        let err = SessionError::InvalidKey("BadKey".into());
        assert_eq!(err.code(), error_codes::INVALID_KEY);
    }

    #[test]
    fn test_session_error_limit_reached_code() {
        let err = SessionError::LimitReached(16);
        assert_eq!(err.code(), error_codes::SESSION_LIMIT);
    }

    #[test]
    fn test_session_error_category() {
        let err = SessionError::NotFound("abc".into());
        assert_eq!(err.category(), ErrorCategory::NotFound);

        let err = SessionError::InvalidKey("x".into());
        assert_eq!(err.category(), ErrorCategory::InvalidInput);

        let err = SessionError::LimitReached(10);
        assert_eq!(err.category(), ErrorCategory::Busy);
    }

    #[test]
    fn test_session_error_context() {
        let err = SessionError::NotFound("sess123".into());
        let ctx = err.context();
        assert_eq!(ctx["session_id"], "sess123");

        let err = SessionError::LimitReached(16);
        let ctx = err.context();
        assert_eq!(ctx["max_sessions"], 16);
    }

    #[test]
    fn test_session_error_suggestion() {
        let err = SessionError::NotFound("x".into());
        assert!(err.suggestion().contains("sessions"));

        let err = SessionError::InvalidKey("x".into());
        assert!(err.suggestion().contains("Enter"));
    }

    #[test]
    fn test_session_error_is_retryable() {
        assert!(!SessionError::NotFound("x".into()).is_retryable());
        assert!(!SessionError::NoActiveSession.is_retryable());
        assert!(!SessionError::InvalidKey("x".into()).is_retryable());
    }

    #[test]
    fn test_session_error_persistence_code() {
        let err = SessionError::Persistence {
            operation: "save".into(),
            reason: "disk full".into(),
        };
        assert_eq!(err.code(), error_codes::PERSISTENCE_ERROR);
    }

    #[test]
    fn test_session_error_persistence_context() {
        let err = SessionError::Persistence {
            operation: "write_json".into(),
            reason: "permission denied".into(),
        };
        let ctx = err.context();
        assert_eq!(ctx["operation"], "write_json");
        assert_eq!(ctx["reason"], "permission denied");
    }

    #[test]
    fn test_session_error_persistence_is_retryable() {
        let err = SessionError::Persistence {
            operation: "save".into(),
            reason: "disk full".into(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_session_error_persistence_display() {
        let err = SessionError::Persistence {
            operation: "write".into(),
            reason: "disk full".into(),
        };
        assert_eq!(err.to_string(), "Persistence error during write: disk full");
    }

    #[test]
    fn test_pty_error_conversion_preserves_context() {
        let pty_err = PtyError::Write("broken pipe".into());
        let session_err = SessionError::Pty(pty_err);
        let domain_err: DomainError = session_err.into();

        match domain_err {
            DomainError::PtyError { operation, reason } => {
                assert_eq!(operation, "write");
                assert_eq!(reason, "broken pipe");
            }
            _ => panic!("Expected PtyError variant"),
        }
    }
}
