use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use crate::core::Component;
use crate::core::CursorPosition;
use crate::core::Element;

use crate::daemon::error::SessionError;
use crate::daemon::session::{Session, SessionId, SessionInfo, SessionManager};

pub trait SessionOps {
    fn update(&mut self) -> Result<(), SessionError>;

    fn screen_text(&self) -> String;

    fn detect_elements(&mut self) -> &[Element];

    fn find_element(&self, element_ref: &str) -> Option<&Element>;

    fn pty_write(&mut self, data: &[u8]) -> Result<(), SessionError>;

    fn analyze_screen(&self) -> Vec<Component>;
}

#[allow(clippy::too_many_arguments)]
pub trait SessionRepository: Send + Sync {
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

    fn get(&self, session_id: &str) -> Result<Arc<Mutex<Session>>, SessionError>;

    fn active(&self) -> Result<Arc<Mutex<Session>>, SessionError>;

    fn resolve(&self, session_id: Option<&str>) -> Result<Arc<Mutex<Session>>, SessionError>;

    fn set_active(&self, session_id: &str) -> Result<(), SessionError>;

    fn list(&self) -> Vec<SessionInfo>;

    fn kill(&self, session_id: &str) -> Result<(), SessionError>;

    fn session_count(&self) -> usize;

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
        fn assert_object_safe<T>(_: &T)
        where
            T: SessionRepository + ?Sized,
        {
        }

        let manager = SessionManager::new();
        assert_object_safe(&manager);
    }

    #[test]
    fn test_session_ops_trait_is_usable_as_generic_bound() {
        fn assert_generic_bound<S: SessionOps>(_session: &S) {}

        let _ = assert_generic_bound::<Session>;
    }
}
