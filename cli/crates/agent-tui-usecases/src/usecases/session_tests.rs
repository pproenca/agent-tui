use super::*;
use crate::domain::SessionId;
use crate::domain::SessionInfo;
use crate::domain::TerminalSize;
use crate::test_support::MockError;
use crate::test_support::MockSession;
use crate::test_support::MockSessionRepository;
use std::collections::HashMap;

#[test]
fn test_spawn_usecase_forwards_all_parameters_to_repository() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_spawn_result("new-session", 12345)
            .build(),
    );
    let usecase = SpawnUseCaseImpl::new(repo.clone());

    let mut env = HashMap::new();
    env.insert("FOO".to_string(), "bar".to_string());

    let input = SpawnInput {
        command: "bash".to_string(),
        args: vec!["-c".to_string(), "echo hello".to_string()],
        cwd: Some("/tmp".to_string()),
        env: Some(env.clone()),
        session_id: Some(SessionId::try_new("custom-id").expect("valid session id")),
        size: TerminalSize::try_new(120, 40).expect("valid terminal size"),
    };

    let result = usecase.execute(input);
    assert!(result.is_ok());

    let params = repo.spawn_params();
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].command, "bash");
    assert_eq!(params[0].args, vec!["-c", "echo hello"]);
    assert_eq!(params[0].cwd, Some("/tmp".to_string()));
    assert_eq!(params[0].env, Some(env));
    assert_eq!(params[0].session_id, Some("custom-id".to_string()));
    assert_eq!(
        params[0].size,
        TerminalSize::try_new(120, 40).expect("valid terminal size")
    );
}

#[test]
fn test_spawn_usecase_returns_session_id_and_pid() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_spawn_result("test-session-123", 54321)
            .build(),
    );
    let usecase = SpawnUseCaseImpl::new(repo);

    let input = SpawnInput {
        command: "vim".to_string(),
        args: vec![],
        cwd: None,
        env: None,
        session_id: None,
        size: TerminalSize::default(),
    };

    let result = usecase.execute(input).expect("spawn should succeed");
    assert_eq!(result.session_id.as_str(), "test-session-123");
    assert_eq!(result.pid, 54321);
}

#[test]
fn test_spawn_usecase_uses_default_cols_and_rows() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_spawn_result("session", 1000)
            .build(),
    );
    let usecase = SpawnUseCaseImpl::new(repo.clone());

    let input = SpawnInput {
        command: "cat".to_string(),
        args: vec![],
        cwd: None,
        env: None,
        session_id: None,
        size: TerminalSize::default(),
    };

    let _ = usecase.execute(input);

    let params = repo.spawn_params();
    assert_eq!(params[0].size, TerminalSize::default());
}

#[test]
fn test_spawn_usecase_propagates_limit_reached_error() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_spawn_error(MockError::LimitReached(16))
            .build(),
    );
    let usecase = SpawnUseCaseImpl::new(repo);

    let input = SpawnInput {
        command: "bash".to_string(),
        args: vec![],
        cwd: None,
        env: None,
        session_id: None,
        size: TerminalSize::default(),
    };

    let result = usecase.execute(input);
    assert!(matches!(
        result,
        Err(SpawnError::SessionLimitReached { max: 16 })
    ));
}

#[test]
fn test_spawn_usecase_custom_session_id_respected() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_spawn_result("my-custom-session", 1)
            .build(),
    );
    let usecase = SpawnUseCaseImpl::new(repo.clone());

    let input = SpawnInput {
        command: "bash".to_string(),
        args: vec![],
        cwd: None,
        env: None,
        session_id: Some(SessionId::try_new("my-custom-session").expect("valid session id")),
        size: TerminalSize::default(),
    };

    let result = usecase
        .execute(input)
        .expect("spawn with explicit session id should succeed");
    assert_eq!(result.session_id.as_str(), "my-custom-session");

    let params = repo.spawn_params();
    assert_eq!(params[0].session_id, Some("my-custom-session".to_string()));
}

#[test]
fn test_spawn_usecase_classifies_command_not_found_error() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_spawn_error(MockError::Terminal {
                kind: crate::usecases::ports::SpawnErrorKind::NotFound,
                reason: "No such file or directory".to_string(),
            })
            .build(),
    );
    let usecase = SpawnUseCaseImpl::new(repo);

    let input = SpawnInput {
        command: "nonexistent-command".to_string(),
        args: vec![],
        cwd: None,
        env: None,
        session_id: None,
        size: TerminalSize::default(),
    };

    let result = usecase.execute(input);
    assert!(matches!(
        result,
        Err(SpawnError::CommandNotFound { command }) if command == "nonexistent-command"
    ));
}

