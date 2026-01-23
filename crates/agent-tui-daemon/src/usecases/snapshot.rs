use std::sync::Arc;

use agent_tui_common::mutex_lock_or_recover;

use crate::domain::{SnapshotInput, SnapshotOutput};
use crate::error::SessionError;
use crate::repository::SessionRepository;
use crate::session::SessionId;

/// Use case for taking a snapshot of a session.
pub trait SnapshotUseCase: Send + Sync {
    fn execute(&self, input: SnapshotInput) -> Result<SnapshotOutput, SessionError>;
}

/// Implementation of the snapshot use case.
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
    use crate::session::{Session, SessionInfo};
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct MockSessionRepository;

    impl SessionRepository for MockSessionRepository {
        fn spawn(
            &self,
            _command: &str,
            _args: &[String],
            _cwd: Option<&str>,
            _env: Option<&HashMap<String, String>>,
            _session_id: Option<String>,
            _cols: u16,
            _rows: u16,
        ) -> Result<(SessionId, u32), SessionError> {
            unimplemented!()
        }

        fn get(&self, _session_id: &str) -> Result<Arc<Mutex<Session>>, SessionError> {
            Err(SessionError::NotFound("test".to_string()))
        }

        fn active(&self) -> Result<Arc<Mutex<Session>>, SessionError> {
            Err(SessionError::NoActiveSession)
        }

        fn resolve(&self, _session_id: Option<&str>) -> Result<Arc<Mutex<Session>>, SessionError> {
            Err(SessionError::NoActiveSession)
        }

        fn set_active(&self, _session_id: &str) -> Result<(), SessionError> {
            unimplemented!()
        }

        fn list(&self) -> Vec<SessionInfo> {
            vec![]
        }

        fn kill(&self, _session_id: &str) -> Result<(), SessionError> {
            unimplemented!()
        }

        fn session_count(&self) -> usize {
            0
        }

        fn active_session_id(&self) -> Option<SessionId> {
            None
        }
    }

    #[test]
    fn test_snapshot_usecase_returns_error_when_no_session() {
        let repository = Arc::new(MockSessionRepository);
        let usecase = SnapshotUseCaseImpl::new(repository);

        let input = SnapshotInput::default();
        let result = usecase.execute(input);

        assert!(result.is_err());
    }
}
