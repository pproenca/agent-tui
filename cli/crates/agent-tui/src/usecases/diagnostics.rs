//! Diagnostics use case.

use std::sync::Arc;

use crate::domain::TerminalWriteInput;
use crate::domain::TerminalWriteOutput;
use crate::usecases::ports::SessionError;
use crate::usecases::ports::SessionRepository;

pub trait TerminalWriteUseCase: Send + Sync {
    fn execute(&self, input: TerminalWriteInput) -> Result<TerminalWriteOutput, SessionError>;
}

pub struct TerminalWriteUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> TerminalWriteUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> TerminalWriteUseCase for TerminalWriteUseCaseImpl<R> {
    fn execute(&self, input: TerminalWriteInput) -> Result<TerminalWriteOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_ref())?;
        let bytes_len = input.data.len();
        session.terminal_write(&input.data)?;
        Ok(TerminalWriteOutput {
            session_id: session.session_id(),
            bytes_written: bytes_len,
            success: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::domain::SessionId;
    use crate::test_support::MockError;
    use crate::test_support::MockSessionRepository;
    #[test]
    fn test_terminal_write_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = TerminalWriteUseCaseImpl::new(repo);

        let input = TerminalWriteInput {
            session_id: None,
            data: b"hello".to_vec(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NoActiveSession)));
    }

    #[test]
    fn test_terminal_write_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_resolve_error(MockError::NotFound("missing".to_string()))
                .build(),
        );
        let usecase = TerminalWriteUseCaseImpl::new(repo);

        let input = TerminalWriteInput {
            session_id: Some(SessionId::new("missing")),
            data: b"test data".to_vec(),
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }
}
