//! Wait use case.

use std::time::Duration;

use crate::domain::WaitInput;
use crate::domain::WaitOutput;
use crate::usecases::ports::Clock;
use crate::usecases::ports::SessionError;
use crate::usecases::ports::SessionRepository;
use crate::usecases::wait_condition::StableTracker;
use crate::usecases::wait_condition::WaitCondition;
use crate::usecases::wait_condition::check_condition;

pub fn wait<R, C>(repository: &R, clock: &C, input: WaitInput) -> Result<WaitOutput, SessionError>
where
    R: SessionRepository + ?Sized,
    C: Clock + ?Sized,
{
    let session = repository.resolve(input.session_id.as_ref())?;
    let timeout = Duration::from_millis(input.timeout_ms);
    let start = clock.now();

    let condition = WaitCondition::parse(input.condition, input.text.as_deref())
        .map_err(|e| SessionError::InvalidKey(e.to_string()))?;

    let mut stable_tracker = StableTracker::new(3);
    let poll_interval = Duration::from_millis(50);
    let subscription = session.stream_subscribe();

    loop {
        session.update()?;

        if check_condition(session.as_ref(), &condition, &mut stable_tracker) {
            let elapsed_ms = clock.elapsed_ms(start);
            return Ok(WaitOutput {
                found: true,
                elapsed_ms,
            });
        }

        if clock.elapsed(start) >= timeout {
            let elapsed_ms = clock.elapsed_ms(start);
            return Ok(WaitOutput {
                found: false,
                elapsed_ms,
            });
        }

        let remaining = timeout.saturating_sub(clock.elapsed(start));
        let wait_timeout = remaining.min(poll_interval);
        let _ = subscription.wait(Some(wait_timeout));
    }
}

#[cfg(test)]
#[path = "wait_tests.rs"]
mod tests;
