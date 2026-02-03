use std::sync::Arc;

use crate::domain::{
    AttachInput, AttachOutput, CleanupFailure, CleanupInput, CleanupOutput, KillOutput,
    ResizeInput, ResizeOutput, RestartOutput, SessionInput, SessionsOutput, SpawnInput,
    SpawnOutput,
};
use crate::usecases::SpawnError;
use crate::usecases::ports::{PtyError, SessionError, SessionRepository, SpawnErrorKind};

pub trait SpawnUseCase: Send + Sync {
    fn execute(&self, input: SpawnInput) -> Result<SpawnOutput, SpawnError>;
}

pub struct SpawnUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> SpawnUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> SpawnUseCase for SpawnUseCaseImpl<R> {
    #[tracing::instrument(
        skip(self, input),
        fields(
            session = ?input.session_id,
            command = %input.command,
            args_len = input.args.len(),
            cwd = ?input.cwd,
            cols = input.cols,
            rows = input.rows
        )
    )]
    fn execute(&self, input: SpawnInput) -> Result<SpawnOutput, SpawnError> {
        let session_id_str = input.session_id.map(|id| id.to_string());
        let command = input.command.clone();

        match self.repository.spawn(
            &input.command,
            &input.args,
            input.cwd.as_deref(),
            input.env.as_ref(),
            session_id_str,
            input.cols,
            input.rows,
        ) {
            Ok((session_id, pid)) => Ok(SpawnOutput { session_id, pid }),
            Err(SessionError::LimitReached(max)) => Err(SpawnError::SessionLimitReached { max }),
            Err(SessionError::AlreadyExists(session_id)) => {
                Err(SpawnError::SessionAlreadyExists { session_id })
            }
            Err(SessionError::Pty(PtyError::Spawn { kind, reason })) => match kind {
                SpawnErrorKind::NotFound => Err(SpawnError::CommandNotFound { command }),
                SpawnErrorKind::PermissionDenied => Err(SpawnError::PermissionDenied { command }),
                SpawnErrorKind::Other => Err(SpawnError::PtyError {
                    operation: "spawn".to_string(),
                    reason,
                }),
            },
            Err(SessionError::Pty(pty_err)) => Err(SpawnError::PtyError {
                operation: pty_err.operation().to_string(),
                reason: pty_err.reason().to_string(),
            }),
            Err(e) => Err(SpawnError::PtyError {
                operation: "spawn".to_string(),
                reason: e.to_string(),
            }),
        }
    }
}

pub trait KillUseCase: Send + Sync {
    fn execute(&self, input: SessionInput) -> Result<KillOutput, SessionError>;
}

pub struct KillUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> KillUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> KillUseCase for KillUseCaseImpl<R> {
    #[tracing::instrument(skip(self, input), fields(session = ?input.session_id))]
    fn execute(&self, input: SessionInput) -> Result<KillOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let id = session.session_id();

        self.repository.kill(id.as_str())?;

        Ok(KillOutput {
            session_id: id,
            success: true,
        })
    }
}

pub trait SessionsUseCase: Send + Sync {
    fn execute(&self) -> SessionsOutput;
}

pub struct SessionsUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> SessionsUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> SessionsUseCase for SessionsUseCaseImpl<R> {
    #[tracing::instrument(skip(self))]
    fn execute(&self) -> SessionsOutput {
        let sessions = self.repository.list();
        let active_session = self.repository.active_session_id();

        SessionsOutput {
            sessions,
            active_session,
        }
    }
}

pub trait RestartUseCase: Send + Sync {
    fn execute(&self, input: SessionInput) -> Result<RestartOutput, SessionError>;
}

pub struct RestartUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> RestartUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> RestartUseCase for RestartUseCaseImpl<R> {
    #[tracing::instrument(skip(self, input), fields(session = ?input.session_id))]
    fn execute(&self, input: SessionInput) -> Result<RestartOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;

        let (old_id, command, cols, rows) = {
            let (c, r) = session.size();
            (session.session_id(), session.command(), c, r)
        };

        self.repository.kill(old_id.as_str())?;

        let (new_session_id, pid) =
            self.repository
                .spawn(&command, &[], None, None, None, cols, rows)?;

        Ok(RestartOutput {
            old_session_id: old_id,
            new_session_id,
            command,
            pid,
        })
    }
}

pub trait AttachUseCase: Send + Sync {
    fn execute(&self, input: AttachInput) -> Result<AttachOutput, SessionError>;
}

pub struct AttachUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> AttachUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> AttachUseCase for AttachUseCaseImpl<R> {
    #[tracing::instrument(skip(self, input), fields(session = %input.session_id))]
    fn execute(&self, input: AttachInput) -> Result<AttachOutput, SessionError> {
        let session_id_str = input.session_id.to_string();
        let session = self.repository.resolve(Some(&session_id_str))?;

        let is_running = session.is_running();

        if !is_running {
            return Err(SessionError::NotFound(format!(
                "{} (session not running)",
                session_id_str
            )));
        }

        self.repository.set_active(&session_id_str)?;

        Ok(AttachOutput {
            session_id: input.session_id,
            success: true,
            message: format!("Now attached to session {}", session_id_str),
        })
    }
}

pub trait ResizeUseCase: Send + Sync {
    fn execute(&self, input: ResizeInput) -> Result<ResizeOutput, SessionError>;
}

pub struct ResizeUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> ResizeUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> ResizeUseCase for ResizeUseCaseImpl<R> {
    #[tracing::instrument(
        skip(self, input),
        fields(session = ?input.session_id, cols = input.cols, rows = input.rows)
    )]
    fn execute(&self, input: ResizeInput) -> Result<ResizeOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        session.resize(input.cols, input.rows)?;

        Ok(ResizeOutput {
            session_id: session.session_id(),
            success: true,
            cols: input.cols,
            rows: input.rows,
        })
    }
}

