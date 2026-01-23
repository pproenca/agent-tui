use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use agent_tui_core::CursorPosition;
use agent_tui_core::Element;

use crate::error::SessionError;
use crate::session::{Session, SessionId, SessionInfo, SessionManager};

/// Repository trait for session access and management.
///
/// This trait abstracts the session storage and retrieval operations,
/// enabling use cases to be testable without a real SessionManager.
#[allow(clippy::too_many_arguments)]
pub trait SessionRepository: Send + Sync {
    /// Spawn a new session with the given parameters.
    fn spawn(
        &self,
        command: &str,
        args: &[String],
        cwd: Option<&str>,
        env: Option<&HashMap<String, String>>,
        session_id: Option<String>,
        cols: u16,
        rows: u16,
    ) -> Result<(SessionId, u32), SessionError>;

    /// Get a session by ID.
    fn get(&self, session_id: &str) -> Result<Arc<Mutex<Session>>, SessionError>;

    /// Get the active session.
    fn active(&self) -> Result<Arc<Mutex<Session>>, SessionError>;

    /// Resolve a session by ID, falling back to active session if None.
    fn resolve(&self, session_id: Option<&str>) -> Result<Arc<Mutex<Session>>, SessionError>;

    /// Set the active session.
    fn set_active(&self, session_id: &str) -> Result<(), SessionError>;

    /// List all sessions.
    fn list(&self) -> Vec<SessionInfo>;

    /// Kill a session by ID.
    fn kill(&self, session_id: &str) -> Result<(), SessionError>;

    /// Get the count of sessions.
    fn session_count(&self) -> usize;

    /// Get the active session ID.
    fn active_session_id(&self) -> Option<SessionId>;
}

impl SessionRepository for SessionManager {
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
        SessionManager::spawn(self, command, args, cwd, env, session_id, cols, rows)
    }

    fn get(&self, session_id: &str) -> Result<Arc<Mutex<Session>>, SessionError> {
        SessionManager::get(self, session_id)
    }

    fn active(&self) -> Result<Arc<Mutex<Session>>, SessionError> {
        SessionManager::active(self)
    }

    fn resolve(&self, session_id: Option<&str>) -> Result<Arc<Mutex<Session>>, SessionError> {
        SessionManager::resolve(self, session_id)
    }

    fn set_active(&self, session_id: &str) -> Result<(), SessionError> {
        SessionManager::set_active(self, session_id)
    }

    fn list(&self) -> Vec<SessionInfo> {
        SessionManager::list(self)
    }

    fn kill(&self, session_id: &str) -> Result<(), SessionError> {
        SessionManager::kill(self, session_id)
    }

    fn session_count(&self) -> usize {
        SessionManager::session_count(self)
    }

    fn active_session_id(&self) -> Option<SessionId> {
        SessionManager::active_session_id(self)
    }
}

/// Snapshot of a session's current state.
#[derive(Debug, Clone)]
pub struct SessionSnapshot {
    pub session_id: SessionId,
    pub screen: String,
    pub elements: Vec<Element>,
    pub cursor: CursorPosition,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_repository_trait_is_object_safe() {
        fn assert_object_safe<T: ?Sized>(_: &T)
        where
            T: SessionRepository,
        {
        }

        let manager = SessionManager::new();
        assert_object_safe(&manager);
    }
}
