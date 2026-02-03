use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnErrorKind {
    NotFound,
    PermissionDenied,
    Other,
}

#[derive(Error, Debug)]
pub enum PtyError {
    #[error("Failed to open PTY: {0}")]
    Open(String),
    #[error("Failed to spawn process: {reason}")]
    Spawn {
        reason: String,
        kind: SpawnErrorKind,
    },
    #[error("Failed to write to PTY: {0}")]
    Write(String),
    #[error("Failed to read from PTY: {0}")]
    Read(String),
    #[error("Failed to resize PTY: {0}")]
    Resize(String),
}

impl PtyError {
    pub fn operation(&self) -> &'static str {
        match self {
            PtyError::Open(_) => "open",
            PtyError::Spawn { .. } => "spawn",
            PtyError::Write(_) => "write",
            PtyError::Read(_) => "read",
            PtyError::Resize(_) => "resize",
        }
    }

    pub fn reason(&self) -> &str {
        match self {
            PtyError::Open(r) | PtyError::Write(r) | PtyError::Read(r) | PtyError::Resize(r) => r,
            PtyError::Spawn { reason, .. } => reason,
        }
    }

    pub fn spawn_kind(&self) -> Option<SpawnErrorKind> {
        match self {
            PtyError::Spawn { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, PtyError::Read(_) | PtyError::Write(_))
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
    #[error("PTY error: {0}")]
    Pty(#[from] PtyError),
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    #[error("Session limit reached: maximum {0} sessions allowed")]
    LimitReached(usize),
    #[error("Persistence error during {operation}: {reason}")]
    Persistence { operation: String, reason: String },
}

#[derive(Error, Debug)]
pub enum LivePreviewError {
    #[error("{0}")]
    Session(#[from] SessionError),
    #[error("Live preview already running")]
    AlreadyRunning,
    #[error("Live preview is not running")]
    NotRunning,
    #[error("Invalid listen address: {0}")]
    InvalidListenAddress(String),
    #[error("Failed to bind live preview listener at {addr}: {reason}")]
    BindFailed { addr: String, reason: String },
}
