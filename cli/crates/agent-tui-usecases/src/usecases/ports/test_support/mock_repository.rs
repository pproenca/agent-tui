//! Mock session repository for use case tests.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use super::mock_error::MockError;
use crate::common::mutex_lock_or_recover;
use crate::domain::RestartOutput;
use crate::domain::SessionId;
use crate::domain::SessionInfo;
use crate::domain::TerminalSize;
use crate::usecases::ports::SessionError;
use crate::usecases::ports::SessionHandle;
use crate::usecases::ports::SessionRepository;

#[derive(Default)]
pub struct MockSessionRepository {
    resolve_error: Option<MockError>,
    spawn_error: Option<MockError>,
    kill_error: Option<MockError>,
    get_error: Option<MockError>,
    set_active_error: Option<MockError>,
    restart_error: Option<MockError>,
    sessions_list: Vec<SessionInfo>,
    active_id: Option<SessionId>,
    session_count: usize,
    spawn_result: Option<(SessionId, u32)>,
    restart_result: Option<RestartOutput>,
    session_handle: Option<SessionHandle>,

    spawn_calls: AtomicUsize,
    resolve_calls: AtomicUsize,
    kill_calls: AtomicUsize,
    get_calls: AtomicUsize,
    set_active_calls: AtomicUsize,
    restart_calls: AtomicUsize,
    killed_sessions: Mutex<Vec<String>>,
    activated_sessions: Mutex<Vec<String>>,
    spawn_params: Mutex<Vec<SpawnParams>>,
}

#[derive(Debug, Clone)]
pub struct SpawnParams {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub session_id: Option<String>,
    pub size: TerminalSize,
}

impl MockSessionRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> MockSessionRepositoryBuilder {
        MockSessionRepositoryBuilder::new()
    }

    pub fn spawn_call_count(&self) -> usize {
        self.spawn_calls.load(Ordering::SeqCst)
    }

    pub fn resolve_call_count(&self) -> usize {
        self.resolve_calls.load(Ordering::SeqCst)
    }

    pub fn kill_call_count(&self) -> usize {
        self.kill_calls.load(Ordering::SeqCst)
    }

    pub fn set_active_call_count(&self) -> usize {
        self.set_active_calls.load(Ordering::SeqCst)
    }

    pub fn restart_call_count(&self) -> usize {
        self.restart_calls.load(Ordering::SeqCst)
    }

    pub fn killed_sessions(&self) -> Vec<String> {
        mutex_lock_or_recover(&self.killed_sessions).clone()
    }

    pub fn activated_sessions(&self) -> Vec<String> {
        mutex_lock_or_recover(&self.activated_sessions).clone()
    }

    pub fn spawn_params(&self) -> Vec<SpawnParams> {
        mutex_lock_or_recover(&self.spawn_params).clone()
    }
}

impl SessionRepository for MockSessionRepository {
    fn spawn(
        &self,
        command: &str,
        args: &[String],
        cwd: Option<&str>,
        env: Option<&HashMap<String, String>>,
        session_id: Option<SessionId>,
        size: TerminalSize,
    ) -> Result<(SessionId, u32), SessionError> {
        self.spawn_calls.fetch_add(1, Ordering::SeqCst);

        mutex_lock_or_recover(&self.spawn_params).push(SpawnParams {
            command: command.to_string(),
            args: args.to_vec(),
            cwd: cwd.map(std::string::ToString::to_string),
            env: env.cloned(),
            session_id: session_id.map(|id| id.to_string()),
            size,
        });

        if let Some(ref err) = self.spawn_error {
            return Err(err.to_session_error());
        }

        if let Some(ref result) = self.spawn_result {
            return Ok(result.clone());
        }

        Err(SessionError::LimitReached(0))
    }

    fn get(&self, session_id: &SessionId) -> Result<SessionHandle, SessionError> {
        self.get_calls.fetch_add(1, Ordering::SeqCst);

        if let Some(ref err) = self.get_error {
            return Err(err.to_session_error());
        }

        self.session_handle
            .clone()
            .ok_or_else(|| SessionError::NotFound(session_id.as_str().to_string()))
    }

    fn active(&self) -> Result<SessionHandle, SessionError> {
        if let Some(ref err) = self.resolve_error {
            return Err(err.to_session_error());
        }
        self.session_handle
            .clone()
            .ok_or(SessionError::NoActiveSession)
    }