#[test]
fn test_spawn_usecase_classifies_not_found_variant_error() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_spawn_error(MockError::Terminal {
                kind: crate::usecases::ports::SpawnErrorKind::NotFound,
                reason: "command not found".to_string(),
            })
            .build(),
    );
    let usecase = SpawnUseCaseImpl::new(repo);

    let input = SpawnInput {
        command: "missing-cmd".to_string(),
        args: vec![],
        cwd: None,
        env: None,
        session_id: None,
        size: TerminalSize::default(),
    };

    let result = usecase.execute(input);
    assert!(matches!(
        result,
        Err(SpawnError::CommandNotFound { command }) if command == "missing-cmd"
    ));
}

#[test]
fn test_spawn_usecase_classifies_permission_denied_error() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_spawn_error(MockError::Terminal {
                kind: crate::usecases::ports::SpawnErrorKind::PermissionDenied,
                reason: "Permission denied".to_string(),
            })
            .build(),
    );
    let usecase = SpawnUseCaseImpl::new(repo);

    let input = SpawnInput {
        command: "/etc/shadow".to_string(),
        args: vec![],
        cwd: None,
        env: None,
        session_id: None,
        size: TerminalSize::default(),
    };

    let result = usecase.execute(input);
    assert!(matches!(
        result,
        Err(SpawnError::PermissionDenied { command }) if command == "/etc/shadow"
    ));
}

#[test]
fn test_spawn_usecase_classifies_generic_terminal_error() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_spawn_error(MockError::Terminal {
                kind: crate::usecases::ports::SpawnErrorKind::Other,
                reason: "unknown error occurred".to_string(),
            })
            .build(),
    );
    let usecase = SpawnUseCaseImpl::new(repo);

    let input = SpawnInput {
        command: "some-command".to_string(),
        args: vec![],
        cwd: None,
        env: None,
        session_id: None,
        size: TerminalSize::default(),
    };

    let result = usecase.execute(input);
    match result {
        Err(SpawnError::TerminalError { operation, reason }) => {
            assert_eq!(operation, "spawn");
            assert!(reason.contains("unknown error"));
        }
        _ => panic!("Expected TerminalError but got {result:?}"),
    }
}

#[test]
fn test_sessions_usecase_returns_empty_list_when_no_sessions() {
    let repo = Arc::new(MockSessionRepository::new());
    let usecase = SessionsUseCaseImpl::new(repo);

    let result = usecase.execute();
    assert!(result.sessions.is_empty());
    assert!(result.active_session.is_none());
}

#[test]
fn test_sessions_usecase_returns_configured_sessions() {
    let sessions = vec![
        SessionInfo {
            id: SessionId::try_new("session1").expect("valid session id"),
            command: "bash".to_string(),
            pid: 1001,
            running: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            size: TerminalSize::default(),
        },
        SessionInfo {
            id: SessionId::try_new("session2").expect("valid session id"),
            command: "vim".to_string(),
            pid: 1002,
            running: true,
            created_at: "2024-01-01T01:00:00Z".to_string(),
            size: TerminalSize::try_new(120, 40).expect("valid terminal size"),
        },
    ];

    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_sessions(sessions)
            .with_active_session("session1")
            .build(),
    );
    let usecase = SessionsUseCaseImpl::new(repo);

    let result = usecase.execute();
    assert_eq!(result.sessions.len(), 2);
    assert_eq!(result.sessions[0].id.as_str(), "session1");
    assert_eq!(result.sessions[0].command, "bash");
    assert_eq!(result.sessions[1].id.as_str(), "session2");
    assert_eq!(result.sessions[1].command, "vim");
    assert_eq!(
        result
            .active_session
            .as_ref()
            .map(agent_tui_domain::SessionId::as_str),
        Some("session1")
    );
}

#[test]
fn test_sessions_usecase_returns_active_session_none_when_not_set() {
    let sessions = vec![SessionInfo {
        id: SessionId::try_new("orphan").expect("valid session id"),
        command: "sleep".to_string(),
        pid: 999,
        running: true,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        size: TerminalSize::default(),
    }];

    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_sessions(sessions)
            .build(),
    );
    let usecase = SessionsUseCaseImpl::new(repo);

    let result = usecase.execute();
    assert_eq!(result.sessions.len(), 1);
    assert!(result.active_session.is_none());
}

