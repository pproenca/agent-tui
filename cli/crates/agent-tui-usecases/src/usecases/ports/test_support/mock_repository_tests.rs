use super::*;

#[test]
fn test_mock_repository_resolve_returns_no_active_session_by_default() {
    let repo = MockSessionRepository::new();
    let result = repo.resolve(None);

    assert!(matches!(result, Err(SessionError::NoActiveSession)));
    assert_eq!(repo.resolve_call_count(), 1);
}

#[test]
fn test_mock_repository_resolve_with_configured_error() {
    let repo = MockSessionRepository::builder()
        .with_resolve_error(MockError::NotFound("custom".to_string()))
        .build();

    let session1 = SessionId::try_new("session1").expect("valid session id");
    let result = repo.resolve(Some(&session1));

    assert!(matches!(result, Err(SessionError::NotFound(id)) if id == "custom"));
}

#[test]
fn test_mock_repository_spawn_tracks_calls() {
    let repo = MockSessionRepository::builder()
        .with_spawn_result("test-session", 12345)
        .build();

    let result = repo.spawn("bash", &[], None, None, None, TerminalSize::default());

    assert!(result.is_ok());
    let (session_id, pid) = result.expect("spawn should succeed");
    assert_eq!(session_id.as_str(), "test-session");
    assert_eq!(pid, 12345);
    assert_eq!(repo.spawn_call_count(), 1);
}

#[test]
fn test_mock_repository_spawn_captures_params() {
    let repo = MockSessionRepository::builder()
        .with_spawn_result("test-session", 12345)
        .build();

    let args = vec!["--version".to_string()];
    let _ = repo.spawn(
        "bash",
        &args,
        Some("/tmp"),
        None,
        Some(SessionId::try_new("custom-id").expect("valid session id")),
        TerminalSize::try_new(120, 40).expect("valid terminal size"),
    );

    let params = repo.spawn_params();
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].command, "bash");
    assert_eq!(params[0].args, vec!["--version"]);
    assert_eq!(params[0].cwd, Some("/tmp".to_string()));
    assert_eq!(params[0].session_id, Some("custom-id".to_string()));
    assert_eq!(
        params[0].size,
        TerminalSize::try_new(120, 40).expect("valid terminal size")
    );
}

#[test]
fn test_mock_repository_kill_tracks_sessions() {
    let repo = MockSessionRepository::new();

    let s1 = SessionId::try_new("session1").expect("valid session id");
    let s2 = SessionId::try_new("session2").expect("valid session id");
    let _ = repo.kill(&s1);
    let _ = repo.kill(&s2);

    assert_eq!(repo.kill_call_count(), 2);
    assert_eq!(repo.killed_sessions(), vec!["session1", "session2"]);
}

#[test]
fn test_mock_repository_set_active_tracks_sessions() {
    let repo = MockSessionRepository::new();

    let s1 = SessionId::try_new("session1").expect("valid session id");
    let s2 = SessionId::try_new("session2").expect("valid session id");
    let _ = repo.set_active(&s1);
    let _ = repo.set_active(&s2);

    assert_eq!(repo.set_active_call_count(), 2);
    assert_eq!(repo.activated_sessions(), vec!["session1", "session2"]);
}

#[test]
fn test_builder_with_sessions_list() {
    use crate::domain::TerminalSize;
    let sessions = vec![SessionInfo {
        id: SessionId::try_new("sess1").expect("valid session id"),
        command: "bash".to_string(),
        pid: 1234,
        running: true,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        size: TerminalSize::default(),
    }];

    let repo = MockSessionRepository::builder()
        .with_sessions(sessions)
        .with_active_session("sess1")
        .build();

    assert_eq!(repo.list().len(), 1);
    assert_eq!(
        repo.active_session_id()
            .as_ref()
            .map(agent_tui_domain::SessionId::as_str),
        Some("sess1")
    );
}

#[test]
fn test_mock_error_conversion() {
    let err = MockError::NotFound("test".to_string());
    let session_err = err.to_session_error();
    assert!(matches!(session_err, SessionError::NotFound(id) if id == "test"));

    let err = MockError::LimitReached(10);
    let session_err = err.to_session_error();
    assert!(matches!(session_err, SessionError::LimitReached(10)));

    let err = MockError::NoActiveSession;
    let session_err = err.to_session_error();
    assert!(matches!(session_err, SessionError::NoActiveSession));
}
