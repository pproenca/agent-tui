//! Wait use case.

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
        let session = self.repository.resolve(input.session_id.as_ref())?;
        let timeout = Duration::from_millis(input.timeout_ms);
        let start = self.clock.now();

        let condition = WaitCondition::parse(input.condition, input.text.as_deref())
            .map_err(|e| SessionError::InvalidKey(e.to_string()))?;

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

            let remaining = timeout.saturating_sub(self.clock.elapsed(start));
            let wait_timeout = remaining.min(poll_interval);
            let _ = subscription.wait(Some(wait_timeout));
        }
    }
}

#[cfg(test)]
#[path = "wait_tests.rs"]
mod tests;
