use std::sync::Arc;

use agent_tui_common::mutex_lock_or_recover;
use agent_tui_terminal::PtyError;

use crate::domain::{
    ConsoleInput, ConsoleOutput, ErrorsInput, ErrorsOutput, PtyReadInput, PtyReadOutput,
    PtyWriteInput, PtyWriteOutput, TraceInput, TraceOutput,
};
use crate::error::SessionError;
use crate::repository::SessionRepository;

/// Use case for trace operations.
pub trait TraceUseCase: Send + Sync {
    fn execute(&self, input: TraceInput) -> Result<TraceOutput, SessionError>;
}

/// Implementation of the trace use case.
pub struct TraceUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> TraceUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> TraceUseCase for TraceUseCaseImpl<R> {
    fn execute(&self, input: TraceInput) -> Result<TraceOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let session_guard = mutex_lock_or_recover(&session);

        let count = if input.count == 0 { 1000 } else { input.count };
        let entries = session_guard.get_trace_entries(count);

        Ok(TraceOutput {
            tracing: true,
            entries,
        })
    }
}

/// Use case for console operations.
pub trait ConsoleUseCase: Send + Sync {
    fn execute(&self, input: ConsoleInput) -> Result<ConsoleOutput, SessionError>;
}

/// Implementation of the console use case.
pub struct ConsoleUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> ConsoleUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> ConsoleUseCase for ConsoleUseCaseImpl<R> {
    fn execute(&self, input: ConsoleInput) -> Result<ConsoleOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        if let Err(e) = session_guard.update() {
            eprintln!("Warning: Session update failed during console: {}", e);
        }

        let screen_text = session_guard.screen_text();
        let lines: Vec<String> = screen_text.lines().map(String::from).collect();

        Ok(ConsoleOutput { lines })
    }
}

/// Use case for errors operations.
pub trait ErrorsUseCase: Send + Sync {
    fn execute(&self, input: ErrorsInput) -> Result<ErrorsOutput, SessionError>;
}

/// Implementation of the errors use case.
pub struct ErrorsUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> ErrorsUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> ErrorsUseCase for ErrorsUseCaseImpl<R> {
    fn execute(&self, input: ErrorsInput) -> Result<ErrorsOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let session_guard = mutex_lock_or_recover(&session);

        let count = if input.count == 0 { 1000 } else { input.count };
        let errors = session_guard.get_errors(count);

        Ok(ErrorsOutput {
            total_count: errors.len(),
            errors,
        })
    }
}

/// Use case for PTY read operations.
pub trait PtyReadUseCase: Send + Sync {
    fn execute(&self, input: PtyReadInput) -> Result<PtyReadOutput, SessionError>;
}

/// Implementation of the PTY read use case.
pub struct PtyReadUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> PtyReadUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> PtyReadUseCase for PtyReadUseCaseImpl<R> {
    fn execute(&self, input: PtyReadInput) -> Result<PtyReadOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let session_guard = mutex_lock_or_recover(&session);

        let max_bytes = if input.max_bytes == 0 {
            4096
        } else {
            input.max_bytes
        };
        let mut buf = vec![0u8; max_bytes];

        match session_guard.pty_try_read(&mut buf, 100) {
            Ok(bytes_read) => {
                buf.truncate(bytes_read);
                let data = String::from_utf8_lossy(&buf).to_string();
                Ok(PtyReadOutput {
                    session_id: session_guard.id.clone(),
                    data,
                    bytes_read,
                })
            }
            Err(e) => Err(SessionError::Pty(PtyError::Read(e.to_string()))),
        }
    }
}

/// Use case for PTY write operations.
pub trait PtyWriteUseCase: Send + Sync {
    fn execute(&self, input: PtyWriteInput) -> Result<PtyWriteOutput, SessionError>;
}

/// Implementation of the PTY write use case.
pub struct PtyWriteUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> PtyWriteUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> PtyWriteUseCase for PtyWriteUseCaseImpl<R> {
    fn execute(&self, input: PtyWriteInput) -> Result<PtyWriteOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let session_guard = mutex_lock_or_recover(&session);

        match session_guard.pty_write(input.data.as_bytes()) {
            Ok(()) => Ok(PtyWriteOutput {
                session_id: session_guard.id.clone(),
                bytes_written: input.data.len(),
                success: true,
            }),
            Err(e) => Err(SessionError::Pty(PtyError::Write(e.to_string()))),
        }
    }
}