    fn resolve(&self, session_id: Option<&SessionId>) -> Result<SessionHandle, SessionError> {
        self.resolve_calls.fetch_add(1, Ordering::SeqCst);

        if let Some(ref err) = self.resolve_error {
            return Err(err.to_session_error());
        }

        match session_id {
            Some(id) => self
                .session_handle
                .clone()
                .ok_or_else(|| SessionError::NotFound(id.as_str().to_string())),
            None => Err(SessionError::NoActiveSession),
        }
    }

    fn set_active(&self, session_id: &SessionId) -> Result<(), SessionError> {
        self.set_active_calls.fetch_add(1, Ordering::SeqCst);
        mutex_lock_or_recover(&self.activated_sessions).push(session_id.as_str().to_string());

        if let Some(ref err) = self.set_active_error {
            return Err(err.to_session_error());
        }

        Ok(())
    }

    fn list(&self) -> Vec<SessionInfo> {
        self.sessions_list.clone()
    }

    fn kill(&self, session_id: &SessionId) -> Result<(), SessionError> {
        self.kill_calls.fetch_add(1, Ordering::SeqCst);
        mutex_lock_or_recover(&self.killed_sessions).push(session_id.as_str().to_string());

        if let Some(ref err) = self.kill_error {
            return Err(err.to_session_error());
        }

        Ok(())
    }

    fn restart(&self, session_id: Option<&SessionId>) -> Result<RestartOutput, SessionError> {
        self.restart_calls.fetch_add(1, Ordering::SeqCst);

        if let Some(ref err) = self.restart_error {
            return Err(err.to_session_error());
        }

        if let Some(ref err) = self.resolve_error {
            return Err(err.to_session_error());
        }

        if let Some(ref result) = self.restart_result {
            return Ok(result.clone());
        }

        let old_session_id = match session_id.cloned().or_else(|| self.active_id.clone()) {
            Some(id) => id,
            None => return Err(SessionError::NoActiveSession),
        };

        Ok(RestartOutput {
            old_session_id,
            new_session_id: SessionId::new_unchecked("restarted-session"),
            command: "bash".to_string(),
            pid: 42,
        })
    }

    fn session_count(&self) -> usize {
        self.session_count
    }

    fn active_session_id(&self) -> Option<SessionId> {
        self.active_id.clone()
    }
}

#[derive(Default)]
pub struct MockSessionRepositoryBuilder {
    repo: MockSessionRepository,
}

impl MockSessionRepositoryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_resolve_error(mut self, error: MockError) -> Self {
        self.repo.resolve_error = Some(error);
        self
    }

    pub fn with_spawn_error(mut self, error: MockError) -> Self {
        self.repo.spawn_error = Some(error);
        self
    }

    pub fn with_spawn_result(mut self, session_id: impl Into<String>, pid: u32) -> Self {
        self.repo.spawn_result = Some((
            SessionId::try_new(session_id.into()).expect("builder session id should be valid"),
            pid,
        ));
        self
    }

    pub fn with_restart_result(
        mut self,
        old_session_id: impl Into<String>,
        new_session_id: impl Into<String>,
        command: impl Into<String>,
        pid: u32,
    ) -> Self {
        self.repo.restart_result = Some(RestartOutput {
            old_session_id: SessionId::try_new(old_session_id.into())
                .expect("builder old session id should be valid"),
            new_session_id: SessionId::try_new(new_session_id.into())
                .expect("builder new session id should be valid"),
            command: command.into(),
            pid,
        });
        self
    }

    pub fn with_restart_error(mut self, error: MockError) -> Self {
        self.repo.restart_error = Some(error);
        self
    }

    pub fn with_sessions(mut self, sessions: Vec<SessionInfo>) -> Self {
        self.repo.sessions_list = sessions;
        self
    }

    pub fn with_active_session(mut self, session_id: impl Into<String>) -> Self {
        self.repo.active_id = Some(
            SessionId::try_new(session_id.into()).expect("builder session id should be valid"),
        );
        self
    }

    pub fn with_session_count(mut self, count: usize) -> Self {
        self.repo.session_count = count;
        self
    }

    pub fn with_session_handle(mut self, session_handle: SessionHandle) -> Self {
        self.repo.session_handle = Some(session_handle);
        self
    }

    pub fn build(self) -> MockSessionRepository {
        self.repo
    }
}

#[cfg(test)]
mod tests {
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
            .with_session_count(1)
            .build();

        assert_eq!(repo.list().len(), 1);
        assert_eq!(repo.session_count(), 1);
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
}
