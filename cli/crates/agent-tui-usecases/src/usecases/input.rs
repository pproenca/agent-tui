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
use crate::domain::MouseClickInput;
use crate::domain::MouseClickOutput;
use crate::domain::MouseMoveInput;
use crate::domain::MouseMoveOutput;
use crate::domain::MouseDownInput;
use crate::domain::MouseDownOutput;
use crate::domain::MouseUpInput;
use crate::domain::MouseUpOutput;
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

// ============================================================
// Mouse use cases
// ============================================================

pub trait MouseClickUseCase: Send + Sync {
    fn execute(&self, input: MouseClickInput) -> Result<MouseClickOutput, SessionError>;
}

pub struct MouseClickUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> MouseClickUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> MouseClickUseCase for MouseClickUseCaseImpl<R> {
    fn execute(&self, input: MouseClickInput) -> Result<MouseClickOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_ref())?;
        session.mouse_click(input.col, input.row, input.button.as_str())?;

        Ok(MouseClickOutput { success: true })
    }
}

pub trait MouseMoveUseCase: Send + Sync {
    fn execute(&self, input: MouseMoveInput) -> Result<MouseMoveOutput, SessionError>;
}

pub struct MouseMoveUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> MouseMoveUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> MouseMoveUseCase for MouseMoveUseCaseImpl<R> {
    fn execute(&self, input: MouseMoveInput) -> Result<MouseMoveOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_ref())?;
        session.mouse_move(input.col, input.row)?;

        Ok(MouseMoveOutput { success: true })
    }
}

pub trait MouseDownUseCase: Send + Sync {
    fn execute(&self, input: MouseDownInput) -> Result<MouseDownOutput, SessionError>;
}

pub struct MouseDownUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> MouseDownUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> MouseDownUseCase for MouseDownUseCaseImpl<R> {
    fn execute(&self, input: MouseDownInput) -> Result<MouseDownOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_ref())?;
        session.mouse_down(input.col, input.row, input.button.as_str())?;

        Ok(MouseDownOutput { success: true })
    }
}

pub trait MouseUpUseCase: Send + Sync {
    fn execute(&self, input: MouseUpInput) -> Result<MouseUpOutput, SessionError>;
}

pub struct MouseUpUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> MouseUpUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> MouseUpUseCase for MouseUpUseCaseImpl<R> {
    fn execute(&self, input: MouseUpInput) -> Result<MouseUpOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_ref())?;
        session.mouse_up(input.col, input.row, input.button.as_str())?;

        Ok(MouseUpOutput { success: true })
    }
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
