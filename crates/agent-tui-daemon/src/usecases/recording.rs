use std::sync::Arc;

use agent_tui_common::mutex_lock_or_recover;

use crate::domain::{
    RecordStartInput, RecordStartOutput, RecordStatusInput, RecordStatusOutput, RecordStopInput,
    RecordStopOutput,
};
use crate::error::SessionError;
use crate::repository::SessionRepository;

/// Use case for starting recording.
pub trait RecordStartUseCase: Send + Sync {
    fn execute(&self, input: RecordStartInput) -> Result<RecordStartOutput, SessionError>;
}

/// Implementation of the record start use case.
pub struct RecordStartUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> RecordStartUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> RecordStartUseCase for RecordStartUseCaseImpl<R> {
    fn execute(&self, input: RecordStartInput) -> Result<RecordStartOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.start_recording();

        Ok(RecordStartOutput {
            session_id: session_guard.id.clone(),
            success: true,
        })
    }
}

/// Use case for stopping recording.
pub trait RecordStopUseCase: Send + Sync {
    fn execute(&self, input: RecordStopInput) -> Result<RecordStopOutput, SessionError>;
}

/// Implementation of the record stop use case.
pub struct RecordStopUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> RecordStopUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> RecordStopUseCase for RecordStopUseCaseImpl<R> {
    fn execute(&self, input: RecordStopInput) -> Result<RecordStopOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        let frames = session_guard.stop_recording();
        let (cols, rows) = session_guard.size();
        let format = input.format.unwrap_or_else(|| "asciicast".to_string());

        Ok(RecordStopOutput {
            session_id: session_guard.id.clone(),
            frame_count: frames.len(),
            frames,
            format,
            cols,
            rows,
        })
    }
}

/// Use case for checking recording status.
pub trait RecordStatusUseCase: Send + Sync {
    fn execute(&self, input: RecordStatusInput) -> Result<RecordStatusOutput, SessionError>;
}

/// Implementation of the record status use case.
pub struct RecordStatusUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> RecordStatusUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> RecordStatusUseCase for RecordStatusUseCaseImpl<R> {
    fn execute(&self, input: RecordStatusInput) -> Result<RecordStatusOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let session_guard = mutex_lock_or_recover(&session);

        Ok(session_guard.recording_status())
    }
}
