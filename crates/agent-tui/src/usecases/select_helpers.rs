use std::time::Duration;

use crate::common::ansi_keys;
use crate::usecases::ports::{SessionError, SessionOps, Sleeper};

pub fn navigate_to_option<Sess: SessionOps + ?Sized, Sl: Sleeper>(
    sess: &Sess,
    sleeper: &Sl,
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
    let key = if steps > 0 { ansi_keys::DOWN } else { ansi_keys::UP };

    for _ in 0..steps.unsigned_abs() {
        sess.pty_write(key)?;
        sleeper.sleep(Duration::from_millis(30));
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
