use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::common::mutex_lock_or_recover;

use crate::daemon::domain::{WaitInput, WaitOutput};
use crate::daemon::error::SessionError;
use crate::daemon::repository::SessionRepository;
use crate::daemon::sleeper::{RealSleeper, Sleeper};
use crate::daemon::wait::{StableTracker, WaitCondition, check_condition};

pub trait WaitUseCase: Send + Sync {
    fn execute(&self, input: WaitInput) -> Result<WaitOutput, SessionError>;
}

pub struct WaitUseCaseImpl<R: SessionRepository, S: Sleeper = RealSleeper> {
    repository: Arc<R>,
    sleeper: S,
}

impl<R: SessionRepository> WaitUseCaseImpl<R, RealSleeper> {
    /// Create a new WaitUseCaseImpl with the default RealSleeper.
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            repository,
            sleeper: RealSleeper,
        }
    }
}

impl<R: SessionRepository, S: Sleeper> WaitUseCaseImpl<R, S> {
    /// Create a new WaitUseCaseImpl with a custom sleeper.
    /// Use this for testing with MockSleeper.
    pub fn with_sleeper(repository: Arc<R>, sleeper: S) -> Self {
        Self {
            repository,
            sleeper,
        }
    }
}

impl<R: SessionRepository, S: Sleeper> WaitUseCase for WaitUseCaseImpl<R, S> {
    fn execute(&self, input: WaitInput) -> Result<WaitOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let timeout = Duration::from_millis(input.timeout_ms);
        let start = Instant::now();

        let condition = match (input.condition.as_deref(), input.text.as_ref()) {
            (Some("stable"), _) => WaitCondition::Stable,
            (Some("element"), Some(target)) => WaitCondition::Element(target.clone()),
            (Some("focused"), Some(target)) => WaitCondition::Focused(target.clone()),
            (Some("not_visible"), Some(target)) => WaitCondition::NotVisible(target.clone()),
            (Some("text_gone"), Some(target)) => WaitCondition::TextGone(target.clone()),
            (Some("value"), Some(target)) => {
                if let Some((element_ref, expected)) = target.split_once('=') {
                    WaitCondition::Value {
                        element: element_ref.to_string(),
                        expected: expected.to_string(),
                    }
                } else {
                    WaitCondition::Text(target.clone())
                }
            }
            (_, Some(text)) => WaitCondition::Text(text.clone()),
            (None, None) => WaitCondition::Stable,
            _ => WaitCondition::Stable,
        };

        let mut stable_tracker = StableTracker::new(3);
        let poll_interval = Duration::from_millis(50);

        loop {
            {
                let mut session_guard = mutex_lock_or_recover(&session);
                session_guard.update()?;

                if check_condition(&mut *session_guard, &condition, &mut stable_tracker) {
                    return Ok(WaitOutput {
                        found: true,
                        elapsed_ms: start.elapsed().as_millis() as u64,
                    });
                }
            }

            if start.elapsed() >= timeout {
                return Ok(WaitOutput {
                    found: false,
                    elapsed_ms: start.elapsed().as_millis() as u64,
                });
            }

            self.sleeper.sleep(poll_interval);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::domain::SessionId;
    use crate::daemon::sleeper::MockSleeper;
    use crate::daemon::test_support::{MockError, MockSessionRepository};

    // ========================================================================
    // WaitUseCase Tests (Error paths)
    // ========================================================================

    // ========================================================================
    // MockSleeper Integration Tests
    // ========================================================================

    #[test]
    fn test_wait_usecase_can_be_constructed_with_mock_sleeper() {
        // This test demonstrates that WaitUseCaseImpl can be constructed
        // with a MockSleeper, enabling deterministic tests without sleeping.
        let repo = Arc::new(MockSessionRepository::new());
        let mock_sleeper = MockSleeper::new();
        let _usecase = WaitUseCaseImpl::with_sleeper(repo, mock_sleeper);
        // Construction succeeds - the mock sleeper is injectable
    }

    #[test]
    fn test_wait_usecase_default_uses_real_sleeper() {
        // This test demonstrates that WaitUseCaseImpl::new() uses RealSleeper by default
        let repo = Arc::new(MockSessionRepository::new());
        let _usecase: WaitUseCaseImpl<_, RealSleeper> = WaitUseCaseImpl::new(repo);
        // Type annotation confirms RealSleeper is the default
    }

    #[test]
    fn test_wait_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = WaitUseCaseImpl::new(repo);

        let input = WaitInput {
            session_id: None,
            text: Some("loading".to_string()),
            timeout_ms: 5000,
            condition: None,
            target: None,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_wait_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = WaitUseCaseImpl::new(repo);

        let input = WaitInput {
            session_id: Some(SessionId::new("missing")),
            text: Some("ready".to_string()),
            timeout_ms: 1000,
            condition: None,
            target: None,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_wait_usecase_returns_error_with_stable_condition() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = WaitUseCaseImpl::new(repo);

        let input = WaitInput {
            session_id: None,
            text: None,
            timeout_ms: 5000,
            condition: Some("stable".to_string()),
            target: None,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_wait_usecase_returns_error_with_element_condition() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = WaitUseCaseImpl::new(repo);

        let input = WaitInput {
            session_id: None,
            text: Some("@btn1".to_string()),
            timeout_ms: 5000,
            condition: Some("element".to_string()),
            target: None,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }
}
