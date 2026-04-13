//! Diagnostics use case.

use std::sync::Arc;

use crate::domain::TerminalWriteInput;
use crate::domain::TerminalWriteOutput;
use crate::usecases::ports::SessionError;
use crate::usecases::ports::SessionRepository;

pub trait TerminalWriteUseCase: Send + Sync {
    fn execute(&self, input: TerminalWriteInput) -> Result<TerminalWriteOutput, SessionError>;
}

pub struct TerminalWriteUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> TerminalWriteUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> TerminalWriteUseCase for TerminalWriteUseCaseImpl<R> {
    fn execute(&self, input: TerminalWriteInput) -> Result<TerminalWriteOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_ref())?;
        let bytes_len = input.data.len();
        session.terminal_write(&input.data)?;
        Ok(TerminalWriteOutput {
            session_id: session.session_id(),
            bytes_written: bytes_len,
            success: true,
        })
    }
}

#[cfg(test)]
#[path = "diagnostics_tests.rs"]
mod tests;
