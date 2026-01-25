use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use crate::common::mutex_lock_or_recover;
use crate::domain::core::Component;
use crate::domain::core::CursorPosition;
use crate::domain::core::Element;
use crate::usecases::ports::{SessionError, SessionHandle, SessionOps, SessionRepository};

use crate::infra::daemon::session::{Session, SessionId, SessionInfo, SessionManager};

struct SessionHandleImpl {
    inner: Arc<Mutex<Session>>,
}

impl SessionHandleImpl {
    fn new(inner: Arc<Mutex<Session>>) -> SessionHandle {
        Arc::new(Self { inner })
    }
}

impl SessionOps for SessionHandleImpl {
    fn update(&self) -> Result<(), SessionError> {
        let mut session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.update()
    }

    fn screen_text(&self) -> String {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.screen_text()
    }

    fn detect_elements(&self) -> Vec<Element> {
        let mut session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.detect_elements().to_vec()
    }

    fn find_element(&self, element_ref: &str) -> Option<Element> {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.find_element(element_ref).cloned()
    }

    fn pty_write(&self, data: &[u8]) -> Result<(), SessionError> {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.pty_write(data)
    }

    fn pty_try_read(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, SessionError> {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.pty_try_read(buf, timeout_ms)
    }

    fn analyze_screen(&self) -> Vec<Component> {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.analyze_screen()
    }

    fn click(&self, element_ref: &str) -> Result<(), SessionError> {
        let mut session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.click(element_ref)
    }

    fn keystroke(&self, key: &str) -> Result<(), SessionError> {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.keystroke(key)
    }

    fn type_text(&self, text: &str) -> Result<(), SessionError> {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.type_text(text)
    }

    fn keydown(&self, key: &str) -> Result<(), SessionError> {
        let mut session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.keydown(key)
    }

    fn keyup(&self, key: &str) -> Result<(), SessionError> {
        let mut session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.keyup(key)
    }

    fn is_running(&self) -> bool {
        let mut session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.is_running()
    }

    fn resize(&self, cols: u16, rows: u16) -> Result<(), SessionError> {
        let mut session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.resize(cols, rows)
    }

    fn cursor(&self) -> CursorPosition {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.cursor()
    }

    fn session_id(&self) -> SessionId {
        let session_guard = mutex_lock_or_recover(&self.inner);
        SessionId::from(session_guard.id.as_str())
    }

    fn command(&self) -> String {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.command.clone()
    }

    fn size(&self) -> (u16, u16) {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.size()
    }

    fn start_recording(&self) {
        let mut session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.start_recording();
    }

    fn stop_recording(&self) -> Vec<crate::domain::session_types::RecordingFrame> {
        let mut session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.stop_recording()
    }

    fn recording_status(&self) -> crate::domain::session_types::RecordingStatus {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.recording_status()
    }

    fn get_trace_entries(&self, count: usize) -> Vec<crate::domain::session_types::TraceEntry> {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.get_trace_entries(count)
    }

    fn get_errors(&self, count: usize) -> Vec<crate::domain::session_types::ErrorEntry> {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.get_errors(count)
    }

    fn clear_errors(&self) {
        let mut session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.clear_errors();
    }

    fn clear_console(&self) {
        let mut session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.clear_console();
    }
}

#[allow(clippy::too_many_arguments)]
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

    fn get(&self, session_id: &str) -> Result<SessionHandle, SessionError> {
        let session = SessionManager::get(self, session_id)?;
        Ok(SessionHandleImpl::new(session))
    }

    fn active(&self) -> Result<SessionHandle, SessionError> {
        let session = SessionManager::active(self)?;
        Ok(SessionHandleImpl::new(session))
    }

    fn resolve(&self, session_id: Option<&str>) -> Result<SessionHandle, SessionError> {
        let session = SessionManager::resolve(self, session_id)?;
        Ok(SessionHandleImpl::new(session))
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

        let manager = SessionManager::new();
        let session = manager
            .spawn("bash", &[], None, None, None, 80, 24)
            .and_then(|(id, _)| manager.get(id.as_str()))
            .unwrap();
        assert_generic_bound(&*session);
    }
}
