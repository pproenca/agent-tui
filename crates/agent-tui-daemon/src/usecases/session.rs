use std::sync::Arc;

use crate::domain::{
    KillOutput, ResizeInput, ResizeOutput, SessionsOutput, SpawnInput, SpawnOutput,
};
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

/// Use case for restarting a session.
pub trait RestartUseCase: Send + Sync {
    fn execute(&self, session_id: Option<String>) -> Result<RestartOutput, SessionError>;
}

/// Output from restarting a session.
#[derive(Debug, Clone)]
pub struct RestartOutput {
    pub old_session_id: SessionId,
    pub new_session_id: SessionId,
    pub command: String,
    pub pid: u32,
}

/// Implementation of the restart use case.
pub struct RestartUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> RestartUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> RestartUseCase for RestartUseCaseImpl<R> {
    fn execute(&self, session_id: Option<String>) -> Result<RestartOutput, SessionError> {
        let session = self.repository.resolve(session_id.as_deref())?;

        let (old_id, command, cols, rows) = {
            let guard = session.lock().unwrap();
            let (c, r) = guard.size();
            (
                SessionId::from(guard.id.as_str()),
                guard.command.clone(),
                c,
                r,
            )
        };

        self.repository.kill(old_id.as_str())?;

        let (new_session_id, pid) =
            self.repository
                .spawn(&command, &[], None, None, None, cols, rows)?;

        Ok(RestartOutput {
            old_session_id: old_id,
            new_session_id,
            command,
            pid,
        })
    }
}

/// Use case for attaching to a session.
pub trait AttachUseCase: Send + Sync {
    fn execute(&self, session_id: &str) -> Result<AttachOutput, SessionError>;
}

/// Output from attaching to a session.
#[derive(Debug, Clone)]
pub struct AttachOutput {
    pub session_id: SessionId,
    pub success: bool,
    pub message: String,
}

/// Implementation of the attach use case.
pub struct AttachUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> AttachUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> AttachUseCase for AttachUseCaseImpl<R> {
    fn execute(&self, session_id: &str) -> Result<AttachOutput, SessionError> {
        let session = self.repository.resolve(Some(session_id))?;

        let is_running = {
            let mut guard = session.lock().unwrap();
            guard.is_running()
        };

        if !is_running {
            return Err(SessionError::NotFound(format!(
                "{} (session not running)",
                session_id
            )));
        }

        self.repository.set_active(session_id)?;

        Ok(AttachOutput {
            session_id: SessionId::from(session_id),
            success: true,
            message: format!("Now attached to session {}", session_id),
        })
    }
}

/// Use case for resizing a session.
pub trait ResizeUseCase: Send + Sync {
    fn execute(&self, input: ResizeInput) -> Result<ResizeOutput, SessionError>;
}

/// Implementation of the resize use case.
pub struct ResizeUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> ResizeUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> ResizeUseCase for ResizeUseCaseImpl<R> {
    fn execute(&self, input: ResizeInput) -> Result<ResizeOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut guard = session.lock().unwrap();

        guard.resize(input.cols, input.rows)?;

        Ok(ResizeOutput {
            session_id: SessionId::from(guard.id.as_str()),
            success: true,
        })
    }
}
