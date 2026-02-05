//! Session repository implementation.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use crate::common::mutex_lock_or_recover;
use crate::domain::core::CursorPosition;
use crate::usecases::ports::LivePreviewSnapshot;
use crate::usecases::ports::SessionError;
use crate::usecases::ports::SessionHandle;
use crate::usecases::ports::SessionOps;
use crate::usecases::ports::SessionRepository;
use crate::usecases::ports::StreamCursor;
use crate::usecases::ports::StreamRead;
use crate::usecases::ports::StreamWaiterHandle;

use crate::infra::daemon::session::PUMP_FLUSH_TIMEOUT;
use crate::infra::daemon::session::Session;
use crate::infra::daemon::session::SessionId;
use crate::infra::daemon::session::SessionInfo;
use crate::infra::daemon::session::SessionManager;
use crate::infra::daemon::session::StreamReader;

struct SessionHandleImpl {
    inner: Arc<Mutex<Session>>,
    stream: StreamReader,
    pty_cursor: Arc<Mutex<StreamCursor>>,
}

impl SessionHandleImpl {
    fn new_handle(inner: Arc<Mutex<Session>>) -> SessionHandle {
        let (stream, pty_cursor) = {
            let session_guard = mutex_lock_or_recover(&inner);
            (
                session_guard.stream_reader(),
                session_guard.pty_cursor_handle(),
            )
        };
        Arc::new(Self {
            inner,
            stream,
            pty_cursor,
        })
    }
}

impl SessionOps for SessionHandleImpl {
    fn update(&self) -> Result<(), SessionError> {
        let ack = {
            let session_guard = mutex_lock_or_recover(&self.inner);
            session_guard.request_flush()
        };
        if let Some(ack) = ack {
            let _ = ack.recv_timeout(PUMP_FLUSH_TIMEOUT);
        }
        Ok(())
    }

    fn screen_text(&self) -> String {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.screen_text()
    }

    fn screen_render(&self) -> String {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.screen_render()
    }

    fn terminal_write(&self, data: &[u8]) -> Result<(), SessionError> {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.pty_write(data)
    }

    fn terminal_try_read(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, SessionError> {
        let mut cursor = self.pty_cursor.lock().unwrap_or_else(|e| e.into_inner());
        let read = self.stream.read(&mut cursor, buf.len(), timeout_ms)?;
        let bytes_read = read.data.len().min(buf.len());
        buf[..bytes_read].copy_from_slice(&read.data[..bytes_read]);
        Ok(bytes_read)
    }

    fn stream_read(
        &self,
        cursor: &mut StreamCursor,
        max_bytes: usize,
        timeout_ms: i32,
    ) -> Result<StreamRead, SessionError> {
        self.stream.read(cursor, max_bytes, timeout_ms)
    }

    fn stream_subscribe(&self) -> StreamWaiterHandle {
        self.stream.subscribe()
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

    fn live_preview_snapshot(&self) -> LivePreviewSnapshot {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.live_preview_snapshot()
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
        Ok(SessionHandleImpl::new_handle(session))
    }

    fn active(&self) -> Result<SessionHandle, SessionError> {
        let session = SessionManager::active(self)?;
        Ok(SessionHandleImpl::new_handle(session))
    }

    fn resolve(&self, session_id: Option<&str>) -> Result<SessionHandle, SessionError> {
        let session = SessionManager::resolve(self, session_id)?;
        Ok(SessionHandleImpl::new_handle(session))
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
        fn assert_generic_bound<S: SessionOps + ?Sized>(_session: &S) {}

        let manager = SessionManager::new();
        let cwd = std::env::current_dir()
            .ok()
            .map(|path| path.to_string_lossy().into_owned());
        let session_handle = manager
            .spawn("/bin/sh", &[], cwd.as_deref(), None, None, 80, 24)
            .and_then(|(id, _)| SessionRepository::get(&manager, id.as_str()))
            .unwrap();
        assert_generic_bound(session_handle.as_ref());
    }
}
