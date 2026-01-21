//! Select widget navigation helpers

use crate::session::Session;
use std::thread;
use std::time::Duration;

/// Arrow key escape sequences
const ARROW_UP: &[u8] = b"\x1b[A";
const ARROW_DOWN: &[u8] = b"\x1b[B";

/// Navigate to a select option using arrow keys
pub fn navigate_to_option(
    sess: &mut Session,
    target: &str,
    screen_text: &str,
) -> Result<(), crate::session::SessionError> {
    let (options, current_idx) = parse_select_options(screen_text);

    // Find target option (case-insensitive partial match)
    let target_lower = target.to_lowercase();
    let target_idx = options
        .iter()
        .position(|opt| opt.to_lowercase().contains(&target_lower))
        .unwrap_or(0);

    // Calculate steps and direction
    let steps = target_idx as i32 - current_idx as i32;
    let key = if steps > 0 { ARROW_DOWN } else { ARROW_UP };

    // Send arrow keys with small delay for TUI to update
    for _ in 0..steps.unsigned_abs() {
        sess.pty_write(key)?;
        thread::sleep(Duration::from_millis(30));
    }

    Ok(())
}

/// Parse select options from screen text
/// Returns (options, currently_selected_index)
pub fn parse_select_options(screen_text: &str) -> (Vec<String>, usize) {
    let mut options = Vec::new();
    let mut selected_idx = 0;

    for line in screen_text.lines() {
        let trimmed = line.trim();

        // Ink/Inquirer selection markers: ❯ or ›
        if trimmed.starts_with('❯') || trimmed.starts_with('›') {
            selected_idx = options.len();
            options.push(trimmed.trim_start_matches(['❯', '›', ' ']).to_string());
        }
        // Inquirer radio buttons: ◉ (selected) or ◯ (unselected)
        else if trimmed.starts_with('◉') {
            selected_idx = options.len();
            options.push(trimmed.trim_start_matches(['◉', ' ']).to_string());
        } else if trimmed.starts_with('◯') {
            options.push(trimmed.trim_start_matches(['◯', ' ']).to_string());
        }
        // BubbleTea/generic: > marker (but not >>)
        else if trimmed.starts_with('>') && !trimmed.starts_with(">>") {
            selected_idx = options.len();
            options.push(trimmed.trim_start_matches(['>', ' ']).to_string());
        }
    }

    (options, selected_idx)
}

/// Strip ANSI escape codes from a string
pub fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // ESC sequence - consume until we hit the terminator
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                              // CSI sequence: consume until we hit a letter (@ through ~)
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() || next == '~' || next == '@' {
                        break;
                    }
                }
            } else if chars.peek() == Some(&']') {
                // OSC sequence: consume until BEL (\x07) or ST (\x1b\\)
                chars.next(); // consume ']'
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
                // Other escape sequences - consume one more char
                chars.next();
            }
        } else {
            result.push(c);
        }
    }
    result
}
