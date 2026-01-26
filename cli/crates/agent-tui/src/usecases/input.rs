use std::sync::Arc;

use crate::domain::{
    KeydownInput, KeydownOutput, KeystrokeInput, KeystrokeOutput, KeyupInput, KeyupOutput,
    TypeInput, TypeOutput,
};
use crate::usecases::ports::{SessionError, SessionRepository};

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
    #[tracing::instrument(
        skip(self, input),
        fields(session = ?input.session_id, key = %input.key)
    )]
    fn execute(&self, input: KeystrokeInput) -> Result<KeystrokeOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
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
    #[tracing::instrument(
        skip(self, input),
        fields(session = ?input.session_id, text_len = input.text.len())
    )]
    fn execute(&self, input: TypeInput) -> Result<TypeOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
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
    #[tracing::instrument(
        skip(self, input),
        fields(session = ?input.session_id, key = %input.key)
    )]
    fn execute(&self, input: KeydownInput) -> Result<KeydownOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
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
    #[tracing::instrument(
        skip(self, input),
        fields(session = ?input.session_id, key = %input.key)
    )]
    fn execute(&self, input: KeyupInput) -> Result<KeyupOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        session.keyup(&input.key)?;

        Ok(KeyupOutput { success: true })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SessionId;
    use crate::infra::daemon::test_support::{MockError, MockSessionRepository};

    #[test]
    fn test_keystroke_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = KeystrokeUseCaseImpl::new(repo);

        let input = KeystrokeInput {
            session_id: None,
            key: "Enter".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_keystroke_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = KeystrokeUseCaseImpl::new(repo);

        let input = KeystrokeInput {
            session_id: Some(SessionId::new("missing")),
            key: "Tab".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_type_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = TypeUseCaseImpl::new(repo);

        let input = TypeInput {
            session_id: None,
            text: "hello world".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_type_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = TypeUseCaseImpl::new(repo);

        let input = TypeInput {
            session_id: Some(SessionId::new("missing")),
            text: "test text".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_keydown_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = KeydownUseCaseImpl::new(repo);

        let input = KeydownInput {
            session_id: None,
            key: "Ctrl".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_keydown_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = KeydownUseCaseImpl::new(repo);

        let input = KeydownInput {
            session_id: Some(SessionId::new("missing")),
            key: "Shift".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_keyup_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = KeyupUseCaseImpl::new(repo);

        let input = KeyupInput {
            session_id: None,
            key: "Ctrl".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_keyup_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = KeyupUseCaseImpl::new(repo);

        let input = KeyupInput {
            session_id: Some(SessionId::new("missing")),
            key: "Alt".to_string(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }
}
