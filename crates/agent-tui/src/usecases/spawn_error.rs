use thiserror::Error;

#[derive(Error, Debug)]
pub enum SpawnError {
    #[error("Session limit reached: maximum {max} sessions allowed")]
    SessionLimitReached { max: usize },

    #[error("Session already exists: {session_id}")]
    SessionAlreadyExists { session_id: String },

    #[error("Command not found: {command}")]
    CommandNotFound { command: String },

    #[error("Permission denied: {command}")]
    PermissionDenied { command: String },

    #[error("PTY error during {operation}: {reason}")]
    PtyError { operation: String, reason: String },
}
