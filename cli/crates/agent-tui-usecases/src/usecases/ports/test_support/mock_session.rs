//! Mock session handle for use case tests.

use crate::common::mutex_lock_or_recover;
use crate::domain::core::CursorPosition;
use crate::domain::session_types::SessionId;
use crate::domain::session_types::TerminalSize;
use crate::usecases::ports::LivePreviewSnapshot;
use crate::usecases::ports::SessionError;
use crate::usecases::ports::SessionOps;
use crate::usecases::ports::StreamCursor;
use crate::usecases::ports::StreamRead;
use crate::usecases::ports::StreamWaiter;
use crate::usecases::ports::StreamWaiterHandle;
use crate::usecases::ports::TerminalError;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

struct MockStreamWaiter;

impl StreamWaiter for MockStreamWaiter {
    fn wait(&self, _timeout: Option<Duration>) -> bool {
        true
    }
}

pub struct MockSession {
    pub id: String,
    command: String,
    size: TerminalSize,
    cursor: CursorPosition,
    screen_text: String,
    screen_render: Option<String>,
    screen_render_compact: Option<String>,
    running: bool,
    update_error: Option<SessionError>,
    terminal_write_error: Option<SessionError>,
    written_data: Mutex<Vec<Vec<u8>>>,
    mouse_calls: Mutex<Vec<String>>,
}

impl MockSession {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            command: "mock".to_string(),
            size: TerminalSize::default(),
            cursor: CursorPosition {
                row: 0,
                col: 0,
                visible: false,
            },
            screen_text: String::new(),
            screen_render: None,
            screen_render_compact: None,
            running: true,
            update_error: None,
            terminal_write_error: None,
            written_data: Mutex::new(Vec::new()),
            mouse_calls: Mutex::new(Vec::new()),
        }
    }

    pub fn builder(id: impl Into<String>) -> MockSessionBuilder {
        MockSessionBuilder::new(id)
    }

    pub fn written_data(&self) -> Vec<Vec<u8>> {
        mutex_lock_or_recover(&self.written_data).clone()
    }

    pub fn mouse_calls(&self) -> Vec<String> {
        mutex_lock_or_recover(&self.mouse_calls).clone()
    }
}

impl SessionOps for MockSession {
    fn update(&self) -> Result<(), SessionError> {
        if let Some(ref err) = self.update_error {
            Err(SessionError::Terminal(TerminalError::Write {
                reason: err.to_string(),
                source: None,
            }))
        } else {
            Ok(())
        }
    }

    fn screen_text(&self) -> String {
        self.screen_text.clone()
    }

    fn screen_render(&self) -> String {
        self.screen_render
            .clone()
            .unwrap_or_else(|| self.screen_text.clone())
    }

    fn screen_render_compact(&self) -> String {
        self.screen_render_compact
            .clone()
            .unwrap_or_else(|| self.screen_text.clone())
    }

    fn terminal_write(&self, data: &[u8]) -> Result<(), SessionError> {
        if let Some(ref err) = self.terminal_write_error {
            Err(SessionError::Terminal(TerminalError::Write {
                reason: err.to_string(),
                source: None,
            }))
        } else {
            mutex_lock_or_recover(&self.written_data).push(data.to_vec());
            Ok(())
        }
    }

    fn terminal_try_read(&self, _buf: &mut [u8], _timeout_ms: i32) -> Result<usize, SessionError> {
        Ok(0)
    }

    fn stream_read(
        &self,
        cursor: &mut StreamCursor,
        _max_bytes: usize,
        _timeout_ms: i32,
    ) -> Result<StreamRead, SessionError> {
        Ok(StreamRead {
            data: Vec::new(),
            next_cursor: *cursor,
            latest_cursor: *cursor,
            dropped_bytes: 0,
            closed: false,
        })
    }

    fn stream_subscribe(&self) -> StreamWaiterHandle {
        Arc::new(MockStreamWaiter)
    }

    fn keystroke(&self, _key: &str) -> Result<(), SessionError> {
        Ok(())
    }

    fn type_text(&self, _text: &str) -> Result<(), SessionError> {
        Ok(())
    }

    fn keydown(&self, _key: &str) -> Result<(), SessionError> {
        Ok(())
    }

    fn keyup(&self, _key: &str) -> Result<(), SessionError> {
        Ok(())
    }

    fn mouse_click(&self, col: u16, row: u16, button: &str) -> Result<(), SessionError> {
        mutex_lock_or_recover(&self.mouse_calls).push(format!("click {}x{} {}", col, row, button));
        Ok(())
    }

    fn mouse_move(&self, col: u16, row: u16) -> Result<(), SessionError> {
        mutex_lock_or_recover(&self.mouse_calls).push(format!("move {}x{}", col, row));
        Ok(())
    }

    fn mouse_down(&self, col: u16, row: u16, button: &str) -> Result<(), SessionError> {
        mutex_lock_or_recover(&self.mouse_calls).push(format!("down {}x{} {}", col, row, button));
        Ok(())
    }

    fn mouse_up(&self, col: u16, row: u16, button: &str) -> Result<(), SessionError> {
        mutex_lock_or_recover(&self.mouse_calls).push(format!("up {}x{} {}", col, row, button));
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn resize(&self, size: TerminalSize) -> Result<(), SessionError> {
        let _ = size;
        Ok(())
    }

    fn cursor(&self) -> CursorPosition {
        self.cursor
    }

    fn session_id(&self) -> SessionId {
        SessionId::try_new(self.id.clone()).expect("mock session id should be valid")
    }

    fn command(&self) -> String {
        self.command.clone()
    }

    fn size(&self) -> TerminalSize {
        self.size
    }

    fn live_preview_snapshot(&self) -> LivePreviewSnapshot {
        LivePreviewSnapshot {
            cols: self.size.cols(),
            rows: self.size.rows(),
            seq: self.screen_text.clone(),
            stream_seq: 0,
        }
    }
}

pub struct MockSessionBuilder {
    session: MockSession,
}

impl MockSessionBuilder {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            session: MockSession::new(id),
        }
    }

    pub fn with_screen_text(mut self, text: impl Into<String>) -> Self {
        self.session.screen_text = text.into();
        self
    }

    pub fn with_rendered_screen(
        mut self,
        rendered: impl Into<String>,
        compact_rendered: impl Into<String>,
    ) -> Self {
        self.session.screen_render = Some(rendered.into());
        self.session.screen_render_compact = Some(compact_rendered.into());
        self
    }

    pub fn with_running(mut self, running: bool) -> Self {
        self.session.running = running;
        self
    }

    pub fn with_update_error(mut self, error: SessionError) -> Self {
        self.session.update_error = Some(error);
        self
    }

    pub fn build(self) -> MockSession {
        self.session
    }
}

#[cfg(test)]
#[path = "mock_session_tests.rs"]
mod tests;
