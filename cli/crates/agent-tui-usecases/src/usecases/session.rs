//! Session use cases.

use std::sync::Arc;

use crate::domain::AssertConditionType;
use crate::domain::AssertInput;
use crate::domain::AssertOutput;
use crate::domain::AttachInput;
use crate::domain::AttachOutput;
use crate::domain::CleanupFailure;
use crate::domain::CleanupInput;
use crate::domain::CleanupOutput;
use crate::domain::KillOutput;
use crate::domain::ResizeInput;
use crate::domain::ResizeOutput;
use crate::domain::RestartOutput;
use crate::domain::SessionInput;
use crate::domain::SessionsOutput;
use crate::domain::SpawnInput;
use crate::domain::SpawnOutput;
use crate::usecases::SpawnError;
use crate::usecases::ports::SessionError;
use crate::usecases::ports::SessionRepository;
use crate::usecases::ports::SpawnErrorKind;
use crate::usecases::ports::TerminalError;

pub trait SpawnUseCase: Send + Sync {
    fn execute(&self, input: SpawnInput) -> Result<SpawnOutput, SpawnError>;
}

pub struct SpawnUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> SpawnUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> SpawnUseCase for SpawnUseCaseImpl<R> {
    fn execute(&self, input: SpawnInput) -> Result<SpawnOutput, SpawnError> {
        let SpawnInput {
            command,
            args,
            cwd,
            env,
            session_id,
            size,
        } = input;

        match self.repository.spawn(
            &command,
            &args,
            cwd.as_deref(),
            env.as_ref(),
            session_id,
            size,
        ) {
            Ok((session_id, pid)) => Ok(SpawnOutput { session_id, pid }),
            Err(SessionError::LimitReached(max)) => Err(SpawnError::SessionLimitReached { max }),
            Err(SessionError::AlreadyExists(session_id)) => {
                Err(SpawnError::SessionAlreadyExists { session_id })
            }
            Err(SessionError::Terminal(TerminalError::Spawn { kind, reason })) => match kind {
                SpawnErrorKind::NotFound => Err(SpawnError::CommandNotFound { command }),
                SpawnErrorKind::PermissionDenied => Err(SpawnError::PermissionDenied { command }),
                SpawnErrorKind::Other => Err(SpawnError::TerminalError {
                    operation: "spawn".to_string(),
                    reason,
                }),
            },
            Err(SessionError::Terminal(term_err)) => Err(SpawnError::TerminalError {
                operation: term_err.operation().to_string(),
                reason: term_err.reason().to_string(),
            }),
            Err(e) => Err(SpawnError::TerminalError {
                operation: "spawn".to_string(),
                reason: e.to_string(),
            }),
        }
    }
}

pub trait SessionsUseCase: Send + Sync {
    fn execute(&self) -> SessionsOutput;
}

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

pub trait KillUseCase: Send + Sync {
    fn execute(&self, input: SessionInput) -> Result<KillOutput, SessionError>;
}

pub struct KillUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> KillUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> KillUseCase for KillUseCaseImpl<R> {
    fn execute(&self, input: SessionInput) -> Result<KillOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_ref())?;
        let session_id = session.session_id();

        self.repository.kill(&session_id)?;

        Ok(KillOutput {
            session_id,
            success: true,
        })
    }
}

pub trait RestartUseCase: Send + Sync {
    fn execute(&self, input: SessionInput) -> Result<RestartOutput, SessionError>;
}

pub struct RestartUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> RestartUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> RestartUseCase for RestartUseCaseImpl<R> {
    fn execute(&self, input: SessionInput) -> Result<RestartOutput, SessionError> {
        self.repository.restart(input.session_id.as_ref())
    }
}

pub trait AttachUseCase: Send + Sync {
    fn execute(&self, input: AttachInput) -> Result<AttachOutput, SessionError>;
}

pub struct AttachUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> AttachUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> AttachUseCase for AttachUseCaseImpl<R> {
    fn execute(&self, input: AttachInput) -> Result<AttachOutput, SessionError> {
        let session = self.repository.resolve(Some(&input.session_id))?;

        if !session.is_running() {
            return Err(SessionError::NotRunning {
                session_id: input.session_id.to_string(),
            });
        }

        self.repository.set_active(&input.session_id)?;

        Ok(AttachOutput {
            session_id: input.session_id,
            success: true,
        })
    }
}

pub trait ResizeUseCase: Send + Sync {
    fn execute(&self, input: ResizeInput) -> Result<ResizeOutput, SessionError>;
}

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
        let session = self.repository.resolve(input.session_id.as_ref())?;
        session.resize(input.size)?;

        Ok(ResizeOutput {
            session_id: session.session_id(),
            success: true,
            size: input.size,
        })
    }
}

pub trait CleanupUseCase: Send + Sync {
    fn execute(&self, input: CleanupInput) -> CleanupOutput;
}

pub struct CleanupUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> CleanupUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> CleanupUseCase for CleanupUseCaseImpl<R> {
    fn execute(&self, input: CleanupInput) -> CleanupOutput {
        let sessions = self.repository.list();
        let mut cleaned = 0;
        let mut failures = Vec::new();

        for session in sessions {
            let should_cleanup = input.all || !session.running;
            if !should_cleanup {
                continue;
            }

            match self.repository.kill(&session.id) {
                Ok(()) => cleaned += 1,
                Err(err) => failures.push(CleanupFailure {
                    session_id: session.id,
                    error: err.to_string(),
                }),
            }
        }

        CleanupOutput { cleaned, failures }
    }
}

pub trait AssertUseCase: Send + Sync {
    fn execute(&self, input: AssertInput) -> Result<AssertOutput, SessionError>;
}

pub struct AssertUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> AssertUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> AssertUseCase for AssertUseCaseImpl<R> {
    fn execute(&self, input: AssertInput) -> Result<AssertOutput, SessionError> {
        let condition = format!("{}:{}", input.condition_type.as_str(), input.value);

        let passed = match input.condition_type {
            AssertConditionType::Text => {
                let session = self.repository.resolve(input.session_id.as_ref())?;
                session.update()?;
                let screen = session.screen_text();
                screen.contains(&input.value)
            }
            AssertConditionType::Session => {
                let sessions = self.repository.list();
                sessions
                    .iter()
                    .any(|s| s.id.as_str() == input.value && s.is_active())
            }
            other => {
                return Err(SessionError::InvalidKey(format!(
                    "Unsupported assert condition: {}",
                    other.as_str()
                )));
            }
        };

        Ok(AssertOutput { passed, condition })
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
