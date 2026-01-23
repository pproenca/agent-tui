use std::sync::Arc;

use agent_tui_common::mutex_lock_or_recover;

use crate::domain::{SnapshotInput, SnapshotOutput};
use crate::error::SessionError;
use crate::repository::SessionRepository;
use crate::session::SessionId;

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
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.update()?;

        let screen = session_guard.screen_text();
        let session_id = SessionId::from(session_guard.id.as_str());

        let elements = if input.include_elements {
            Some(session_guard.detect_elements().to_vec())
        } else {
            None
        };

        let cursor = if input.include_cursor {
            Some(session_guard.cursor())
        } else {
            None
        };

        Ok(SnapshotOutput {
            session_id,
            screen,
            elements,
            cursor,
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
