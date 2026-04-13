use super::*;

#[test]
fn test_pty_error_code() {
    let err = PtyError::Open {
        reason: "test".into(),
        source: None,
    };
    assert_eq!(err.code(), error_codes::PTY_ERROR);
}

#[test]
fn test_pty_error_category() {
    let err = PtyError::Write {
        reason: "broken pipe".into(),
        source: None,
    };
    assert_eq!(err.category(), ErrorCategory::External);
}

#[test]
fn test_pty_error_context() {
    let err = PtyError::Spawn {
        reason: "command not found".into(),
        kind: SpawnErrorKind::NotFound,
    };
    let ctx = err.context();
    assert_eq!(ctx.operation, "spawn");
    assert_eq!(ctx.reason, "command not found");
}

#[test]
fn test_pty_error_suggestion_not_found() {
    let err = PtyError::Spawn {
        reason: "No such file or directory".into(),
        kind: SpawnErrorKind::NotFound,
    };
    assert!(err.suggestion().contains("not found"));
}

#[test]
fn test_pty_error_suggestion_permission() {
    let err = PtyError::Spawn {
        reason: "Permission denied".into(),
        kind: SpawnErrorKind::PermissionDenied,
    };
    assert!(err.suggestion().contains("Permission"));
}

#[test]
fn test_pty_error_is_retryable() {
    assert!(
        PtyError::Read {
            reason: "timeout".into(),
            source: None,
        }
        .is_retryable()
    );
    assert!(
        PtyError::Write {
            reason: "broken pipe".into(),
            source: None,
        }
        .is_retryable()
    );
    assert!(
        !PtyError::Open {
            reason: "failed".into(),
            source: None,
        }
        .is_retryable()
    );
    assert!(
        !PtyError::Spawn {
            reason: "not found".into(),
            kind: SpawnErrorKind::NotFound
        }
        .is_retryable()
    );
}

#[test]
fn test_pty_error_operation() {
    assert_eq!(
        PtyError::Open {
            reason: "x".into(),
            source: None,
        }
        .operation(),
        "open"
    );
    assert_eq!(
        PtyError::Spawn {
            reason: "x".into(),
            kind: SpawnErrorKind::Other
        }
        .operation(),
        "spawn"
    );
    assert_eq!(
        PtyError::Write {
            reason: "x".into(),
            source: None,
        }
        .operation(),
        "write"
    );
    assert_eq!(
        PtyError::Read {
            reason: "x".into(),
            source: None,
        }
        .operation(),
        "read"
    );
    assert_eq!(
        PtyError::Resize {
            reason: "x".into(),
            source: None,
        }
        .operation(),
        "resize"
    );
}

#[test]
fn test_pty_error_reason() {
    let err = PtyError::Open {
        reason: "allocation failed".into(),
        source: None,
    };
    assert_eq!(err.reason(), "allocation failed");
}
