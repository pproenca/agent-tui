use std::sync::Arc;

use crate::domain::core::vom::snapshot::{SnapshotOptions, format_snapshot};

use crate::adapters::{core_cursor_to_domain, core_elements_to_domain, core_snapshot_to_domain};
use crate::domain::{
    AccessibilitySnapshotInput, AccessibilitySnapshotOutput, SnapshotInput, SnapshotOutput,
};
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
    #[tracing::instrument(
        skip(self, input),
        fields(
            session = ?input.session_id,
            include_elements = input.include_elements,
            include_cursor = input.include_cursor,
            include_render = input.include_render,
            strip_ansi = input.strip_ansi,
            region = ?input.region
        )
    )]
    fn execute(&self, input: SnapshotInput) -> Result<SnapshotOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;

        let screenshot = session.screen_text();
        let session_id = session.session_id();

        let elements = if input.include_elements {
            Some(core_elements_to_domain(&session.detect_elements()))
        } else {
            None
        };

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
            elements,
            cursor,
            rendered,
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
    #[tracing::instrument(
        skip(self, input),
        fields(session = ?input.session_id, interactive_only = input.interactive_only)
    )]
    fn execute(
        &self,
        input: AccessibilitySnapshotInput,
    ) -> Result<AccessibilitySnapshotOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        session.update()?;

        let session_id = session.session_id();
        let components = session.analyze_screen();

        let options = SnapshotOptions {
            interactive_only: input.interactive_only,
            ..Default::default()
        };
        let core_snapshot = format_snapshot(&components, &options);
        let snapshot = core_snapshot_to_domain(&core_snapshot);

        Ok(AccessibilitySnapshotOutput {
            session_id,
            snapshot,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SessionId;
    use crate::infra::daemon::test_support::MockSessionRepository;

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
            session_id: Some(SessionId::new("test-session")),
            interactive_only: true,
        };

        assert_eq!(input.session_id, Some(SessionId::new("test-session")));
        assert!(input.interactive_only);
    }
}
