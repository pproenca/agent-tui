use std::sync::Arc;

use crate::domain::{
    KillOutput, ResizeInput, ResizeOutput, SessionsOutput, SpawnInput, SpawnOutput,
};
use crate::error::SessionError;
use crate::repository::SessionRepository;
use crate::session::SessionId;

pub trait SpawnUseCase: Send + Sync {
    fn execute(&self, input: SpawnInput) -> Result<SpawnOutput, SessionError>;
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
    fn execute(&self, input: SpawnInput) -> Result<SpawnOutput, SessionError> {
        let (session_id, pid) = self.repository.spawn(
            &input.command,
            &input.args,
            input.cwd.as_deref(),
            input.env.as_ref(),
            input.session_id,
            input.cols,
            input.rows,
        )?;

        Ok(SpawnOutput { session_id, pid })
    }
}

pub trait KillUseCase: Send + Sync {
    fn execute(&self, session_id: Option<String>) -> Result<KillOutput, SessionError>;
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
    fn execute(&self, session_id: Option<String>) -> Result<KillOutput, SessionError> {
        let session = self.repository.resolve(session_id.as_deref())?;
        let id = {
            let guard = session.lock().unwrap();
            SessionId::from(guard.id.as_str())
        };

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
    fn execute(&self, session_id: Option<String>) -> Result<RestartOutput, SessionError>;
}

#[derive(Debug, Clone)]
pub struct RestartOutput {
    pub old_session_id: SessionId,
    pub new_session_id: SessionId,
    pub command: String,
    pub pid: u32,
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
    fn execute(&self, session_id: Option<String>) -> Result<RestartOutput, SessionError> {
        let session = self.repository.resolve(session_id.as_deref())?;

        let (old_id, command, cols, rows) = {
            let guard = session.lock().unwrap();
            let (c, r) = guard.size();
            (
                SessionId::from(guard.id.as_str()),
                guard.command.clone(),
                c,
                r,
            )
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
    fn execute(&self, session_id: &str) -> Result<AttachOutput, SessionError>;
}

#[derive(Debug, Clone)]
pub struct AttachOutput {
    pub session_id: SessionId,
    pub success: bool,
    pub message: String,
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
    fn execute(&self, session_id: &str) -> Result<AttachOutput, SessionError> {
        let session = self.repository.resolve(Some(session_id))?;

        let is_running = {
            let mut guard = session.lock().unwrap();
            guard.is_running()
        };

        if !is_running {
            return Err(SessionError::NotFound(format!(
                "{} (session not running)",
                session_id
            )));
        }

        self.repository.set_active(session_id)?;

        Ok(AttachOutput {
            session_id: SessionId::from(session_id),
            success: true,
            message: format!("Now attached to session {}", session_id),
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
    fn execute(&self, input: ResizeInput) -> Result<ResizeOutput, SessionError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        let mut guard = session.lock().unwrap();

        guard.resize(input.cols, input.rows)?;

        Ok(ResizeOutput {
            session_id: SessionId::from(guard.id.as_str()),
            success: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionInfo;
    use crate::test_support::{MockError, MockSessionRepository};
    use std::collections::HashMap;

    // ========================================================================
    // SpawnUseCase Tests
    // ========================================================================

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
            session_id: Some("custom-id".to_string()),
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
        assert!(matches!(result, Err(SessionError::LimitReached(16))));
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
            session_id: Some("my-custom-session".to_string()),
            cols: 80,
            rows: 24,
        };

        let result = usecase.execute(input).unwrap();
        assert_eq!(result.session_id.as_str(), "my-custom-session");

        let params = repo.spawn_params();
        assert_eq!(params[0].session_id, Some("my-custom-session".to_string()));
    }

    // ========================================================================
    // SessionsUseCase Tests
    // ========================================================================

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

    // ========================================================================
    // KillUseCase Tests (Error paths only - happy path needs real Session)
    // ========================================================================

    #[test]
    fn test_kill_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = KillUseCaseImpl::new(repo);

        let result = usecase.execute(None);
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

        let result = usecase.execute(Some("nonexistent".to_string()));
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }

    // ========================================================================
    // RestartUseCase Tests (Error paths only - happy path needs real Session)
    // ========================================================================

    #[test]
    fn test_restart_usecase_returns_error_when_no_active_session() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = RestartUseCaseImpl::new(repo);

        let result = usecase.execute(None);
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

        let result = usecase.execute(Some("missing".to_string()));
        assert!(matches!(result, Err(SessionError::NotFound(id)) if id == "missing"));
    }

    // ========================================================================
    // AttachUseCase Tests (Error paths only - happy path needs real Session)
    // ========================================================================

    #[test]
    fn test_attach_usecase_returns_error_when_session_not_found() {
        let repo = Arc::new(MockSessionRepository::new());
        let usecase = AttachUseCaseImpl::new(repo);

        let result = usecase.execute("nonexistent");
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

        let result = usecase.execute("target-session");
        assert!(matches!(result, Err(SessionError::NotFound(id)) if id == "target-session"));
    }

    // ========================================================================
    // ResizeUseCase Tests (Error paths only - happy path needs real Session)
    // ========================================================================

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
            session_id: Some("unknown".to_string()),
            cols: 80,
            rows: 24,
        };

        let result = usecase.execute(input);
        assert!(matches!(result, Err(SessionError::NotFound(_))));
    }
}
