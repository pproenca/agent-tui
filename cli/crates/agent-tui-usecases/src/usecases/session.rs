//! Session use cases.

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

pub fn spawn<R: SessionRepository + ?Sized>(
    repository: &R,
    input: SpawnInput,
) -> Result<SpawnOutput, SpawnError> {
    let SpawnInput {
        command,
        args,
        cwd,
        env,
        session_id,
        size,
    } = input;

    match repository.spawn(
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

pub fn sessions<R: SessionRepository + ?Sized>(repository: &R) -> SessionsOutput {
    let sessions = repository.list();
    let active_session = repository.active_session_id();

    SessionsOutput {
        sessions,
        active_session,
    }
}

pub fn kill<R: SessionRepository + ?Sized>(
    repository: &R,
    input: SessionInput,
) -> Result<KillOutput, SessionError> {
    let session = repository.resolve(input.session_id.as_ref())?;
    let session_id = session.session_id();

    repository.kill(&session_id)?;

    Ok(KillOutput { session_id })
}

pub fn restart<R: SessionRepository + ?Sized>(
    repository: &R,
    input: SessionInput,
) -> Result<RestartOutput, SessionError> {
    repository.restart(input.session_id.as_ref())
}

pub fn attach<R: SessionRepository + ?Sized>(
    repository: &R,
    input: AttachInput,
) -> Result<AttachOutput, SessionError> {
    let session = repository.resolve(Some(&input.session_id))?;

    if !session.is_running() {
        return Err(SessionError::NotRunning {
            session_id: input.session_id.to_string(),
        });
    }

    repository.set_active(&input.session_id)?;

    Ok(AttachOutput {
        session_id: input.session_id,
    })
}

pub fn resize<R: SessionRepository + ?Sized>(
    repository: &R,
    input: ResizeInput,
) -> Result<ResizeOutput, SessionError> {
    let session = repository.resolve(input.session_id.as_ref())?;
    session.resize(input.size)?;

    Ok(ResizeOutput {
        session_id: session.session_id(),
        size: input.size,
    })
}

pub fn cleanup<R: SessionRepository + ?Sized>(
    repository: &R,
    input: CleanupInput,
) -> CleanupOutput {
    let sessions = repository.list();
    let mut cleaned = 0;
    let mut failures = Vec::new();

    for session in sessions {
        let should_cleanup = input.all || !session.running;
        if !should_cleanup {
            continue;
        }

        match repository.kill(&session.id) {
            Ok(()) => cleaned += 1,
            Err(err) => failures.push(CleanupFailure {
                session_id: session.id,
                error: err.to_string(),
            }),
        }
    }

    CleanupOutput { cleaned, failures }
}

pub fn assert<R: SessionRepository + ?Sized>(
    repository: &R,
    input: AssertInput,
) -> Result<AssertOutput, SessionError> {
    let condition = format!("{}:{}", input.condition_type.as_str(), input.value);

    let passed = match input.condition_type {
        AssertConditionType::Text => {
            let session = repository.resolve(input.session_id.as_ref())?;
            session.update()?;
            let screen = session.screen_text();
            screen.contains(&input.value)
        }
        AssertConditionType::Session => {
            let sessions = repository.list();
            sessions
                .iter()
                .any(|s| s.id.as_str() == input.value && s.running)
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

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
