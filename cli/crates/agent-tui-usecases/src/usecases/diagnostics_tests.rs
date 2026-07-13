use super::*;

use crate::domain::SessionId;
use crate::test_support::MockError;
use crate::test_support::MockSessionRepository;
use std::sync::Arc;
#[test]
fn test_terminal_write_usecase_returns_error_when_no_active_session() {
    let repo = Arc::new(MockSessionRepository::new());
    let input = TerminalWriteInput {
        session_id: None,
        data: b"hello".to_vec(),
    };

    let result = terminal_write(repo.as_ref(), input);
    assert!(matches!(result, Err(SessionError::NoActiveSession)));
}

#[test]
fn test_terminal_write_usecase_returns_error_when_session_not_found() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_resolve_error(MockError::NotFound("missing".to_string()))
            .build(),
    );
    let input = TerminalWriteInput {
        session_id: Some(SessionId::try_new("missing").expect("valid session id")),
        data: b"test data".to_vec(),
    };

    let result = terminal_write(repo.as_ref(), input);
    assert!(matches!(result, Err(SessionError::NotFound(_))));
}
