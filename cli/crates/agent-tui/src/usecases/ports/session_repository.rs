use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::domain::ScrollDirection;
use crate::domain::core::Component;
use crate::domain::core::CursorPosition;
use crate::domain::session_types::SessionId;
use crate::domain::session_types::SessionInfo;

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
    fn terminal_write(&self, data: &[u8]) -> Result<(), SessionError>;
    fn terminal_try_read(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, SessionError>;
    fn stream_read(
        &self,
        cursor: &mut StreamCursor,
        max_bytes: usize,
        timeout_ms: i32,
    ) -> Result<StreamRead, SessionError>;
    fn stream_subscribe(&self) -> StreamWaiterHandle;
    fn analyze_screen(&self) -> Vec<Component>;
    fn keystroke(&self, key: &str) -> Result<(), SessionError>;
    fn type_text(&self, text: &str) -> Result<(), SessionError>;
    fn keydown(&self, key: &str) -> Result<(), SessionError>;
    fn keyup(&self, key: &str) -> Result<(), SessionError>;
    fn scroll(&self, direction: ScrollDirection, amount: u16) -> Result<(), SessionError>;
    fn is_running(&self) -> bool;
    fn resize(&self, cols: u16, rows: u16) -> Result<(), SessionError>;
    fn cursor(&self) -> CursorPosition;
    fn session_id(&self) -> SessionId;
    fn command(&self) -> String;
    fn size(&self) -> (u16, u16);
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
        session_id: Option<String>,
        cols: u16,
        rows: u16,
    ) -> Result<(SessionId, u32), SessionError>;

    fn get(&self, session_id: &str) -> Result<SessionHandle, SessionError>;
    fn active(&self) -> Result<SessionHandle, SessionError>;
    fn resolve(&self, session_id: Option<&str>) -> Result<SessionHandle, SessionError>;
    fn set_active(&self, session_id: &str) -> Result<(), SessionError>;
    fn list(&self) -> Vec<SessionInfo>;
    fn kill(&self, session_id: &str) -> Result<(), SessionError>;
    fn session_count(&self) -> usize;
    fn active_session_id(&self) -> Option<SessionId>;
}