#[test]
fn test_kill_usecase_returns_error_when_no_active_session() {
    let repo = Arc::new(MockSessionRepository::new());
    let usecase = KillUseCaseImpl::new(repo);

    let input = SessionInput { session_id: None };
    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NoActiveSession)));
}

#[test]
fn test_kill_usecase_returns_error_when_session_not_found() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_resolve_error(MockError::NotFound("nonexistent".to_string()))
            .build(),
    );
    let usecase = KillUseCaseImpl::new(repo);

    let input = SessionInput {
        session_id: Some(SessionId::try_new("nonexistent").expect("valid session id")),
    };
    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NotFound(_))));
}

#[test]
fn test_restart_usecase_returns_error_when_no_active_session() {
    let repo = Arc::new(MockSessionRepository::new());
    let usecase = RestartUseCaseImpl::new(repo);

    let input = SessionInput { session_id: None };
    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NoActiveSession)));
}

#[test]
fn test_restart_usecase_returns_error_when_session_not_found() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_resolve_error(MockError::NotFound("missing".to_string()))
            .build(),
    );
    let usecase = RestartUseCaseImpl::new(repo);

    let input = SessionInput {
        session_id: Some(SessionId::try_new("missing").expect("valid session id")),
    };
    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NotFound(id)) if id == "missing"));
}

#[test]
fn test_restart_usecase_returns_repository_restart_output() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_restart_result("session-a", "session-b", "bash", 4242)
            .build(),
    );
    let usecase = RestartUseCaseImpl::new(Arc::clone(&repo));
    let input = SessionInput {
        session_id: Some(SessionId::try_new("session-a").expect("valid session id")),
    };

    let result = usecase.execute(input).expect("restart should succeed");

    assert_eq!(repo.restart_call_count(), 1);
    assert_eq!(result.old_session_id.as_str(), "session-a");
    assert_eq!(result.new_session_id.as_str(), "session-b");
    assert_eq!(result.command, "bash");
    assert_eq!(result.pid, 4242);
}

#[test]
fn test_attach_usecase_returns_error_when_session_not_found() {
    let repo = Arc::new(MockSessionRepository::new());
    let usecase = AttachUseCaseImpl::new(repo);

    let input = AttachInput {
        session_id: SessionId::try_new("nonexistent").expect("valid session id"),
    };
    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NotFound(_))));
}

#[test]
fn test_attach_usecase_returns_error_with_configured_error() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_resolve_error(MockError::NotFound("target-session".to_string()))
            .build(),
    );
    let usecase = AttachUseCaseImpl::new(repo);

    let input = AttachInput {
        session_id: SessionId::try_new("target-session").expect("valid session id"),
    };
    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NotFound(id)) if id == "target-session"));
}

#[test]
fn test_attach_usecase_returns_error_when_session_is_not_running() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_session_handle(Arc::new(
                MockSession::builder("target-session")
                    .with_running(false)
                    .build(),
            ))
            .build(),
    );
    let usecase = AttachUseCaseImpl::new(repo);

    let input = AttachInput {
        session_id: SessionId::try_new("target-session").expect("valid session id"),
    };
    let result = usecase.execute(input);
    assert!(matches!(
        result,
        Err(SessionError::NotRunning { session_id }) if session_id == "target-session"
    ));
}

#[test]
fn test_resize_usecase_returns_error_when_no_active_session() {
    let repo = Arc::new(MockSessionRepository::new());
    let usecase = ResizeUseCaseImpl::new(repo);

    let input = ResizeInput {
        session_id: None,
        size: TerminalSize::try_new(120, 40).expect("valid terminal size"),
    };

    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NoActiveSession)));
}

#[test]
fn test_resize_usecase_returns_error_when_session_not_found() {
    let repo = Arc::new(
        MockSessionRepository::builder()
            .with_resolve_error(MockError::NotFound("unknown".to_string()))
            .build(),
    );
    let usecase = ResizeUseCaseImpl::new(repo);

    let input = ResizeInput {
        session_id: Some(SessionId::try_new("unknown").expect("valid session id")),
        size: TerminalSize::default(),
    };

    let result = usecase.execute(input);
    assert!(matches!(result, Err(SessionError::NotFound(_))));
}
