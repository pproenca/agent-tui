use std::sync::Arc;


use crate::domain::{
    RecordStartInput, RecordStartOutput, RecordStatusInput, RecordStatusOutput, RecordStopInput,
    RecordStopOutput,
};
use crate::usecases::ports::SessionError;
use crate::usecases::ports::SessionRepository;

pub trait RecordStartUseCase: Send + Sync {
    fn execute(&self, input: RecordStartInput) -> Result<RecordStartOutput, SessionError>;
}

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
        session.start_recording();

        Ok(RecordStartOutput {
            session_id: session.session_id(),
            success: true,
        })
    }
}

pub trait RecordStopUseCase: Send + Sync {
    fn execute(&self, input: RecordStopInput) -> Result<RecordStopOutput, SessionError>;
}

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

        let frames = session.stop_recording();
        let (cols, rows) = session.size();
        let format = input.format.unwrap_or_else(|| "asciicast".to_string());

        Ok(RecordStopOutput {
            session_id: session.session_id(),
            frame_count: frames.len(),
            frames,
            format,
            cols,
            rows,
        })
    }
}

pub trait RecordStatusUseCase: Send + Sync {
    fn execute(&self, input: RecordStatusInput) -> Result<RecordStatusOutput, SessionError>;
}

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
        Ok(session.recording_status())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SessionId;
    use crate::infra::daemon::test_support::{MockError, MockSessionRepository};

    #[test]
    fn test_record_start_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = RecordStartUseCaseImpl::new(repo);

        let input = RecordStartInput { session_id: None };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_record_start_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = RecordStartUseCaseImpl::new(repo);

        let input = RecordStartInput {
            session_id: Some(SessionId::new("missing")),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_record_stop_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = RecordStopUseCaseImpl::new(repo);

        let input = RecordStopInput {
            session_id: None,
            format: None,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_record_stop_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = RecordStopUseCaseImpl::new(repo);

        let input = RecordStopInput {
            session_id: Some(SessionId::new("missing")),
            format: Some("json".to_string()),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_record_status_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = RecordStatusUseCaseImpl::new(repo);

        let input = RecordStatusInput { session_id: None };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_record_status_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = RecordStatusUseCaseImpl::new(repo);

        let input = RecordStatusInput {
            session_id: Some(SessionId::new("missing")),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }
}