pub trait CleanupUseCase: Send + Sync {
    fn execute(&self, input: CleanupInput) -> CleanupOutput;
}

pub struct CleanupUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> CleanupUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> CleanupUseCase for CleanupUseCaseImpl<R> {
    #[tracing::instrument(skip(self, input), fields(all = input.all))]
    fn execute(&self, input: CleanupInput) -> CleanupOutput {
        let sessions = self.repository.list();
        let mut cleaned = 0;
        let mut failures = Vec::new();

        for info in sessions {
            let should_cleanup = input.all || !info.is_active();

            if should_cleanup {
                match self.repository.kill(info.id.as_str()) {
                    Ok(_) => cleaned += 1,
                    Err(e) => failures.push(CleanupFailure {
                        session_id: info.id.clone(),
                        error: e.to_string(),
                    }),
                }
            }
        }

        CleanupOutput { cleaned, failures }
    }
}

use crate::domain::{AssertConditionType, AssertInput, AssertOutput};

pub trait AssertUseCase: Send + Sync {
    fn execute(&self, input: AssertInput) -> Result<AssertOutput, SessionError>;
}

pub struct AssertUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
}

impl<R: SessionRepository> AssertUseCaseImpl<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R: SessionRepository> AssertUseCase for AssertUseCaseImpl<R> {
    #[tracing::instrument(
        skip(self, input),
        fields(
            session = ?input.session_id,
            condition = %input.condition_type.as_str(),
            value_len = input.value.len()
        )
    )]
    fn execute(&self, input: AssertInput) -> Result<AssertOutput, SessionError> {
        let condition = format!("{}:{}", input.condition_type.as_str(), input.value);

        let passed = match input.condition_type {
            AssertConditionType::Text => {
                let session = self.repository.resolve(input.session_id.as_deref())?;
                session.update()?;
                let screen = session.screen_text();
                screen.contains(&input.value)
            }
            AssertConditionType::Session => {
                let sessions = self.repository.list();
                sessions
                    .iter()
                    .any(|s| s.id.as_str() == input.value && s.is_active())
            }
        };

        Ok(AssertOutput { passed, condition })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SessionId;
    use crate::domain::SessionInfo;
    use crate::usecases::ports::test_support::{MockError, MockSessionRepository};
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
            session_id: Some(SessionId::new("custom-id")),
            cols: 120,
            rows: 40,
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
        assert_eq!(params[0].cols, 120);
        assert_eq!(params[0].rows, 40);
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
            cols: 80,
            rows: 24,
        };

        let result = usecase.execute(input).unwrap();
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
            cols: 80,
            rows: 24,
        };

        let _ = usecase.execute(input);

        let params = repo.spawn_params();
        assert_eq!(params[0].cols, 80);
        assert_eq!(params[0].rows, 24);
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
            cols: 80,
            rows: 24,
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
            session_id: Some(SessionId::new("my-custom-session")),
            cols: 80,
            rows: 24,
        };

        let result = usecase.execute(input).unwrap();
        assert_eq!(result.session_id.as_str(), "my-custom-session");

        let params = repo.spawn_params();
        assert_eq!(params[0].session_id, Some("my-custom-session".to_string()));
    }

    #[test]
    fn test_spawn_usecase_classifies_command_not_found_error() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_spawn_error(MockError::Pty {
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
            cols: 80,
            rows: 24,
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
                .with_spawn_error(MockError::Pty {
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
            cols: 80,
            rows: 24,
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
                .with_spawn_error(MockError::Pty {
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
            cols: 80,
            rows: 24,
        };

        let result = usecase.execute(input);
        assert!(matches!(
            result,
            Err(SpawnError::PermissionDenied { command }) if command == "/etc/shadow"
        ));
    }

    #[test]
    fn test_spawn_usecase_classifies_generic_pty_error() {
        let repo = Arc::new(
            MockSessionRepository::builder()
                .with_spawn_error(MockError::Pty {
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
            cols: 80,
            rows: 24,
        };

        let result = usecase.execute(input);
        match result {
            Err(SpawnError::PtyError { operation, reason }) => {
                assert_eq!(operation, "spawn");
                assert!(reason.contains("unknown error"));
            }
            _ => panic!("Expected PtyError but got {:?}", result),
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
                id: SessionId::new("session1"),
                command: "bash".to_string(),
                pid: 1001,
                running: true,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                size: (80, 24),
            },
            SessionInfo {
                id: SessionId::new("session2"),
                command: "vim".to_string(),
                pid: 1002,
                running: true,
                created_at: "2024-01-01T01:00:00Z".to_string(),
                size: (120, 40),
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
        assert_eq!(result.active_session.unwrap().as_str(), "session1");
    }

    #[test]
    fn test_sessions_usecase_returns_active_session_none_when_not_set() {
        let sessions = vec![SessionInfo {
            id: SessionId::new("orphan"),
            command: "sleep".to_string(),
            pid: 999,
            running: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            size: (80, 24),
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
            session_id: Some(SessionId::new("nonexistent")),
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
            session_id: Some(SessionId::new("missing")),
        };
        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(id)) if id == "missing"));
    }

    #[test]
    fn test_attach_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = AttachUseCaseImpl::new(repo);

        let input = AttachInput {
            session_id: SessionId::new("nonexistent"),
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
            session_id: SessionId::new("target-session"),
        };
        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(id)) if id == "target-session"));
    }

    #[test]
    fn test_resize_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = ResizeUseCaseImpl::new(repo);

        let input = ResizeInput {
            session_id: None,
            cols: 120,
            rows: 40,
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
            session_id: Some(SessionId::new("unknown")),
            cols: 80,
            rows: 24,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }
}
