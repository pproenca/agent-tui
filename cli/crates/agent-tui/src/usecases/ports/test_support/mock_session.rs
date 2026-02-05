//! Mock session handle for use case tests.

use crate::domain::core::CursorPosition;
use crate::domain::session_types::SessionId;
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
    cols: u16,
    rows: u16,
    cursor: CursorPosition,
    screen_text: String,
    update_error: Option<SessionError>,
    terminal_write_error: Option<SessionError>,
    written_data: Mutex<Vec<Vec<u8>>>,
}

impl MockSession {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            command: "mock".to_string(),
            cols: 80,
            rows: 24,
            cursor: CursorPosition {
                row: 0,
                col: 0,
                visible: false,
            },
            screen_text: String::new(),
            update_error: None,
            terminal_write_error: None,
            written_data: Mutex::new(Vec::new()),
        }
    }

    pub fn builder(id: impl Into<String>) -> MockSessionBuilder {
        MockSessionBuilder::new(id)
    }

    pub fn written_data(&self) -> Vec<Vec<u8>> {
        self.written_data.lock().unwrap().clone()
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
        self.screen_text.clone()
    }

    fn terminal_write(&self, data: &[u8]) -> Result<(), SessionError> {
        if let Some(ref err) = self.terminal_write_error {
            Err(SessionError::Terminal(TerminalError::Write {
                reason: err.to_string(),
                source: None,
            }))
        } else {
            self.written_data.lock().unwrap().push(data.to_vec());
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

    fn is_running(&self) -> bool {
        true
    }

    fn resize(&self, cols: u16, rows: u16) -> Result<(), SessionError> {
        let _ = (cols, rows);
        Ok(())
    }

    fn cursor(&self) -> CursorPosition {
        self.cursor
    }

    fn session_id(&self) -> SessionId {
        SessionId::new(self.id.clone())
    }

    fn command(&self) -> String {
        self.command.clone()
    }

    fn size(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }

    fn live_preview_snapshot(&self) -> LivePreviewSnapshot {
        LivePreviewSnapshot {
            cols: self.cols,
            rows: self.rows,
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

    pub fn with_update_error(mut self, error: SessionError) -> Self {
        self.session.update_error = Some(error);
        self
    }

    pub fn build(self) -> MockSession {
        self.session
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_session_default_screen_text() {
        let session = MockSession::new("test-session");
        assert_eq!(session.screen_text(), "");
    }

    #[test]
    fn test_mock_session_with_screen_text() {
        let session = MockSession::builder("test")
            .with_screen_text("Hello, World!")
            .build();
        assert_eq!(session.screen_text(), "Hello, World!");
    }

    #[test]
    fn test_mock_session_update_succeeds() {
        let session = MockSession::new("test");
        let result = session.update();
        assert!(result.is_ok());
    }

    #[test]
    fn test_mock_session_update_with_error() {
        let session = MockSession::builder("test")
            .with_update_error(SessionError::NoActiveSession)
            .build();

        let result = session.update();
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_session_terminal_write_tracks_data() {
        let session = MockSession::new("test");

        session.terminal_write(b"hello").unwrap();
        session.terminal_write(b"world").unwrap();

        let written = session.written_data();
        assert_eq!(written.len(), 2);
        assert_eq!(written[0], b"hello");
        assert_eq!(written[1], b"world");
    }

    #[test]
    fn test_mock_session_builder_chaining() {
        let session = MockSession::builder("chain-test")
            .with_screen_text("Screen content")
            .build();

        assert_eq!(session.id, "chain-test");
        assert_eq!(session.screen_text(), "Screen content");
    }
}
