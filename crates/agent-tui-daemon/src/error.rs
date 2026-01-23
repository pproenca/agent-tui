//! Domain errors for daemon operations.
//!
//! These errors are mapped to specific JSON-RPC error codes and include
//! structured context for AI agents to handle programmatically.

use agent_tui_ipc::error_codes::{self, ErrorCategory};
use agent_tui_terminal::PtyError;
use serde_json::{Value, json};
use thiserror::Error;

/// Session-level errors with structured context for AI agents.
#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("No active session")]
    NoActiveSession,
    #[error("PTY error: {0}")]
    Pty(#[from] PtyError),
    #[error("Element not found: {0}")]
    ElementNotFound(String),
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    #[error("Session limit reached: maximum {0} sessions allowed")]
    LimitReached(usize),
    #[error("Persistence error during {operation}: {reason}")]
    Persistence { operation: String, reason: String },
}

impl SessionError {
    /// Returns the JSON-RPC error code for this error.
    pub fn code(&self) -> i32 {
        match self {
            SessionError::NotFound(_) => error_codes::SESSION_NOT_FOUND,
            SessionError::NoActiveSession => error_codes::NO_ACTIVE_SESSION,
            SessionError::ElementNotFound(_) => error_codes::ELEMENT_NOT_FOUND,
            SessionError::InvalidKey(_) => error_codes::INVALID_KEY,
            SessionError::LimitReached(_) => error_codes::SESSION_LIMIT,
            SessionError::Pty(_) => error_codes::PTY_ERROR,
            SessionError::Persistence { .. } => error_codes::PERSISTENCE_ERROR,
        }
    }

    /// Returns the error category for programmatic handling.
    pub fn category(&self) -> ErrorCategory {
        error_codes::category_for_code(self.code())
    }

    /// Returns structured context about the error for debugging.
    pub fn context(&self) -> Value {
        match self {
            SessionError::NotFound(id) => json!({ "session_id": id }),
            SessionError::NoActiveSession => json!({}),
            SessionError::ElementNotFound(element_ref) => json!({ "element_ref": element_ref }),
            SessionError::InvalidKey(key) => json!({ "key": key }),
            SessionError::LimitReached(max) => json!({ "max_sessions": max }),
            SessionError::Pty(pty_err) => pty_err.context(),
            SessionError::Persistence { operation, reason } => {
                json!({ "operation": operation, "reason": reason })
            }
        }
    }

    /// Returns a helpful suggestion for resolving the error.
    pub fn suggestion(&self) -> String {
        match self {
            SessionError::NotFound(_) | SessionError::NoActiveSession => {
                "Run 'sessions' to list active sessions or 'spawn <cmd>' to start a new one."
                    .to_string()
            }
            SessionError::ElementNotFound(element_ref) => {
                format!(
                    "Element '{}' not found. Run 'snapshot -i' to see current elements and their refs.",
                    element_ref
                )
            }
            SessionError::InvalidKey(_) => {
                "Supported keys: Enter, Tab, Escape, Backspace, Delete, ArrowUp/Down/Left/Right, Home, End, PageUp/Down, F1-F12. Modifiers: Ctrl+, Alt+, Shift+".to_string()
            }
            SessionError::LimitReached(_) => {
                "Kill unused sessions with 'kill <session_id>' or increase limit with AGENT_TUI_MAX_SESSIONS env var.".to_string()
            }
            SessionError::Pty(pty_err) => pty_err.suggestion(),
            SessionError::Persistence { .. } => {
                "Persistence error is non-fatal. Session continues to operate normally.".to_string()
            }
        }
    }

    /// Returns whether this error is potentially transient and may succeed on retry.
    pub fn is_retryable(&self) -> bool {
        match self {
            SessionError::Pty(pty_err) => pty_err.is_retryable(),
            SessionError::Persistence { .. } => true,
            _ => error_codes::is_retryable(self.code()),
        }
    }
}

/// Daemon startup and lifecycle errors.
#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("Failed to bind socket: {0}")]
    SocketBind(String),
    #[error("Another daemon instance is already running")]
    AlreadyRunning,
    #[error("Failed to acquire lock: {0}")]
    LockFailed(String),
    #[error("Failed to setup signal handler: {0}")]
    SignalSetup(String),
    #[error("Failed to create thread pool: {0}")]
    ThreadPool(String),
}

impl DaemonError {
    /// Returns the JSON-RPC error code for this error.
    pub fn code(&self) -> i32 {
        error_codes::DAEMON_ERROR
    }

    /// Returns the error category for programmatic handling.
    pub fn category(&self) -> ErrorCategory {
        ErrorCategory::External
    }

