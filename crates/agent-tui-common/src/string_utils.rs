//! String utility functions shared across agent-tui crates.

/// Strip ANSI escape codes from a string.
///
/// Handles:
/// - SGR sequences (colors, styles): `\x1b[...m`
/// - OSC sequences (titles, etc.): `\x1b]...\x07` or `\x1b]...\x1b\\`
/// - Cursor movement and other CSI sequences
pub fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();

                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() || next == '~' || next == '@' {
                        break;
                    }
                }
            } else if chars.peek() == Some(&']') {
                chars.next();
                while let Some(&next) = chars.peek() {
                    if next == '\x07' {
                        chars.next();
                        break;
                    } else if next == '\x1b' {
                        chars.next();
                        if chars.peek() == Some(&'\\') {
                            chars.next();
                        }
                        break;
                    }
                    chars.next();
                }
            } else {
                chars.next();
            }
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
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
}
