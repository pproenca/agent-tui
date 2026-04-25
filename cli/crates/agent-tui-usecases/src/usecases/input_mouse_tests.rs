use crate::domain::MouseButton;
use crate::domain::MouseClickInput;
use crate::domain::MouseDownInput;
use crate::domain::MouseMoveInput;
use crate::domain::MouseUpInput;
use crate::domain::SessionId;
use crate::usecases::MouseClickUseCase;
use crate::usecases::MouseClickUseCaseImpl;
use crate::usecases::MouseDownUseCase;
use crate::usecases::MouseDownUseCaseImpl;
use crate::usecases::MouseMoveUseCase;
use crate::usecases::MouseMoveUseCaseImpl;
use crate::usecases::MouseUpUseCase;
use crate::usecases::MouseUpUseCaseImpl;
use crate::usecases::ports::test_support::MockSession;
use crate::usecases::ports::test_support::MockSessionRepository;
use std::sync::Arc;

#[test]
fn test_mouse_click_usecase_calls_session_mouse_click() {
    let session = Arc::new(MockSession::new("test-session"));
    let session_id = SessionId::try_new("test-session").unwrap();
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_session_handle(session.clone())
            .build(),
    );
    let usecase = MouseClickUseCaseImpl::new(repo);

    let input = MouseClickInput {
        session_id: Some(session_id),
        col: 10,
        row: 20,
        button: MouseButton::Right,
    };

    usecase
        .execute(input)
        .expect("usecase execution should succeed");

    let calls = session.mouse_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], "click 10x20 right");
}

#[test]
fn test_mouse_move_usecase_calls_session_mouse_move() {
    let session = Arc::new(MockSession::new("test-session"));
    let session_id = SessionId::try_new("test-session").unwrap();
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_session_handle(session.clone())
            .build(),
    );
    let usecase = MouseMoveUseCaseImpl::new(repo);

    let input = MouseMoveInput {
        session_id: Some(session_id),
        col: 15,
        row: 25,
    };

    usecase
        .execute(input)
        .expect("usecase execution should succeed");

    let calls = session.mouse_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], "move 15x25");
}

#[test]
fn test_mouse_down_usecase_calls_session_mouse_down() {
    let session = Arc::new(MockSession::new("test-session"));
    let session_id = SessionId::try_new("test-session").unwrap();
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_session_handle(session.clone())
            .build(),
    );
    let usecase = MouseDownUseCaseImpl::new(repo);

    let input = MouseDownInput {
        session_id: Some(session_id),
        col: 5,
        row: 5,
        button: MouseButton::Left,
    };

    usecase
        .execute(input)
        .expect("usecase execution should succeed");

    let calls = session.mouse_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], "down 5x5 left");
}

#[test]
fn test_mouse_up_usecase_calls_session_mouse_up() {
    let session = Arc::new(MockSession::new("test-session"));
    let session_id = SessionId::try_new("test-session").unwrap();
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_session_handle(session.clone())
            .build(),
    );
    let usecase = MouseUpUseCaseImpl::new(repo);

    let input = MouseUpInput {
        session_id: Some(session_id),
        col: 8,
        row: 8,
        button: MouseButton::Middle,
    };

    usecase
        .execute(input)
        .expect("usecase execution should succeed");

    let calls = session.mouse_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], "up 8x8 middle");
}
