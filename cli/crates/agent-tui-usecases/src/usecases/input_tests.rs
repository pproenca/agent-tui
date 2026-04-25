use super::*;
use crate::domain::MouseButton;
use crate::domain::SessionId;
use crate::test_support::MockError;
use crate::test_support::MockSessionRepository;

#[test]
fn test_keystroke_usecase_returns_error_when_no_active_session() {
    let repo = Arc::new(MockSessionRepository::new());
    let usecase = KeystrokeUseCaseImpl::new(repo);

    let input = KeystrokeInput {
        session_id: None,
        key: "Enter".to_string(),
    };

    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NoActiveSession)));
}

#[test]
fn test_keystroke_usecase_returns_error_when_session_not_found() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_resolve_error(MockError::NotFound("missing".to_string()))
            .build(),
    );
    let usecase = KeystrokeUseCaseImpl::new(repo);

    let input = KeystrokeInput {
        session_id: Some(SessionId::try_new("missing").expect("valid session id")),
        key: "Tab".to_string(),
    };

    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NotFound(_))));
}

#[test]
fn test_type_usecase_returns_error_when_no_active_session() {
    let repo = Arc::new(MockSessionRepository::new());
    let usecase = TypeUseCaseImpl::new(repo);

    let input = TypeInput {
        session_id: None,
        text: "hello world".to_string(),
    };

    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NoActiveSession)));
}

#[test]
fn test_type_usecase_returns_error_when_session_not_found() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_resolve_error(MockError::NotFound("missing".to_string()))
            .build(),
    );
    let usecase = TypeUseCaseImpl::new(repo);

    let input = TypeInput {
        session_id: Some(SessionId::try_new("missing").expect("valid session id")),
        text: "test text".to_string(),
    };

    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NotFound(_))));
}

#[test]
fn test_keydown_usecase_returns_error_when_no_active_session() {
    let repo = Arc::new(MockSessionRepository::new());
    let usecase = KeydownUseCaseImpl::new(repo);

    let input = KeydownInput {
        session_id: None,
        key: "Ctrl".to_string(),
    };

    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NoActiveSession)));
}

#[test]
fn test_keydown_usecase_returns_error_when_session_not_found() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_resolve_error(MockError::NotFound("missing".to_string()))
            .build(),
    );
    let usecase = KeydownUseCaseImpl::new(repo);

    let input = KeydownInput {
        session_id: Some(SessionId::try_new("missing").expect("valid session id")),
        key: "Shift".to_string(),
    };

    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NotFound(_))));
}

#[test]
fn test_keyup_usecase_returns_error_when_no_active_session() {
    let repo = Arc::new(MockSessionRepository::new());
    let usecase = KeyupUseCaseImpl::new(repo);

    let input = KeyupInput {
        session_id: None,
        key: "Ctrl".to_string(),
    };

    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NoActiveSession)));
}

#[test]
fn test_keyup_usecase_returns_error_when_session_not_found() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_resolve_error(MockError::NotFound("missing".to_string()))
            .build(),
    );
    let usecase = KeyupUseCaseImpl::new(repo);

    let input = KeyupInput {
        session_id: Some(SessionId::try_new("missing").expect("valid session id")),
        key: "Alt".to_string(),
    };

    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NotFound(_))));
}

#[test]
fn test_mouse_click_usecase_returns_error_when_no_active_session() {
    let repo = Arc::new(MockSessionRepository::new());
    let usecase = MouseClickUseCaseImpl::new(repo);

    let input = MouseClickInput {
        session_id: None,
        col: 5,
        row: 10,
        button: MouseButton::Left,
    };

    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NoActiveSession)));
}

#[test]
fn test_mouse_click_usecase_returns_error_when_session_not_found() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_resolve_error(MockError::NotFound("missing".to_string()))
            .build(),
    );
    let usecase = MouseClickUseCaseImpl::new(repo);

    let input = MouseClickInput {
        session_id: Some(SessionId::try_new("missing").expect("valid session id")),
        col: 5,
        row: 10,
        button: MouseButton::Right,
    };

    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NotFound(_))));
}

#[test]
fn test_mouse_move_usecase_returns_error_when_no_active_session() {
    let repo = Arc::new(MockSessionRepository::new());
    let usecase = MouseMoveUseCaseImpl::new(repo);

    let input = MouseMoveInput {
        session_id: None,
        col: 5,
        row: 10,
    };

    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NoActiveSession)));
}

#[test]
fn test_mouse_down_usecase_returns_error_when_no_active_session() {
    let repo = Arc::new(MockSessionRepository::new());
    let usecase = MouseDownUseCaseImpl::new(repo);

    let input = MouseDownInput {
        session_id: None,
        col: 5,
        row: 10,
        button: MouseButton::Middle,
    };

    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NoActiveSession)));
}

#[test]
fn test_mouse_up_usecase_returns_error_when_session_not_found() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_resolve_error(MockError::NotFound("missing".to_string()))
            .build(),
    );
    let usecase = MouseUpUseCaseImpl::new(repo);

    let input = MouseUpInput {
        session_id: Some(SessionId::try_new("missing").expect("valid session id")),
        col: 5,
        row: 10,
        button: MouseButton::Left,
    };

    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NotFound(_))));
}
