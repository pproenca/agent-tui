use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::terminal::PtyError;

use crate::daemon::error::SessionError;
use crate::daemon::repository::SessionRepository;
use crate::daemon::session::{Session, SessionId, SessionInfo};

#[derive(Debug, Clone, Default)]
pub enum MockError {
    #[default]
    NoActiveSession,
    NotFound(String),
    LimitReached(usize),
    ElementNotFound(String),
    WrongElementType {
        element_ref: String,
        expected: String,
        actual: String,
    },
    InvalidKey(String),
    Pty(String),
}

impl MockError {
    fn to_session_error(&self) -> SessionError {
        match self {
            MockError::NoActiveSession => SessionError::NoActiveSession,
            MockError::NotFound(id) => SessionError::NotFound(id.clone()),
            MockError::LimitReached(max) => SessionError::LimitReached(*max),
            MockError::ElementNotFound(el) => SessionError::ElementNotFound(el.clone()),
            MockError::WrongElementType {
                element_ref,
                expected,
                actual,
            } => SessionError::WrongElementType {
                element_ref: element_ref.clone(),
                expected: expected.clone(),
                actual: actual.clone(),
            },
            MockError::InvalidKey(key) => SessionError::InvalidKey(key.clone()),
            MockError::Pty(message) => SessionError::Pty(PtyError::Spawn(message.clone())),
        }
    }
}

#[derive(Default)]
pub struct MockSessionRepository {
    resolve_error: Option<MockError>,
    spawn_error: Option<MockError>,
    kill_error: Option<MockError>,
    get_error: Option<MockError>,
    set_active_error: Option<MockError>,
    sessions_list: Vec<SessionInfo>,
    active_id: Option<SessionId>,
    session_count: usize,
    spawn_result: Option<(SessionId, u32)>,

    spawn_calls: AtomicUsize,
    resolve_calls: AtomicUsize,
    kill_calls: AtomicUsize,
    get_calls: AtomicUsize,
    set_active_calls: AtomicUsize,
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
    pub cols: u16,
    pub rows: u16,
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

    pub fn get_call_count(&self) -> usize {
        self.get_calls.load(Ordering::SeqCst)
    }

    pub fn set_active_call_count(&self) -> usize {
        self.set_active_calls.load(Ordering::SeqCst)
    }

    pub fn killed_sessions(&self) -> Vec<String> {
        self.killed_sessions.lock().unwrap().clone()
    }

    pub fn activated_sessions(&self) -> Vec<String> {
        self.activated_sessions.lock().unwrap().clone()
    }

    pub fn spawn_params(&self) -> Vec<SpawnParams> {
        self.spawn_params.lock().unwrap().clone()
    }
}

impl SessionRepository for MockSessionRepository {
    fn spawn(
        &self,
        command: &str,
        args: &[String],
        cwd: Option<&str>,
        env: Option<&HashMap<String, String>>,
        session_id: Option<String>,
        cols: u16,
        rows: u16,
    ) -> Result<(SessionId, u32), SessionError> {
        self.spawn_calls.fetch_add(1, Ordering::SeqCst);

        self.spawn_params.lock().unwrap().push(SpawnParams {
            command: command.to_string(),
            args: args.to_vec(),
            cwd: cwd.map(|s| s.to_string()),
            env: env.cloned(),
            session_id: session_id.clone(),
            cols,
            rows,
        });

        if let Some(ref err) = self.spawn_error {
            return Err(err.to_session_error());
        }

        if let Some(ref result) = self.spawn_result {
            return Ok(result.clone());
        }

        Err(SessionError::LimitReached(0))
    }

    fn get(&self, session_id: &str) -> Result<Arc<Mutex<Session>>, SessionError> {
        self.get_calls.fetch_add(1, Ordering::SeqCst);

        if let Some(ref err) = self.get_error {
            return Err(err.to_session_error());
        }

        Err(SessionError::NotFound(session_id.to_string()))
    }

    fn active(&self) -> Result<Arc<Mutex<Session>>, SessionError> {
        if let Some(ref err) = self.resolve_error {
            return Err(err.to_session_error());
        }
        Err(SessionError::NoActiveSession)
    }

