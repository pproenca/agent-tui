use std::sync::Arc;

use agent_tui_common::mutex_lock_or_recover;
use agent_tui_core::vom::snapshot::{SnapshotOptions, format_snapshot};

use crate::domain::{
    AccessibilitySnapshotInput, AccessibilitySnapshotOutput, SnapshotInput, SnapshotOutput,
};
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

pub trait AccessibilitySnapshotUseCase: Send + Sync {
    fn execute(
        &self,
        input: AccessibilitySnapshotInput,
    ) -> Result<AccessibilitySnapshotOutput, SessionError>;
}

pub struct AccessibilitySnapshotUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> AccessibilitySnapshotUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> AccessibilitySnapshotUseCase for AccessibilitySnapshotUseCaseImpl<R> {
    fn execute(
        &self,
        input: AccessibilitySnapshotInput,
    ) -> Result<AccessibilitySnapshotOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut session_guard = mutex_lock_or_recover(&session);

        session_guard.update()?;

        let session_id = SessionId::from(session_guard.id.as_str());
        let components = session_guard.analyze_screen();

        let options = SnapshotOptions {
            interactive: input.interactive_only,
        };
        let snapshot = format_snapshot(&components, &options);

        Ok(AccessibilitySnapshotOutput {
            session_id,
            snapshot,
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

    #[test]
    fn test_enhanced_snapshot_usecase_returns_error_when_no_session() {
        let repository = Arc::new(MockSessionRepository::new());
        let usecase = AccessibilitySnapshotUseCaseImpl::new(repository);

        let input = AccessibilitySnapshotInput::default();
        let result = usecase.execute(input);

        assert!(result.is_err());
    }

    #[test]
    fn test_enhanced_snapshot_input_default() {
        let input = AccessibilitySnapshotInput::default();

        assert!(input.session_id.is_none());
        assert!(!input.interactive_only);
    }

    #[test]
    fn test_enhanced_snapshot_input_with_options() {
        let input = AccessibilitySnapshotInput {
            session_id: Some("test-session".to_string()),
            interactive_only: true,
        };

        assert_eq!(input.session_id, Some("test-session".to_string()));
        assert!(input.interactive_only);
    }
}
