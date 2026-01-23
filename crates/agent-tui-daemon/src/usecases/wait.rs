use std::sync::Arc;
use std::time::{Duration, Instant};

use agent_tui_common::mutex_lock_or_recover;

use crate::domain::{WaitInput, WaitOutput};
use crate::error::SessionError;
use crate::repository::SessionRepository;
use crate::wait::{StableTracker, WaitCondition, check_condition};

/// Use case for waiting for a condition.
pub trait WaitUseCase: Send + Sync {
    fn execute(&self, input: WaitInput) -> Result<WaitOutput, SessionError>;
}

/// Implementation of the wait use case.
pub struct WaitUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> WaitUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> WaitUseCase for WaitUseCaseImpl<R> {
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

            std::thread::sleep(poll_interval);
        }
    }
}
