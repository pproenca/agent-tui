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
#[path = "mock_repository_tests.rs"]
mod tests;
