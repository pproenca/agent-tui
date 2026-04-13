use super::*;

#[test]
fn test_is_retryable_lock_timeout() {
    assert!(is_retryable(LOCK_TIMEOUT));
}

#[test]
fn test_is_retryable_generic() {
    assert!(is_retryable(GENERIC_ERROR));
}

#[test]
fn test_not_retryable_session_not_found() {
    assert!(!is_retryable(SESSION_NOT_FOUND));
}

#[test]
fn test_category_for_code_not_found() {
    assert_eq!(
        category_for_code(SESSION_NOT_FOUND),
        ErrorCategory::NotFound
    );
    assert_eq!(
        category_for_code(NO_ACTIVE_SESSION),
        ErrorCategory::NotFound
    );
}

#[test]
fn test_category_for_code_invalid_input() {
    assert_eq!(category_for_code(INVALID_KEY), ErrorCategory::InvalidInput);
    assert_eq!(
        category_for_code(INVALID_INPUT),
        ErrorCategory::InvalidInput
    );
}

#[test]
fn test_category_for_code_busy() {
    assert_eq!(category_for_code(SESSION_LIMIT), ErrorCategory::Busy);
    assert_eq!(category_for_code(LOCK_TIMEOUT), ErrorCategory::Busy);
}

#[test]
fn test_category_for_code_external() {
    assert_eq!(category_for_code(PTY_ERROR), ErrorCategory::External);
    assert_eq!(
        category_for_code(COMMAND_NOT_FOUND),
        ErrorCategory::External
    );
    assert_eq!(
        category_for_code(PERMISSION_DENIED),
        ErrorCategory::External
    );
    assert_eq!(category_for_code(DAEMON_ERROR), ErrorCategory::External);
    assert_eq!(
        category_for_code(PERSISTENCE_ERROR),
        ErrorCategory::External
    );
}

#[test]
fn test_category_for_code_timeout() {
    assert_eq!(category_for_code(WAIT_TIMEOUT), ErrorCategory::Timeout);
}

#[test]
fn test_category_as_str() {
    assert_eq!(ErrorCategory::NotFound.as_str(), "not_found");
    assert_eq!(ErrorCategory::InvalidInput.as_str(), "invalid_input");
    assert_eq!(ErrorCategory::Busy.as_str(), "busy");
    assert_eq!(ErrorCategory::Internal.as_str(), "internal");
    assert_eq!(ErrorCategory::External.as_str(), "external");
    assert_eq!(ErrorCategory::Timeout.as_str(), "timeout");
}

#[test]
fn test_category_from_str() {
    assert_eq!(
        "not_found".parse::<ErrorCategory>(),
        Ok(ErrorCategory::NotFound)
    );
    assert_eq!(
        "invalid_input".parse::<ErrorCategory>(),
        Ok(ErrorCategory::InvalidInput)
    );
    assert_eq!("busy".parse::<ErrorCategory>(), Ok(ErrorCategory::Busy));
    assert_eq!(
        "internal".parse::<ErrorCategory>(),
        Ok(ErrorCategory::Internal)
    );
    assert_eq!(
        "external".parse::<ErrorCategory>(),
        Ok(ErrorCategory::External)
    );
    assert_eq!(
        "timeout".parse::<ErrorCategory>(),
        Ok(ErrorCategory::Timeout)
    );
    assert!("unknown".parse::<ErrorCategory>().is_err());
}
