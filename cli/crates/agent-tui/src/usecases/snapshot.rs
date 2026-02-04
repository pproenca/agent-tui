//! Snapshot use case.

use std::sync::Arc;

use crate::domain::SnapshotInput;
use crate::domain::SnapshotOutput;
use crate::domain::core_cursor_to_domain;
use crate::usecases::ports::SessionError;
use crate::usecases::ports::SessionRepository;

pub trait SnapshotUseCase: Send + Sync {
    fn execute(&self, input: SnapshotInput) -> Result<SnapshotOutput, SessionError>;
}

pub struct SnapshotUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> SnapshotUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> SnapshotUseCase for SnapshotUseCaseImpl<R> {
    fn execute(&self, input: SnapshotInput) -> Result<SnapshotOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;

        let screenshot = session.screen_text();
        let session_id = session.session_id();

        let cursor = if input.include_cursor {
            Some(core_cursor_to_domain(&session.cursor()))
        } else {
            None
        };

        let rendered = if input.include_render {
            Some(session.screen_render())
        } else {
            None
        };

        Ok(SnapshotOutput {
            session_id,
            screenshot,
            cursor,
            rendered,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::MockSessionRepository;

    #[test]
    fn test_snapshot_usecase_returns_error_when_no_session() {
        let repository = Arc::new(MockSessionRepository::new());
        let usecase = SnapshotUseCaseImpl::new(repository);

        let input = SnapshotInput::default();
        let result = usecase.execute(input);

        assert!(result.is_err());
    }
}
