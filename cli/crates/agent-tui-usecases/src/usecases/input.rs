//! Input use case.

use crate::domain::KeydownInput;
use crate::domain::KeystrokeInput;
use crate::domain::KeyupInput;
use crate::domain::TypeInput;
use crate::usecases::ports::SessionError;
use crate::usecases::ports::SessionRepository;

pub fn keystroke<R: SessionRepository + ?Sized>(
    repository: &R,
    input: KeystrokeInput,
) -> Result<(), SessionError> {
    let session = repository.resolve(input.session_id.as_ref())?;
    session.keystroke(&input.key)?;

    Ok(())
}

pub fn type_text<R: SessionRepository + ?Sized>(
    repository: &R,
    input: TypeInput,
) -> Result<(), SessionError> {
    let session = repository.resolve(input.session_id.as_ref())?;
    session.type_text(&input.text)?;

    Ok(())
}

pub fn keydown<R: SessionRepository + ?Sized>(
    repository: &R,
    input: KeydownInput,
) -> Result<(), SessionError> {
    let session = repository.resolve(input.session_id.as_ref())?;
    session.keydown(&input.key)?;

    Ok(())
}

pub fn keyup<R: SessionRepository + ?Sized>(
    repository: &R,
    input: KeyupInput,
) -> Result<(), SessionError> {
    let session = repository.resolve(input.session_id.as_ref())?;
    session.keyup(&input.key)?;

    Ok(())
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
