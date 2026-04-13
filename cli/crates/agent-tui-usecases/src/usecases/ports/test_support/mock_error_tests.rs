use super::*;

#[test]
fn test_mock_error_conversion() {
    let err = MockError::NotFound("test".to_string());
    let session_err = err.to_session_error();
    assert!(matches!(session_err, SessionError::NotFound(id) if id == "test"));

    let err = MockError::LimitReached(10);
    let session_err = err.to_session_error();
    assert!(matches!(session_err, SessionError::LimitReached(10)));

    let err = MockError::NotRunning("test".to_string());
    let session_err = err.to_session_error();
    assert!(matches!(
        session_err,
        SessionError::NotRunning { session_id } if session_id == "test"
    ));

    let err = MockError::NoActiveSession;
    let session_err = err.to_session_error();
    assert!(matches!(session_err, SessionError::NoActiveSession));
}
