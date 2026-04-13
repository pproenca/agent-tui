use super::*;

#[test]
fn test_strip_ansi_removes_color_codes() {
    assert_eq!(strip_ansi_codes("\x1b[31mRed\x1b[0m"), "Red");
}

#[test]
fn test_strip_ansi_handles_sgr_sequences() {
    assert_eq!(
        strip_ansi_codes("\x1b[1;32;40mBold Green\x1b[m"),
        "Bold Green"
    );
}

#[test]
fn test_strip_ansi_handles_osc_sequences() {
    assert_eq!(strip_ansi_codes("\x1b]0;Title\x07Content"), "Content");
}

#[test]
fn test_strip_ansi_preserves_plain_text() {
    assert_eq!(strip_ansi_codes("Hello, World!"), "Hello, World!");
}

#[test]
fn test_strip_ansi_handles_cursor_movement() {
    assert_eq!(strip_ansi_codes("Line1\x1b[ALine2"), "Line1Line2");
}

#[test]
fn test_strip_ansi_handles_empty_string() {
    assert_eq!(strip_ansi_codes(""), "");
}
