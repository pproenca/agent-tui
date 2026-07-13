//! Session repository implementation.

use crossbeam_channel as channel;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use crate::common::mutex_lock_or_recover;
use crate::domain::RestartOutput;
use crate::domain::core::CursorPosition;
use crate::domain::session_types::TerminalSize;
use crate::usecases::ports::LivePreviewSnapshot;
use crate::usecases::ports::SessionError;
use crate::usecases::ports::SessionHandle;
use crate::usecases::ports::SessionOps;
use crate::usecases::ports::SessionRepository;
use crate::usecases::ports::StreamCursor;
use crate::usecases::ports::StreamRead;
use crate::usecases::ports::StreamWaiterHandle;
use crate::usecases::ports::TerminalError;

use crate::infra::daemon::session::PUMP_FLUSH_TIMEOUT;
use crate::infra::daemon::session::Session;
use crate::infra::daemon::session::SessionId;
use crate::infra::daemon::session::SessionInfo;
use crate::infra::daemon::session::SessionManager;
use crate::infra::daemon::session::StreamReader;

struct SessionHandleImpl {
    inner: Arc<Mutex<Session>>,
    stream: StreamReader,
}

impl SessionHandleImpl {
    fn new_handle(inner: Arc<Mutex<Session>>) -> SessionHandle {
        let stream = {
            let session_guard = mutex_lock_or_recover(&inner);
            session_guard.stream_reader()
        };
        Arc::new(Self { inner, stream })
    }
}

fn wait_for_flush_ack(
    ack: Option<channel::Receiver<()>>,
    timeout: Duration,
) -> Result<(), SessionError> {
    let Some(ack) = ack else {
        return Ok(());
    };

    ack.recv_timeout(timeout).map_err(|err| match err {
        channel::RecvTimeoutError::Timeout => SessionError::Terminal(TerminalError::Read {
            reason: "Timed out waiting for session state to flush".to_string(),
            source: None,
        }),
        channel::RecvTimeoutError::Disconnected => SessionError::Terminal(TerminalError::Read {
            reason: "Session state flush closed before it acknowledged".to_string(),
            source: None,
        }),
    })
}

impl SessionOps for SessionHandleImpl {
    fn update(&self) -> Result<(), SessionError> {
        let ack = {
            let session_guard = mutex_lock_or_recover(&self.inner);
            session_guard.request_flush()
        };
        wait_for_flush_ack(ack, PUMP_FLUSH_TIMEOUT)
    }

    fn screen_text(&self) -> String {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.screen_text()
    }

    fn screen_render(&self) -> String {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.screen_render()
    }

    fn screen_render_compact(&self) -> String {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.screen_render_compact()
    }

    fn terminal_write(&self, data: &[u8]) -> Result<(), SessionError> {
        let mut session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.pty_write(data)
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
        let mut session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.keystroke(key)
    }

    fn type_text(&self, text: &str) -> Result<(), SessionError> {
        let mut session_guard = mutex_lock_or_recover(&self.inner);
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

    fn resize(&self, size: TerminalSize) -> Result<(), SessionError> {
        let mut session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.resize(size)
    }

    fn cursor(&self) -> CursorPosition {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.cursor()
    }

    fn session_id(&self) -> SessionId {
        let session_guard = mutex_lock_or_recover(&self.inner);
        session_guard.id.clone()
    }

    fn size(&self) -> TerminalSize {
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
        session_id: Option<SessionId>,
        size: TerminalSize,
    ) -> Result<(SessionId, u32), SessionError> {
        SessionManager::spawn(self, command, args, cwd, env, session_id, size)
    }

    fn resolve(&self, session_id: Option<&SessionId>) -> Result<SessionHandle, SessionError> {
        let session = SessionManager::resolve(self, session_id)?;
        Ok(SessionHandleImpl::new_handle(session))
    }

    fn set_active(&self, session_id: &SessionId) -> Result<(), SessionError> {
        SessionManager::set_active(self, session_id)
    }

    fn list(&self) -> Vec<SessionInfo> {
        SessionManager::list(self)
    }

    fn kill(&self, session_id: &SessionId) -> Result<(), SessionError> {
        SessionManager::kill(self, session_id)
    }

    fn restart(&self, session_id: Option<&SessionId>) -> Result<RestartOutput, SessionError> {
        SessionManager::restart(self, session_id)
    }

    fn active_session_id(&self) -> Option<SessionId> {
        SessionManager::active_session_id(self)
    }
}

#[cfg(test)]
#[path = "repository_tests.rs"]
mod tests;
