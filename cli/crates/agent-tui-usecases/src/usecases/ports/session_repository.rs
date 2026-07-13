//! Session repository port.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::domain::RestartOutput;
use crate::domain::core::CursorPosition;
use crate::domain::session_types::SessionId;
use crate::domain::session_types::SessionInfo;
use crate::domain::session_types::TerminalSize;

use super::SessionError;

#[derive(Debug, Clone)]
pub struct LivePreviewSnapshot {
    pub cols: u16,
    pub rows: u16,
    pub seq: String,
    pub stream_seq: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StreamCursor {
    pub seq: u64,
}

#[derive(Debug, Clone)]
pub struct StreamRead {
    pub data: Vec<u8>,
    pub next_cursor: StreamCursor,
    pub latest_cursor: StreamCursor,
    pub dropped_bytes: u64,
    pub closed: bool,
}

pub trait StreamWaiter: Send + Sync {
    fn wait(&self, timeout: Option<Duration>) -> bool;
}

pub type StreamWaiterHandle = Arc<dyn StreamWaiter>;

pub trait SessionOps: Send + Sync {
    fn update(&self) -> Result<(), SessionError>;
    fn screen_text(&self) -> String;
    fn screen_render(&self) -> String;
    fn screen_render_compact(&self) -> String;
    fn terminal_write(&self, data: &[u8]) -> Result<(), SessionError>;
    fn stream_read(
        &self,
        cursor: &mut StreamCursor,
        max_bytes: usize,
        timeout_ms: i32,
    ) -> Result<StreamRead, SessionError>;
    fn stream_subscribe(&self) -> StreamWaiterHandle;
    fn keystroke(&self, key: &str) -> Result<(), SessionError>;
    fn type_text(&self, text: &str) -> Result<(), SessionError>;
    fn keydown(&self, key: &str) -> Result<(), SessionError>;
    fn keyup(&self, key: &str) -> Result<(), SessionError>;
    fn is_running(&self) -> bool;
    fn resize(&self, size: TerminalSize) -> Result<(), SessionError>;
    fn cursor(&self) -> CursorPosition;
    fn session_id(&self) -> SessionId;
    fn size(&self) -> TerminalSize;
    fn live_preview_snapshot(&self) -> LivePreviewSnapshot;
}

pub type SessionHandle = Arc<dyn SessionOps>;

#[allow(clippy::too_many_arguments)]
pub trait SessionRepository: Send + Sync {
    fn spawn(
        &self,
        command: &str,
        args: &[String],
        cwd: Option<&str>,
        env: Option<&HashMap<String, String>>,
        session_id: Option<SessionId>,
        size: TerminalSize,
    ) -> Result<(SessionId, u32), SessionError>;

    fn resolve(&self, session_id: Option<&SessionId>) -> Result<SessionHandle, SessionError>;
    fn set_active(&self, session_id: &SessionId) -> Result<(), SessionError>;
    fn list(&self) -> Vec<SessionInfo>;
    fn kill(&self, session_id: &SessionId) -> Result<(), SessionError>;
    fn restart(&self, session_id: Option<&SessionId>) -> Result<RestartOutput, SessionError>;
    fn active_session_id(&self) -> Option<SessionId>;
}
