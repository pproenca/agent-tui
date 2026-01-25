use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::core::{Component, CursorPosition, Element};
use crate::domain::session_types::{ErrorEntry, RecordingFrame, RecordingStatus, SessionId, SessionInfo, TraceEntry};

use super::SessionError;

pub trait SessionOps: Send + Sync {
    fn update(&self) -> Result<(), SessionError>;
    fn screen_text(&self) -> String;
    fn detect_elements(&self) -> Vec<Element>;
    fn find_element(&self, element_ref: &str) -> Option<Element>;
    fn pty_write(&self, data: &[u8]) -> Result<(), SessionError>;
    fn pty_try_read(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, SessionError>;
    fn analyze_screen(&self) -> Vec<Component>;
    fn click(&self, element_ref: &str) -> Result<(), SessionError>;
    fn keystroke(&self, key: &str) -> Result<(), SessionError>;
    fn type_text(&self, text: &str) -> Result<(), SessionError>;
    fn keydown(&self, key: &str) -> Result<(), SessionError>;
    fn keyup(&self, key: &str) -> Result<(), SessionError>;
    fn is_running(&self) -> bool;
    fn resize(&self, cols: u16, rows: u16) -> Result<(), SessionError>;
    fn cursor(&self) -> CursorPosition;
    fn session_id(&self) -> SessionId;
    fn command(&self) -> String;
    fn size(&self) -> (u16, u16);
    fn start_recording(&self);
    fn stop_recording(&self) -> Vec<RecordingFrame>;
    fn recording_status(&self) -> RecordingStatus;
    fn get_trace_entries(&self, count: usize) -> Vec<TraceEntry>;
    fn get_errors(&self, count: usize) -> Vec<ErrorEntry>;
    fn clear_errors(&self);
    fn clear_console(&self);
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
