use super::*;
use crate::domain::session_types::TerminalSize;
use crate::infra::terminal::render::render_screen;
use crate::infra::terminal::render::render_screen_trimmed;
use insta::assert_debug_snapshot;

#[derive(Debug)]
struct TerminalRenderState {
    size: TerminalSize,
    cursor: CursorPosition,
    plain_text: String,
    rendered: String,
    compact_rendered: String,
}

fn capture_render_state(term: &VirtualTerminal) -> TerminalRenderState {
    let buffer = term.screen_buffer();
    TerminalRenderState {
        size: term.size(),
        cursor: term.cursor(),
        plain_text: term.screen_text(),
        rendered: render_screen(&buffer).escape_debug().to_string(),
        compact_rendered: render_screen_trimmed(&buffer).escape_debug().to_string(),
    }
}

#[test]
fn test_basic_terminal() {
    let mut term = VirtualTerminal::new(TerminalSize::default());
    term.process(b"Hello, World!");
    let text = term.screen_text();
    assert!(text.contains("Hello, World!"));
}

#[test]
fn test_cursor_position() {
    let mut term = VirtualTerminal::new(TerminalSize::default());
    term.process(b"ABC");
    let cursor = term.cursor();
    assert_eq!(cursor.col, 3);
    assert_eq!(cursor.row, 0);
}

#[test]
fn test_screen_buffer() {
    let mut term = VirtualTerminal::new(TerminalSize::default());
    term.process(b"\x1b[1mBold\x1b[0m Normal");
    let buffer = term.screen_buffer();

    assert!(buffer.cells[0][0].style.bold);
    assert_eq!(buffer.cells[0][0].char, 'B');
}

#[test]
fn test_resize_reflow_snapshot() {
    let mut term = VirtualTerminal::new(TerminalSize::try_new(10, 4).expect("valid terminal size"));
    term.process(b"wrap me\nagain");
    let before = capture_render_state(&term);

    term.resize(TerminalSize::try_new(14, 4).expect("valid terminal size"));
    let after = capture_render_state(&term);

    assert_debug_snapshot!("virtual_terminal_resize_reflow", (before, after));
}
