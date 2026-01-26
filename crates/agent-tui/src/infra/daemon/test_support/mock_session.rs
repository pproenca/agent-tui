use crate::domain::core::CursorPosition;
use crate::domain::core::Element;
use crate::domain::core::vom::Component;
use std::sync::Mutex;

use crate::domain::session_types::SessionId;
use crate::usecases::ports::{
    LivePreviewSnapshot, SessionOps, StreamCursor, StreamRead, StreamSubscription,
};
use crate::usecases::ports::{PtyError, SessionError};

pub struct MockSession {
    pub id: String,
    command: String,
    cols: u16,
    rows: u16,
    cursor: CursorPosition,
    screen_text: String,
    elements: Vec<Element>,
    components: Vec<Component>,
    update_error: Option<SessionError>,
    pty_write_error: Option<SessionError>,
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
            elements: Vec::new(),
            components: Vec::new(),
            update_error: None,
            pty_write_error: None,
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
            Err(SessionError::Pty(PtyError::Write(err.to_string())))
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

    fn detect_elements(&self) -> Vec<Element> {
        self.elements.clone()
    }

    fn find_element(&self, element_ref: &str) -> Option<Element> {
        self.elements
            .iter()
            .find(|e| e.element_ref == element_ref)
            .cloned()
    }

    fn pty_write(&self, data: &[u8]) -> Result<(), SessionError> {
        if let Some(ref err) = self.pty_write_error {
            Err(SessionError::Pty(PtyError::Write(err.to_string())))
        } else {
            self.written_data.lock().unwrap().push(data.to_vec());
            Ok(())
        }
    }

    fn pty_try_read(&self, _buf: &mut [u8], _timeout_ms: i32) -> Result<usize, SessionError> {
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

    fn stream_subscribe(&self) -> StreamSubscription {
        let (_tx, rx) = crossbeam_channel::bounded(1);
        StreamSubscription::new(rx)
    }

    fn analyze_screen(&self) -> Vec<Component> {
        self.components.clone()
    }

    fn click(&self, _element_ref: &str) -> Result<(), SessionError> {
        Ok(())
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
        self.cursor.clone()
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

    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.session.command = command.into();
        self
    }

    pub fn with_size(mut self, cols: u16, rows: u16) -> Self {
        self.session.cols = cols;
        self.session.rows = rows;
        self
    }

    pub fn with_elements(mut self, elements: Vec<Element>) -> Self {
        self.session.elements = elements;
        self
    }

    pub fn with_components(mut self, components: Vec<Component>) -> Self {
        self.session.components = components;
        self
    }

    pub fn with_update_error(mut self, error: SessionError) -> Self {
        self.session.update_error = Some(error);
        self
    }

    pub fn with_pty_write_error(mut self, error: SessionError) -> Self {
        self.session.pty_write_error = Some(error);
        self
    }

    pub fn build(self) -> MockSession {
        self.session
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::core::{ElementType, Position};

    fn make_element(ref_id: &str) -> Element {
        Element {
            element_ref: ref_id.to_string(),
            element_type: ElementType::Button,
            label: Some("Test".to_string()),
            value: None,
            position: Position {
                row: 0,
                col: 0,
                width: Some(10),
                height: Some(1),
            },
            focused: false,
            selected: false,
            checked: None,
            disabled: None,
            hint: None,
        }
    }

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
    fn test_mock_session_with_elements() {
        let elements = vec![make_element("@e1"), make_element("@e2")];
        let session = MockSession::builder("test").with_elements(elements).build();

        let detected = session.detect_elements();
        assert_eq!(detected.len(), 2);
        assert_eq!(detected[0].element_ref, "@e1");
        assert_eq!(detected[1].element_ref, "@e2");
    }

    #[test]
    fn test_mock_session_find_element_by_ref() {
        let elements = vec![make_element("@btn1"), make_element("@btn2")];
        let session = MockSession::builder("test").with_elements(elements).build();

        let found = session.find_element("@btn1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().element_ref, "@btn1");
    }

    #[test]
    fn test_mock_session_find_element_not_found() {
        let session = MockSession::new("test");
        let found = session.find_element("@missing");
        assert!(found.is_none());
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
    fn test_mock_session_pty_write_tracks_data() {
        let session = MockSession::new("test");

        session.pty_write(b"hello").unwrap();
        session.pty_write(b"world").unwrap();

        let written = session.written_data();
        assert_eq!(written.len(), 2);
        assert_eq!(written[0], b"hello");
        assert_eq!(written[1], b"world");
    }

    #[test]
    fn test_mock_session_analyze_screen_returns_components() {
        use crate::domain::core::vom::{Rect, Role};

        let components = vec![Component::new(
            Role::Button,
            Rect::new(0, 0, 10, 1),
            "OK".to_string(),
            12345,
        )];

        let session = MockSession::builder("test")
            .with_components(components)
            .build();

        let analyzed = session.analyze_screen();
        assert_eq!(analyzed.len(), 1);
        assert_eq!(analyzed[0].text_content, "OK");
    }

    #[test]
    fn test_mock_session_builder_chaining() {
        let session = MockSession::builder("chain-test")
            .with_screen_text("Screen content")
            .with_elements(vec![make_element("@e1")])
            .build();

        assert_eq!(session.id, "chain-test");
        assert_eq!(session.screen_text(), "Screen content");
    }
}
