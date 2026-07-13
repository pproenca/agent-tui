use super::*;
use crate::domain::SessionId;
use crate::test_support::MockError;
use crate::test_support::MockSessionRepository;
use std::sync::Arc;

#[test]
fn test_keystroke_usecase_returns_error_when_no_active_session() {
    let repo = Arc::new(MockSessionRepository::new());
    let input = KeystrokeInput {
        session_id: None,
        key: "Enter".to_string(),
    };

    let result = keystroke(repo.as_ref(), input);
    assert!(matches!(result, Err(SessionError::NoActiveSession)));
}

#[test]
fn test_keystroke_usecase_returns_error_when_session_not_found() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_resolve_error(MockError::NotFound("missing".to_string()))
            .build(),
    );
    let input = KeystrokeInput {
        session_id: Some(SessionId::try_new("missing").expect("valid session id")),
        key: "Tab".to_string(),
    };

    let result = keystroke(repo.as_ref(), input);
    assert!(matches!(result, Err(SessionError::NotFound(_))));
}

#[test]
fn test_type_usecase_returns_error_when_no_active_session() {
    let repo = Arc::new(MockSessionRepository::new());
    let input = TypeInput {
        session_id: None,
        text: "hello world".to_string(),
    };

    let result = type_text(repo.as_ref(), input);
    assert!(matches!(result, Err(SessionError::NoActiveSession)));
}

#[test]
fn test_type_usecase_returns_error_when_session_not_found() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_resolve_error(MockError::NotFound("missing".to_string()))
            .build(),
    );
    let input = TypeInput {
        session_id: Some(SessionId::try_new("missing").expect("valid session id")),
        text: "test text".to_string(),
    };

    let result = type_text(repo.as_ref(), input);
    assert!(matches!(result, Err(SessionError::NotFound(_))));
}

#[test]
fn test_keydown_usecase_returns_error_when_no_active_session() {
    let repo = Arc::new(MockSessionRepository::new());
    let input = KeydownInput {
        session_id: None,
        key: "Ctrl".to_string(),
    };

    let result = keydown(repo.as_ref(), input);
    assert!(matches!(result, Err(SessionError::NoActiveSession)));
}

#[test]
fn test_keydown_usecase_returns_error_when_session_not_found() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_resolve_error(MockError::NotFound("missing".to_string()))
            .build(),
    );
    let input = KeydownInput {
        session_id: Some(SessionId::try_new("missing").expect("valid session id")),
        key: "Shift".to_string(),
    };

    let result = keydown(repo.as_ref(), input);
    assert!(matches!(result, Err(SessionError::NotFound(_))));
}

#[test]
fn test_keyup_usecase_returns_error_when_no_active_session() {
    let repo = Arc::new(MockSessionRepository::new());
    let input = KeyupInput {
        session_id: None,
        key: "Ctrl".to_string(),
    };

    let result = keyup(repo.as_ref(), input);
    assert!(matches!(result, Err(SessionError::NoActiveSession)));
}

#[test]
fn test_keyup_usecase_returns_error_when_session_not_found() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_resolve_error(MockError::NotFound("missing".to_string()))
            .build(),
    );
    let input = KeyupInput {
        session_id: Some(SessionId::try_new("missing").expect("valid session id")),
        key: "Alt".to_string(),
    };

    let result = keyup(repo.as_ref(), input);
    assert!(matches!(result, Err(SessionError::NotFound(_))));
}
