//! Use case error types.

use std::error::Error as StdError;
use thiserror::Error;

type ErrorSource = Box<dyn StdError + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnErrorKind {
    NotFound,
    PermissionDenied,
    Other,
}

#[derive(Error, Debug)]
pub enum TerminalError {
    #[error("Failed to open terminal: {reason}")]
    Open {
        reason: String,
        #[source]
        source: Option<ErrorSource>,
    },
    #[error("Failed to spawn process: {reason}")]
    Spawn {
        reason: String,
        kind: SpawnErrorKind,
    },
    #[error("Failed to write to terminal: {reason}")]
    Write {
        reason: String,
        #[source]
        source: Option<ErrorSource>,
    },
    #[error("Failed to read from terminal: {reason}")]
    Read {
        reason: String,
        #[source]
        source: Option<ErrorSource>,
    },
    #[error("Failed to resize terminal: {reason}")]
    Resize {
        reason: String,
        #[source]
        source: Option<ErrorSource>,
    },
}

impl TerminalError {
    pub fn operation(&self) -> &'static str {
        match self {
            TerminalError::Open { .. } => "open",
            TerminalError::Spawn { .. } => "spawn",
            TerminalError::Write { .. } => "write",
            TerminalError::Read { .. } => "read",
            TerminalError::Resize { .. } => "resize",
        }
    }

    pub fn reason(&self) -> &str {
        match self {
            TerminalError::Open { reason, .. }
            | TerminalError::Write { reason, .. }
            | TerminalError::Read { reason, .. }
            | TerminalError::Resize { reason, .. } => reason,
            TerminalError::Spawn { reason, .. } => reason,
        }
    }

    pub fn spawn_kind(&self) -> Option<SpawnErrorKind> {
        match self {
            TerminalError::Spawn { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            TerminalError::Read { .. } | TerminalError::Write { .. }
        )
    }
}

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Session already exists: {0}")]
    AlreadyExists(String),
    #[error("No active session")]
    NoActiveSession,
    #[error("Terminal error: {0}")]
    Terminal(#[from] TerminalError),
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    #[error("Session limit reached: maximum {0} sessions allowed")]
    LimitReached(usize),
    #[error("Persistence error during {operation}: {reason}")]
    Persistence {
        operation: String,
        reason: String,
        #[source]
        source: Option<ErrorSource>,
    },
}
