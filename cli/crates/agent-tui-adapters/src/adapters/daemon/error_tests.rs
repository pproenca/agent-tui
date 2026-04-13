use super::*;

#[test]
fn test_session_not_found_code() {
    let err = DomainError::SessionNotFound {
        session_id: "abc123".into(),
    };
    assert_eq!(err.code(), error_codes::SESSION_NOT_FOUND);
}

#[test]
fn test_lock_timeout_is_retryable() {
    let err = DomainError::LockTimeout {
        session_id: Some("abc".into()),
    };
    assert!(error_codes::is_retryable(err.code()));
}

#[test]
fn test_from_session_error() {
    let session_err = SessionError::NotFound("test123".into());
    let domain_err: DomainError = session_err.into();
    assert_eq!(domain_err.code(), error_codes::SESSION_NOT_FOUND);
}

#[test]
fn test_from_session_error_preserves_persistence_variant() {
    let session_err = SessionError::Persistence {
        operation: "save".into(),
        reason: "disk full".into(),
        source: None,
    };
    let domain_err: DomainError = session_err.into();

    match domain_err {
        DomainError::PersistenceError { operation, reason } => {
            assert_eq!(operation, "save");
            assert_eq!(reason, "disk full");
        }
        other => panic!("expected PersistenceError, got {other:?}"),
    }
}

#[test]
fn test_display_session_not_found() {
    let err = DomainError::SessionNotFound {
        session_id: "abc".into(),
    };
    assert_eq!(err.to_string(), "Session not found: abc");
}

#[test]
fn test_session_error_not_found_code() {
    let err = SessionError::NotFound("abc123".into());
    assert_eq!(err.code(), error_codes::SESSION_NOT_FOUND);
}

#[test]
fn test_session_error_no_active_session_code() {
    let err = SessionError::NoActiveSession;
    assert_eq!(err.code(), error_codes::NO_ACTIVE_SESSION);
}

#[test]
fn test_session_error_invalid_key_code() {
    let err = SessionError::InvalidKey("BadKey".into());
    assert_eq!(err.code(), error_codes::INVALID_KEY);
}

#[test]
fn test_session_error_invalid_input_code() {
    let err = SessionError::InvalidInput {
        field: "region".into(),
        reason: "not supported".into(),
    };
    assert_eq!(err.code(), error_codes::INVALID_INPUT);
}

#[test]
fn test_session_error_limit_reached_code() {
    let err = SessionError::LimitReached(16);
    assert_eq!(err.code(), error_codes::SESSION_LIMIT);
}

#[test]
fn test_session_error_category() {
    let err = SessionError::NotFound("abc".into());
    assert_eq!(err.category(), ErrorCategory::NotFound);

    let err = SessionError::InvalidKey("x".into());
    assert_eq!(err.category(), ErrorCategory::InvalidInput);

    let err = SessionError::InvalidInput {
        field: "region".into(),
        reason: "not supported".into(),
    };
    assert_eq!(err.category(), ErrorCategory::InvalidInput);

    let err = SessionError::LimitReached(10);
    assert_eq!(err.category(), ErrorCategory::Busy);
}

#[test]
fn test_session_error_context() {
    let err = SessionError::NotFound("sess123".into());
    let ctx = err.context();
    assert_eq!(ctx["session_id"], "sess123");

    let err = SessionError::LimitReached(16);
    let ctx = err.context();
    assert_eq!(ctx["max_sessions"], 16);

    let err = SessionError::InvalidInput {
        field: "region".into(),
        reason: "not supported".into(),
    };
    let ctx = err.context();
    assert_eq!(ctx["field"], "region");
    assert_eq!(ctx["reason"], "not supported");
}

#[test]
fn test_session_error_suggestion() {
    let err = SessionError::NotFound("x".into());
    assert!(err.suggestion().contains("sessions"));

    let err = SessionError::InvalidKey("x".into());
    assert!(err.suggestion().contains("Enter"));

    let err = SessionError::InvalidInput {
        field: "region".into(),
        reason: "not supported".into(),
    };
    assert!(err.suggestion().contains("Adjust"));
}

#[test]
fn test_session_error_is_retryable() {
    assert!(!SessionError::NotFound("x".into()).is_retryable());
    assert!(!SessionError::NoActiveSession.is_retryable());
    assert!(!SessionError::InvalidKey("x".into()).is_retryable());
    assert!(
        !SessionError::InvalidInput {
            field: "region".into(),
            reason: "not supported".into(),
        }
        .is_retryable()
    );
}

#[test]
fn test_session_error_persistence_code() {
    let err = SessionError::Persistence {
        operation: "save".into(),
        reason: "disk full".into(),
        source: None,
    };
    assert_eq!(err.code(), error_codes::PERSISTENCE_ERROR);
}

#[test]
fn test_session_error_persistence_context() {
    let err = SessionError::Persistence {
        operation: "write_json".into(),
        reason: "permission denied".into(),
        source: None,
    };
    let ctx = err.context();
    assert_eq!(ctx["operation"], "write_json");
    assert_eq!(ctx["reason"], "permission denied");
}

#[test]
fn test_session_error_persistence_is_retryable() {
    let err = SessionError::Persistence {
        operation: "save".into(),
        reason: "disk full".into(),
        source: None,
    };
    assert!(err.is_retryable());
}

#[test]
fn test_session_error_persistence_display() {
    let err = SessionError::Persistence {
        operation: "write".into(),
        reason: "disk full".into(),
        source: None,
    };
    assert_eq!(err.to_string(), "Persistence error during write: disk full");
}

#[test]
fn test_terminal_error_conversion_preserves_context() {
    let terminal_err = TerminalError::Write {
        reason: "broken pipe".into(),
        source: None,
    };
    let session_err = SessionError::Terminal(terminal_err);
    let domain_err: DomainError = session_err.into();

    match domain_err {
        DomainError::TerminalError { operation, reason } => {
            assert_eq!(operation, "write");
            assert_eq!(reason, "broken pipe");
        }
        _ => panic!("Expected TerminalError variant"),
    }
}

#[test]
fn test_domain_persistence_error_code_and_context() {
    let err = DomainError::PersistenceError {
        operation: "write".into(),
        reason: "permission denied".into(),
    };

    assert_eq!(err.code(), error_codes::PERSISTENCE_ERROR);
    assert_eq!(err.context()["operation"], "write");
    assert_eq!(err.context()["reason"], "permission denied");
}
