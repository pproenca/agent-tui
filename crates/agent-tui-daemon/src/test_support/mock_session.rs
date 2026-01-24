//! Mock implementation of SessionOps for testing use cases.
//!
//! This module provides a MockSession that implements the SessionOps trait,
//! enabling happy-path testing of use cases without real PTY sessions.

use agent_tui_core::Element;
use agent_tui_core::vom::Component;
use std::cell::RefCell;

use crate::repository::SessionOps;
use crate::session::SessionError;

/// Mock session for testing use cases.
///
/// Implements SessionOps with configurable behavior for testing happy paths
/// and error scenarios.
pub struct MockSession {
    /// Session identifier.
    pub id: String,
    /// Screen text to return from screen_text().
    screen_text: String,
    /// Elements to return from detect_elements().
    elements: Vec<Element>,
    /// Components to return from analyze_screen().
    components: Vec<Component>,
    /// Error to return from update(), if any.
    update_error: Option<SessionError>,
    /// Error to return from pty_write(), if any.
    pty_write_error: Option<SessionError>,
    /// Track bytes written via pty_write().
    written_data: RefCell<Vec<Vec<u8>>>,
}

impl MockSession {
    /// Create a new MockSession with default values.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            screen_text: String::new(),
            elements: Vec::new(),
            components: Vec::new(),
            update_error: None,
            pty_write_error: None,
            written_data: RefCell::new(Vec::new()),
        }
    }

    /// Create a builder for configuring MockSession.
    pub fn builder(id: impl Into<String>) -> MockSessionBuilder {
        MockSessionBuilder::new(id)
    }

    /// Get the data written via pty_write() calls.
    pub fn written_data(&self) -> Vec<Vec<u8>> {
        self.written_data.borrow().clone()
    }
}

impl SessionOps for MockSession {
    fn update(&mut self) -> Result<(), SessionError> {
        if let Some(ref err) = self.update_error {
            // Clone the error message for return
            Err(SessionError::Pty(agent_tui_terminal::PtyError::Write(
                err.to_string(),
            )))
        } else {
            Ok(())
        }
    }

    fn screen_text(&self) -> String {
        self.screen_text.clone()
    }

    fn detect_elements(&mut self) -> &[Element] {
        &self.elements
    }

    fn find_element(&self, element_ref: &str) -> Option<&Element> {
        self.elements.iter().find(|e| e.element_ref == element_ref)
    }

    fn pty_write(&mut self, data: &[u8]) -> Result<(), SessionError> {
        if let Some(ref err) = self.pty_write_error {
            Err(SessionError::Pty(agent_tui_terminal::PtyError::Write(
                err.to_string(),
            )))
        } else {
            self.written_data.borrow_mut().push(data.to_vec());
            Ok(())
        }
    }

    fn analyze_screen(&self) -> Vec<Component> {
        self.components.clone()
    }
}

/// Builder for MockSession with fluent configuration.
pub struct MockSessionBuilder {
    session: MockSession,
}

impl MockSessionBuilder {
    /// Create a new builder.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            session: MockSession::new(id),
        }
    }

    /// Configure the screen text to return.
    pub fn with_screen_text(mut self, text: impl Into<String>) -> Self {
        self.session.screen_text = text.into();
        self
    }

    /// Configure the elements to return from detect_elements().
    pub fn with_elements(mut self, elements: Vec<Element>) -> Self {
        self.session.elements = elements;
        self
    }

    /// Configure the components to return from analyze_screen().
    pub fn with_components(mut self, components: Vec<Component>) -> Self {
        self.session.components = components;
        self
    }

    /// Configure update() to return an error.
    pub fn with_update_error(mut self, error: SessionError) -> Self {
        self.session.update_error = Some(error);
        self
    }

    /// Configure pty_write() to return an error.
    pub fn with_pty_write_error(mut self, error: SessionError) -> Self {
        self.session.pty_write_error = Some(error);
        self
    }

    /// Build the configured MockSession.
    pub fn build(self) -> MockSession {
        self.session
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_tui_core::{ElementType, Position};

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
        let mut session = MockSession::builder("test").with_elements(elements).build();

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
        let mut session = MockSession::new("test");
        let result = session.update();
        assert!(result.is_ok());
    }

    #[test]
    fn test_mock_session_update_with_error() {
        let mut session = MockSession::builder("test")
            .with_update_error(SessionError::NoActiveSession)
            .build();

        let result = session.update();
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_session_pty_write_tracks_data() {
        let mut session = MockSession::new("test");

        session.pty_write(b"hello").unwrap();
        session.pty_write(b"world").unwrap();

        let written = session.written_data();
        assert_eq!(written.len(), 2);
        assert_eq!(written[0], b"hello");
        assert_eq!(written[1], b"world");
    }

    #[test]
    fn test_mock_session_analyze_screen_returns_components() {
        use agent_tui_core::vom::{Rect, Role};

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
