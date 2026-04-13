//! Input use case.

use std::sync::Arc;

use crate::domain::KeydownInput;
use crate::domain::KeydownOutput;
use crate::domain::KeystrokeInput;
use crate::domain::KeystrokeOutput;
use crate::domain::KeyupInput;
use crate::domain::KeyupOutput;
use crate::domain::TypeInput;
use crate::domain::TypeOutput;
use crate::usecases::ports::SessionError;
use crate::usecases::ports::SessionRepository;

pub trait KeystrokeUseCase: Send + Sync {
    fn execute(&self, input: KeystrokeInput) -> Result<KeystrokeOutput, SessionError>;
}

pub struct KeystrokeUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> KeystrokeUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> KeystrokeUseCase for KeystrokeUseCaseImpl<R> {
    fn execute(&self, input: KeystrokeInput) -> Result<KeystrokeOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_ref())?;
        session.keystroke(&input.key)?;

        Ok(KeystrokeOutput { success: true })
    }
}

pub trait TypeUseCase: Send + Sync {
    fn execute(&self, input: TypeInput) -> Result<TypeOutput, SessionError>;
}

pub struct TypeUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> TypeUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> TypeUseCase for TypeUseCaseImpl<R> {
    fn execute(&self, input: TypeInput) -> Result<TypeOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_ref())?;
        session.type_text(&input.text)?;

        Ok(TypeOutput { success: true })
    }
}

pub trait KeydownUseCase: Send + Sync {
    fn execute(&self, input: KeydownInput) -> Result<KeydownOutput, SessionError>;
}

pub struct KeydownUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> KeydownUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> KeydownUseCase for KeydownUseCaseImpl<R> {
    fn execute(&self, input: KeydownInput) -> Result<KeydownOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_ref())?;
        session.keydown(&input.key)?;

        Ok(KeydownOutput { success: true })
    }
}

pub trait KeyupUseCase: Send + Sync {
    fn execute(&self, input: KeyupInput) -> Result<KeyupOutput, SessionError>;
}

pub struct KeyupUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> KeyupUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> KeyupUseCase for KeyupUseCaseImpl<R> {
    fn execute(&self, input: KeyupInput) -> Result<KeyupOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_ref())?;
        session.keyup(&input.key)?;

        Ok(KeyupOutput { success: true })
    }
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
