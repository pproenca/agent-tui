use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::domain::{WaitInput, WaitOutput};
use crate::usecases::ports::{SessionError, SessionRepository, Sleeper};
use crate::usecases::wait_condition::{StableTracker, WaitCondition, check_condition};

pub trait WaitUseCase: Send + Sync {
    fn execute(&self, input: WaitInput) -> Result<WaitOutput, SessionError>;
}

pub struct WaitUseCaseImpl<R: SessionRepository, S: Sleeper> {
    repository: Arc<R>,
    sleeper: S,
}

impl<R: SessionRepository, S: Sleeper> WaitUseCaseImpl<R, S> {
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

        let condition = resolve_wait_condition(
            input.condition.as_deref(),
            input.target.as_deref(),
            input.text.as_deref(),
        );

        let mut stable_tracker = StableTracker::new(3);
        let poll_interval = Duration::from_millis(50);

        loop {
            session.update()?;

            if check_condition(session.as_ref(), &condition, &mut stable_tracker) {
                return Ok(WaitOutput {
                    found: true,
                    elapsed_ms: start.elapsed().as_millis() as u64,
                });
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

fn resolve_wait_condition(
    condition: Option<&str>,
    target: Option<&str>,
    text: Option<&str>,
) -> WaitCondition {
    match (condition, target, text) {
        (Some("stable"), _, _) => WaitCondition::Stable,
        (Some("element"), Some(target), _) => WaitCondition::Element(target.to_string()),
        (Some("focused"), Some(target), _) => WaitCondition::Focused(target.to_string()),
        (Some("not_visible"), Some(target), _) => WaitCondition::NotVisible(target.to_string()),
        (Some("text_gone"), Some(target), _) => WaitCondition::TextGone(target.to_string()),
        (Some("value"), Some(target), _) => {
            if let Some((element_ref, expected)) = target.split_once('=') {
                WaitCondition::Value {
                    element: element_ref.to_string(),
                    expected: expected.to_string(),
                }
            } else if let Some(expected) = text {
                WaitCondition::Value {
                    element: target.to_string(),
                    expected: expected.to_string(),
                }
            } else {
                WaitCondition::Text(target.to_string())
            }
        }
        (_, _, Some(text)) => WaitCondition::Text(text.to_string()),
        (None, None, None) => WaitCondition::Stable,
        _ => WaitCondition::Stable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SessionId;
    use crate::usecases::ports::MockSleeper;
    use crate::infra::daemon::test_support::{MockError, MockSessionRepository};

    #[test]
    fn test_wait_usecase_can_be_constructed_with_mock_sleeper() {
        let repo = Arc::new(MockSessionRepository::new());
        let mock_sleeper = MockSleeper::new();
        let _usecase = WaitUseCaseImpl::with_sleeper(repo, mock_sleeper);
    }

    #[test]
    fn test_wait_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = WaitUseCaseImpl::with_sleeper(repo, MockSleeper::new());

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
        let usecase = WaitUseCaseImpl::with_sleeper(repo, MockSleeper::new());

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
        let usecase = WaitUseCaseImpl::with_sleeper(repo, MockSleeper::new());

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
        let usecase = WaitUseCaseImpl::with_sleeper(repo, MockSleeper::new());

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

    #[test]
    fn test_resolve_wait_condition_value_uses_text_when_no_equals() {
        let condition = resolve_wait_condition(Some("value"), Some("@inp1"), Some("hello"));

        assert!(
            matches!(condition, WaitCondition::Value { element, expected } if element == "@inp1" && expected == "hello")
        );
    }
}
