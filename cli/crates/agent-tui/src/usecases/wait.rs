use std::sync::Arc;
use std::time::Duration;

use crate::domain::WaitInput;
use crate::domain::WaitOutput;
use crate::usecases::ports::Clock;
use crate::usecases::ports::SessionError;
use crate::usecases::ports::SessionRepository;
use crate::usecases::wait_condition::StableTracker;
use crate::usecases::wait_condition::WaitCondition;
use crate::usecases::wait_condition::check_condition;

pub trait WaitUseCase: Send + Sync {
    fn execute(&self, input: WaitInput) -> Result<WaitOutput, SessionError>;
}

pub struct WaitUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
    clock: Arc<dyn Clock>,
}

impl<R: SessionRepository> WaitUseCaseImpl<R> {
    pub fn new(repository: Arc<R>, clock: Arc<dyn Clock>) -> Self {
        Self { repository, clock }
    }
}

impl<R: SessionRepository> WaitUseCase for WaitUseCaseImpl<R> {
    fn execute(&self, input: WaitInput) -> Result<WaitOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let timeout = Duration::from_millis(input.timeout_ms);
        let start = self.clock.now();

        let condition = WaitCondition::parse(input.condition, input.text.as_deref())
            .unwrap_or(WaitCondition::Stable);

        let mut stable_tracker = StableTracker::new(3);
        let poll_interval = Duration::from_millis(50);
        let subscription = session.stream_subscribe();

        loop {
            session.update()?;

            if check_condition(session.as_ref(), &condition, &mut stable_tracker) {
                let elapsed_ms = self.clock.elapsed_ms(start);
                return Ok(WaitOutput {
                    found: true,
                    elapsed_ms,
                });
            }

            if self.clock.elapsed(start) >= timeout {
                let elapsed_ms = self.clock.elapsed_ms(start);
                return Ok(WaitOutput {
                    found: false,
                    elapsed_ms,
                });
            }

            let _ = subscription.wait(Some(poll_interval));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SessionId;
    use crate::test_support::MockError;
    use crate::test_support::MockSessionRepository;
    use std::time::Instant;

    #[derive(Default)]
    struct TestClock;

    impl Clock for TestClock {
        fn now(&self) -> Instant {
            Instant::now()
        }
    }

    #[test]
    fn test_wait_usecase_can_be_constructed_with_mock_clock() {
        let repo = Arc::new(MockSessionRepository::new());
        let clock = Arc::new(TestClock);
        let _usecase = WaitUseCaseImpl::new(repo, clock);
    }

    #[test]
    fn test_wait_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let clock = Arc::new(TestClock);
        let usecase = WaitUseCaseImpl::new(repo, clock);

        let input = WaitInput {
            session_id: None,
            text: Some("loading".to_string()),
            timeout_ms: 5000,
            condition: None,
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
        let clock = Arc::new(TestClock);
        let usecase = WaitUseCaseImpl::new(repo, clock);

        let input = WaitInput {
            session_id: Some(SessionId::new("missing")),
            text: Some("ready".to_string()),
            timeout_ms: 1000,
            condition: None,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    #[test]
    fn test_wait_usecase_returns_error_with_stable_condition() {
        let repo = Arc::new(MockSessionRepository::new());
        let clock = Arc::new(TestClock);
        let usecase = WaitUseCaseImpl::new(repo, clock);

        let input = WaitInput {
            session_id: None,
            text: None,
            timeout_ms: 5000,
            condition: Some(crate::domain::WaitConditionType::Stable),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    // WaitCondition parsing is covered in wait_condition.rs tests.
}
