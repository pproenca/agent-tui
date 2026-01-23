use std::thread;
use std::time::Duration;

use crate::ansi_keys;
use crate::error::SessionError;
use crate::repository::SessionOps;

pub fn navigate_to_option<S: SessionOps>(
    sess: &mut S,
    target: &str,
    screen_text: &str,
) -> Result<(), SessionError> {
    let (options, current_idx) = parse_select_options(screen_text);

    let target_lower = target.to_lowercase();
    let target_idx = options
        .iter()
        .position(|opt| opt.to_lowercase().contains(&target_lower))
        .unwrap_or(0);

    let steps = target_idx as i32 - current_idx as i32;
    let key = if steps > 0 {
        ansi_keys::DOWN
    } else {
        ansi_keys::UP
    };

    for _ in 0..steps.unsigned_abs() {
        sess.pty_write(key)?;
        thread::sleep(Duration::from_millis(30));
    }

    Ok(())
}

pub fn parse_select_options(screen_text: &str) -> (Vec<String>, usize) {
    let mut options = Vec::new();
    let mut selected_idx = 0;

    for line in screen_text.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('❯') || trimmed.starts_with('›') {
            selected_idx = options.len();
            options.push(trimmed.trim_start_matches(['❯', '›', ' ']).to_string());
        } else if trimmed.starts_with('◉') {
            selected_idx = options.len();
            options.push(trimmed.trim_start_matches(['◉', ' ']).to_string());
        } else if trimmed.starts_with('◯') {
            options.push(trimmed.trim_start_matches(['◯', ' ']).to_string());
        } else if trimmed.starts_with('>') && !trimmed.starts_with(">>") {
            selected_idx = options.len();
            options.push(trimmed.trim_start_matches(['>', ' ']).to_string());
        }
    }

    (options, selected_idx)
}

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
