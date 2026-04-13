use super::*;
use crate::test_support::MockSession;
use crate::test_support::MockSessionRepository;
use insta::assert_debug_snapshot;
use std::sync::Arc;

#[test]
fn test_snapshot_usecase_returns_error_when_no_session() {
    let repository = Arc::new(MockSessionRepository::new());
    let usecase = SnapshotUseCaseImpl::new(repository);

    let input = SnapshotInput::default();
    let result = usecase.execute(input);

    assert!(result.is_err());
}

#[test]
fn test_snapshot_usecase_rejects_named_region() {
    let session = Arc::new(MockSession::new("test-session"));
    let repository = Arc::new(
        MockSessionRepository::builder()
            .with_session_handle(session)
            .build(),
    );
    let usecase = SnapshotUseCaseImpl::new(repository);

    let input = SnapshotInput {
        session_id: Some(
            crate::domain::SessionId::try_new("test-session").expect("session id should be valid"),
        ),
        region: Some("modal".to_string()),
        ..SnapshotInput::default()
    };
    let result = usecase.execute(input);

    assert!(matches!(
        result,
        Err(SessionError::InvalidInput { field, .. }) if field == "region"
    ));
}

#[test]
fn test_snapshot_usecase_propagates_update_error() {
    let session = Arc::new(
        MockSession::builder("test-session")
            .with_update_error(SessionError::NoActiveSession)
            .build(),
    );
    let repository = Arc::new(
        MockSessionRepository::builder()
            .with_session_handle(session)
            .build(),
    );
    let usecase = SnapshotUseCaseImpl::new(repository);

    let result = usecase.execute(SnapshotInput {
        session_id: Some(
            crate::domain::SessionId::try_new("test-session").expect("session id should be valid"),
        ),
        ..SnapshotInput::default()
    });

    assert!(matches!(result, Err(SessionError::Terminal(_))));
}

#[test]
fn test_snapshot_usecase_returns_rendered_fields_and_cursor() {
    let session = Arc::new(
        MockSession::builder("test-session")
            .with_screen_text("plain text\n")
            .with_rendered_screen("\u{001b}[31mplain text\u{001b}[0m\r\n", "plain text\n")
            .build(),
    );
    let repository = Arc::new(
        MockSessionRepository::builder()
            .with_session_handle(session)
            .build(),
    );
    let usecase = SnapshotUseCaseImpl::new(repository);

    let output = usecase
        .execute(SnapshotInput {
            session_id: Some(
                crate::domain::SessionId::try_new("test-session")
                    .expect("session id should be valid"),
            ),
            include_cursor: true,
            include_render: true,
            ..SnapshotInput::default()
        })
        .expect("snapshot should succeed");

    assert_eq!(output.screenshot, "plain text\n");
    assert_eq!(
        output.rendered.as_deref(),
        Some("\u{001b}[31mplain text\u{001b}[0m\r\n")
    );
    assert_eq!(output.compact_rendered.as_deref(), Some("plain text\n"));
    assert_eq!(output.cursor.map(|cursor| cursor.visible), Some(false));
    assert_debug_snapshot!("snapshot_usecase_rendered_output", output);
}
