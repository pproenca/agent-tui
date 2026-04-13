use super::*;

#[test]
fn test_attach_error_code() {
    let err = AttachError::PtyWrite("broken pipe".into());
    assert_eq!(err.code(), error_codes::PTY_ERROR);

    let err = AttachError::PtyRead("timeout".into());
    assert_eq!(err.code(), error_codes::PTY_ERROR);

    let err = AttachError::EventRead;
    assert_eq!(err.code(), error_codes::PTY_ERROR);
}

#[test]
fn test_attach_error_category() {
    let err = AttachError::PtyWrite("x".into());
    assert_eq!(err.category(), ErrorCategory::External);

    let err = AttachError::EventRead;
    assert_eq!(err.category(), ErrorCategory::External);
}

#[test]
fn test_attach_error_context() {
    let err = AttachError::PtyWrite("broken pipe".into());
    let ctx = err.context();
    assert_eq!(ctx.operation, "pty_write");
    assert_eq!(ctx.reason, "broken pipe");

    let err = AttachError::PtyRead("timeout".into());
    let ctx = err.context();
    assert_eq!(ctx.operation, "pty_read");
    assert_eq!(ctx.reason, "timeout");
}

#[test]
fn test_attach_error_suggestion() {
    let err = AttachError::PtyWrite("x".into());
    assert!(err.suggestion().contains("session"));

    let err = AttachError::EventRead;
    assert!(err.suggestion().contains("terminal"));
}

#[test]
fn test_attach_error_is_retryable() {
    assert!(AttachError::PtyWrite("x".into()).is_retryable());
    assert!(AttachError::PtyRead("x".into()).is_retryable());
    assert!(!AttachError::EventRead.is_retryable());
}

#[test]
fn test_attach_error_exit_code() {
    let err = AttachError::PtyWrite("x".into());
    assert_eq!(err.exit_code(), 74);
}

#[test]
fn test_attach_error_to_payload() {
    let err = AttachError::PtyRead("connection reset".into());
    let payload = err.to_payload();
    assert_eq!(payload.code, error_codes::PTY_ERROR);
    assert_eq!(payload.category, "external");
    assert!(payload.retryable);
    assert_eq!(payload.context.operation, "pty_read");
}