    /// Returns structured context about the error for debugging.
    pub fn context(&self) -> Value {
        match self {
            DaemonError::SocketBind(reason) => {
                json!({ "operation": "socket_bind", "reason": reason })
            }
            DaemonError::AlreadyRunning => {
                json!({ "operation": "startup", "reason": "another instance running" })
            }
            DaemonError::LockFailed(reason) => json!({ "operation": "lock", "reason": reason }),
            DaemonError::SignalSetup(reason) => {
                json!({ "operation": "signal_setup", "reason": reason })
            }
            DaemonError::ThreadPool(reason) => {
                json!({ "operation": "thread_pool", "reason": reason })
            }
        }
    }

    /// Returns a helpful suggestion for resolving the error.
    pub fn suggestion(&self) -> String {
        match self {
            DaemonError::SocketBind(_) => {
                "Check if the socket directory is writable. Try: rm /tmp/agent-tui.sock".to_string()
            }
            DaemonError::AlreadyRunning => {
                "Another daemon is running. Use 'agent-tui sessions' to connect or kill existing daemon.".to_string()
            }
            DaemonError::LockFailed(_) => {
                "Lock file issue. Try removing the lock file: rm /tmp/agent-tui.sock.lock".to_string()
            }
            DaemonError::SignalSetup(_) => {
                "Signal handler setup failed. Check system signal configuration.".to_string()
            }
            DaemonError::ThreadPool(_) => {
                "Thread pool creation failed. Check system thread limits (ulimit -u).".to_string()
            }
        }
    }

    /// Returns whether this error is potentially transient and may succeed on retry.
    pub fn is_retryable(&self) -> bool {
        matches!(self, DaemonError::LockFailed(_))
    }
}

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
                operation: pty_err.operation().to_string(),
                reason: pty_err.reason().to_string(),
            },
            SessionError::Persistence { operation, reason } => DomainError::Generic {
                message: format!("Persistence error during {}: {}", operation, reason),
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

    // SessionError tests
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
    fn test_session_error_element_not_found_code() {
        let err = SessionError::ElementNotFound("@btn1".into());
        assert_eq!(err.code(), error_codes::ELEMENT_NOT_FOUND);
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

        let err = SessionError::ElementNotFound("@btn5".into());
        let ctx = err.context();
        assert_eq!(ctx["element_ref"], "@btn5");

        let err = SessionError::LimitReached(16);
        let ctx = err.context();
        assert_eq!(ctx["max_sessions"], 16);
    }

    #[test]
    fn test_session_error_suggestion() {
        let err = SessionError::NotFound("x".into());
        assert!(err.suggestion().contains("sessions"));

        let err = SessionError::ElementNotFound("@btn1".into());
        assert!(err.suggestion().contains("snapshot"));

        let err = SessionError::InvalidKey("x".into());
        assert!(err.suggestion().contains("Enter"));
    }

    #[test]
    fn test_session_error_is_retryable() {
        assert!(!SessionError::NotFound("x".into()).is_retryable());
        assert!(!SessionError::NoActiveSession.is_retryable());
        assert!(!SessionError::ElementNotFound("x".into()).is_retryable());
        assert!(!SessionError::InvalidKey("x".into()).is_retryable());
    }

    // SessionError::Persistence tests
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

    // DaemonError tests
    #[test]
    fn test_daemon_error_socket_bind() {
        let err = DaemonError::SocketBind("address in use".into());
        assert_eq!(err.code(), error_codes::DAEMON_ERROR);
        assert_eq!(err.category(), ErrorCategory::External);
        assert!(err.suggestion().contains("socket"));
    }

    #[test]
    fn test_daemon_error_already_running() {
        let err = DaemonError::AlreadyRunning;
        assert_eq!(err.code(), error_codes::DAEMON_ERROR);
        assert!(err.suggestion().contains("Another daemon"));
    }

    #[test]
    fn test_daemon_error_lock_failed() {
        let err = DaemonError::LockFailed("permission denied".into());
        assert_eq!(err.code(), error_codes::DAEMON_ERROR);
        assert!(err.is_retryable());
    }

    #[test]
    fn test_daemon_error_not_retryable() {
        assert!(!DaemonError::SocketBind("x".into()).is_retryable());
        assert!(!DaemonError::AlreadyRunning.is_retryable());
        assert!(!DaemonError::SignalSetup("x".into()).is_retryable());
        assert!(!DaemonError::ThreadPool("x".into()).is_retryable());
    }

    #[test]
    fn test_daemon_error_context() {
        let err = DaemonError::SocketBind("address in use".into());
        let ctx = err.context();
        assert_eq!(ctx["operation"], "socket_bind");
        assert_eq!(ctx["reason"], "address in use");
    }

    #[test]
    fn test_daemon_error_display() {
        let err = DaemonError::AlreadyRunning;
        assert_eq!(
            err.to_string(),
            "Another daemon instance is already running"
        );
    }

    // PtyError conversion test
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
