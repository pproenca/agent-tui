//! Snapshot use case.

use std::sync::Arc;

use crate::domain::SnapshotInput;
use crate::domain::SnapshotOutput;
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
    fn execute(&self, input: SnapshotInput) -> Result<SnapshotOutput, SessionError> {
        if input.region.is_some() {
            return Err(SessionError::InvalidInput {
                field: "region".to_string(),
                reason: "Named snapshot regions are not supported".to_string(),
            });
        }

        let session = self.repository.resolve(input.session_id.as_ref())?;

        session.update()?;

        let screenshot = session.screen_text();
        let session_id = session.session_id();

        let cursor = if input.include_cursor {
            Some(session.cursor())
        } else {
            None
        };

        let rendered = if input.include_render {
            Some(session.screen_render())
        } else {
            None
        };
        let compact_rendered = if input.include_render {
            Some(session.screen_render_compact())
        } else {
            None
        };

        Ok(SnapshotOutput {
            session_id,
            screenshot,
            cursor,
            rendered,
            compact_rendered,
        })
    }
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;
