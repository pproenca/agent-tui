//! Diagnostics use case.

use crate::domain::TerminalWriteInput;
use crate::domain::TerminalWriteOutput;
use crate::usecases::ports::SessionError;
use crate::usecases::ports::SessionRepository;

pub fn terminal_write<R: SessionRepository + ?Sized>(
    repository: &R,
    input: TerminalWriteInput,
) -> Result<TerminalWriteOutput, SessionError> {
    let session = repository.resolve(input.session_id.as_ref())?;
    let bytes_len = input.data.len();
    session.terminal_write(&input.data)?;
    Ok(TerminalWriteOutput {
        session_id: session.session_id(),
        bytes_written: bytes_len,
    })
}

#[cfg(test)]
#[path = "diagnostics_tests.rs"]
mod tests;
