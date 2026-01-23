use std::sync::Arc;

use crate::domain::{KillOutput, SessionsOutput, SpawnInput, SpawnOutput};
use crate::error::SessionError;
use crate::repository::SessionRepository;
use crate::session::SessionId;

/// Use case for spawning a new session.
pub trait SpawnUseCase: Send + Sync {
    fn execute(&self, input: SpawnInput) -> Result<SpawnOutput, SessionError>;
}

/// Implementation of the spawn use case.
pub struct SpawnUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> SpawnUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> SpawnUseCase for SpawnUseCaseImpl<R> {
    fn execute(&self, input: SpawnInput) -> Result<SpawnOutput, SessionError> {
        let (session_id, pid) = self.repository.spawn(
            &input.command,
            &input.args,
            input.cwd.as_deref(),
            input.env.as_ref(),
            input.session_id,
            input.cols,
            input.rows,
        )?;

        Ok(SpawnOutput { session_id, pid })
    }
}

/// Use case for killing a session.
pub trait KillUseCase: Send + Sync {
    fn execute(&self, session_id: Option<String>) -> Result<KillOutput, SessionError>;
}

/// Implementation of the kill use case.
pub struct KillUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> KillUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> KillUseCase for KillUseCaseImpl<R> {
    fn execute(&self, session_id: Option<String>) -> Result<KillOutput, SessionError> {
        let session = self.repository.resolve(session_id.as_deref())?;
        let id = {
            let guard = session.lock().unwrap();
            SessionId::from(guard.id.as_str())
        };

        self.repository.kill(id.as_str())?;

        Ok(KillOutput {
            session_id: id,
            success: true,
        })
    }
}

/// Use case for listing sessions.
pub trait SessionsUseCase: Send + Sync {
    fn execute(&self) -> SessionsOutput;
}

/// Implementation of the sessions list use case.
pub struct SessionsUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> SessionsUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> SessionsUseCase for SessionsUseCaseImpl<R> {
    fn execute(&self) -> SessionsOutput {
        let sessions = self.repository.list();
        let active_session = self.repository.active_session_id();

        SessionsOutput {
            sessions,
            active_session,
        }
    }
}
