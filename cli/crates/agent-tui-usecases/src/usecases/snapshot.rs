//! Snapshot use case.

use crate::domain::SnapshotInput;
use crate::domain::SnapshotOutput;
use crate::usecases::ports::SessionError;
use crate::usecases::ports::SessionRepository;

pub fn snapshot<R: SessionRepository + ?Sized>(
    repository: &R,
    input: SnapshotInput,
) -> Result<SnapshotOutput, SessionError> {
    if input.region.is_some() {
        return Err(SessionError::InvalidInput {
            field: "region".to_string(),
            reason: "Named snapshot regions are not supported".to_string(),
        });
    }

    let session = repository.resolve(input.session_id.as_ref())?;

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

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;