    fn resolve(&self, session_id: Option<&str>) -> Result<Arc<Mutex<Session>>, SessionError> {
        self.resolve_calls.fetch_add(1, Ordering::SeqCst);

        if let Some(ref err) = self.resolve_error {
            return Err(err.to_session_error());
        }

        match session_id {
            Some(id) => Err(SessionError::NotFound(id.to_string())),
            None => Err(SessionError::NoActiveSession),
        }
    }

    fn set_active(&self, session_id: &str) -> Result<(), SessionError> {
        self.set_active_calls.fetch_add(1, Ordering::SeqCst);
        self.activated_sessions
            .lock()
            .unwrap()
            .push(session_id.to_string());

        if let Some(ref err) = self.set_active_error {
            return Err(err.to_session_error());
        }

        Ok(())
    }

    fn list(&self) -> Vec<SessionInfo> {
        self.sessions_list.clone()
    }

    fn kill(&self, session_id: &str) -> Result<(), SessionError> {
        self.kill_calls.fetch_add(1, Ordering::SeqCst);
        self.killed_sessions
            .lock()
            .unwrap()
            .push(session_id.to_string());

        if let Some(ref err) = self.kill_error {
            return Err(err.to_session_error());
        }

        Ok(())
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

    pub fn with_kill_error(mut self, error: MockError) -> Self {
        self.repo.kill_error = Some(error);
        self
    }

    pub fn with_get_error(mut self, error: MockError) -> Self {
        self.repo.get_error = Some(error);
        self
    }

    pub fn with_set_active_error(mut self, error: MockError) -> Self {
        self.repo.set_active_error = Some(error);
        self
    }

    pub fn with_spawn_result(mut self, session_id: impl Into<String>, pid: u32) -> Self {
        self.repo.spawn_result = Some((SessionId::new(session_id.into()), pid));
        self
    }

    pub fn with_sessions(mut self, sessions: Vec<SessionInfo>) -> Self {
        self.repo.sessions_list = sessions;
        self
    }

    pub fn with_active_session(mut self, session_id: impl Into<String>) -> Self {
        self.repo.active_id = Some(SessionId::new(session_id.into()));
        self
    }

    pub fn with_session_count(mut self, count: usize) -> Self {
        self.repo.session_count = count;
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

        let result = repo.resolve(Some("session1"));

        assert!(matches!(result, Err(SessionError::NotFound(id)) if id == "custom"));
    }

    #[test]
    fn test_mock_repository_spawn_tracks_calls() {
        let repo = MockSessionRepository::builder()
            .with_spawn_result("test-session", 12345)
            .build();

        let result = repo.spawn("bash", &[], None, None, None, 80, 24);

        assert!(result.is_ok());
        let (session_id, pid) = result.unwrap();
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
            Some("custom-id".to_string()),
            120,
            40,
        );

        let params = repo.spawn_params();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].command, "bash");
        assert_eq!(params[0].args, vec!["--version"]);
        assert_eq!(params[0].cwd, Some("/tmp".to_string()));
        assert_eq!(params[0].session_id, Some("custom-id".to_string()));
        assert_eq!(params[0].cols, 120);
        assert_eq!(params[0].rows, 40);
    }

    #[test]
    fn test_mock_repository_kill_tracks_sessions() {
        let repo = MockSessionRepository::new();

        let _ = repo.kill("session1");
        let _ = repo.kill("session2");

        assert_eq!(repo.kill_call_count(), 2);
        assert_eq!(repo.killed_sessions(), vec!["session1", "session2"]);
    }

    #[test]
    fn test_mock_repository_set_active_tracks_sessions() {
        let repo = MockSessionRepository::new();

        let _ = repo.set_active("session1");
        let _ = repo.set_active("session2");

        assert_eq!(repo.set_active_call_count(), 2);
        assert_eq!(repo.activated_sessions(), vec!["session1", "session2"]);
    }

    #[test]
    fn test_builder_with_sessions_list() {
        let sessions = vec![SessionInfo {
            id: SessionId::new("sess1"),
            command: "bash".to_string(),
            pid: 1234,
            running: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            size: (80, 24),
        }];

        let repo = MockSessionRepository::builder()
            .with_sessions(sessions)
            .with_active_session("sess1")
            .with_session_count(1)
            .build();

        assert_eq!(repo.list().len(), 1);
        assert_eq!(repo.session_count(), 1);
        assert_eq!(repo.active_session_id().unwrap().as_str(), "sess1");
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
