//! Mock error types for use case tests.

use crate::usecases::ports::SessionError;
use crate::usecases::ports::SpawnErrorKind;
use crate::usecases::ports::TerminalError;

#[derive(Debug, Clone, Default)]
pub enum MockError {
    #[default]
    NoActiveSession,
    NotFound(String),
    NotRunning(String),
    LimitReached(usize),
    Terminal {
        kind: SpawnErrorKind,
        reason: String,
    },
}

impl MockError {
    pub fn to_session_error(&self) -> SessionError {
        match self {
            MockError::NoActiveSession => SessionError::NoActiveSession,
            MockError::NotFound(id) => SessionError::NotFound(id.clone()),
            MockError::NotRunning(session_id) => SessionError::NotRunning {
                session_id: session_id.clone(),
            },
            MockError::LimitReached(max) => SessionError::LimitReached(*max),
            MockError::Terminal { kind, reason } => SessionError::Terminal(TerminalError::Spawn {
                reason: reason.clone(),
                kind: *kind,
            }),
        }
    }
}

#[cfg(test)]
#[path = "mock_error_tests.rs"]
mod tests;
