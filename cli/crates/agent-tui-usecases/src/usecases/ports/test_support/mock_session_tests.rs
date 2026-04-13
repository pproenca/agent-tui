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
fn test_mock_session_with_rendered_screen() {
    let session = MockSession::builder("test")
        .with_screen_text("plain")
        .with_rendered_screen("\u{001b}[31mplain\u{001b}[0m", "plain")
        .build();

    assert_eq!(session.screen_render(), "\u{001b}[31mplain\u{001b}[0m");
    assert_eq!(session.screen_render_compact(), "plain");
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

    session
        .terminal_write(b"hello")
        .expect("first write should succeed");
    session
        .terminal_write(b"world")
        .expect("second write should succeed");

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
