use std::sync::Arc;

use agent_tui_common::mutex_lock_or_recover;

use crate::domain::{
    KeydownInput, KeydownOutput, KeystrokeInput, KeystrokeOutput, KeyupInput, KeyupOutput,
    TypeInput, TypeOutput,
};
use crate::error::SessionError;
use crate::repository::SessionRepository;

/// Use case for sending a keystroke.
pub trait KeystrokeUseCase: Send + Sync {
    fn execute(&self, input: KeystrokeInput) -> Result<KeystrokeOutput, SessionError>;
}

/// Implementation of the keystroke use case.
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
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let session_guard = mutex_lock_or_recover(&session);

        session_guard.keystroke(&input.key)?;

        Ok(KeystrokeOutput { success: true })
    }
}

/// Use case for typing text.
pub trait TypeUseCase: Send + Sync {
    fn execute(&self, input: TypeInput) -> Result<TypeOutput, SessionError>;
}

/// Implementation of the type use case.
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
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let session_guard = mutex_lock_or_recover(&session);

        session_guard.type_text(&input.text)?;

        Ok(TypeOutput { success: true })
    }
}

/// Use case for sending a key down event.
pub trait KeydownUseCase: Send + Sync {
    fn execute(&self, input: KeydownInput) -> Result<KeydownOutput, SessionError>;
}

/// Implementation of the keydown use case.
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
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.keydown(&input.key)?;

        Ok(KeydownOutput { success: true })
    }
}

/// Use case for sending a key up event.
pub trait KeyupUseCase: Send + Sync {
    fn execute(&self, input: KeyupInput) -> Result<KeyupOutput, SessionError>;
}

/// Implementation of the keyup use case.
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
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.keyup(&input.key)?;

        Ok(KeyupOutput { success: true })
    }
}
