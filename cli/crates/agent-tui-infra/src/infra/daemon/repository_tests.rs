use super::*;
use crate::usecases::ports::TerminalError;

#[test]
fn test_session_repository_trait_is_object_safe() {
    fn assert_object_safe<T>(_: &T)
    where
        T: SessionRepository + ?Sized,
    {
    }

    let manager = SessionManager::new().expect("session manager should initialize");
    assert_object_safe(&manager);
}

#[test]
fn test_session_ops_trait_is_usable_as_generic_bound() {
    use crate::usecases::ports::test_support::MockSession;

    fn assert_generic_bound<S: SessionOps + ?Sized>(_session: &S) {}

    let session = MockSession::new("test");
    assert_generic_bound(&session);
}

#[test]
fn test_wait_for_flush_ack_returns_ok_without_pump() {
    assert!(wait_for_flush_ack(None, Duration::ZERO).is_ok());
}

#[test]
fn test_wait_for_flush_ack_returns_timeout_error() {
    let (_tx, rx) = channel::bounded(1);

    let result = wait_for_flush_ack(Some(rx), Duration::ZERO);

    assert!(matches!(
        result,
        Err(SessionError::Terminal(TerminalError::Read { reason, .. }))
        if reason.contains("Timed out waiting for session state to flush")
    ));
}

#[test]
fn test_wait_for_flush_ack_returns_disconnect_error() {
    let (tx, rx) = channel::bounded(1);
    drop(tx);

    let result = wait_for_flush_ack(Some(rx), Duration::from_millis(1));

    assert!(matches!(
        result,
        Err(SessionError::Terminal(TerminalError::Read { reason, .. }))
        if reason.contains("Session state flush closed before it acknowledged")
    ));
}
