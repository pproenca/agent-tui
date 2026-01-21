//! Native terminal attachment for interactive session control
//!
//! This module handles attaching the user's terminal directly to an agent-tui session,
//! providing a native terminal experience with direct I/O forwarding.

use crate::client::DaemonClient;
use crate::color::Colors;
use crate::session::Session;
use crate::sync_utils::mutex_lock_or_recover;
use base64::{engine::general_purpose::STANDARD, Engine};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{self, disable_raw_mode, enable_raw_mode},
};
use serde_json::json;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AttachError {
    #[error("Terminal error: {0}")]
    Terminal(#[from] io::Error),
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Session not running: {0}")]
    SessionNotRunning(String),
}

/// Synchronous attach to a session using direct PTY I/O
///
/// This provides interactive terminal access without requiring a WebSocket server.
/// Uses crossterm for raw mode and handles keyboard input/PTY output directly.
///
/// Detach with Ctrl+\ (sends SIGQUIT-like sequence)
pub fn attach_sync(session: Arc<Mutex<Session>>, session_id: &str) -> Result<(), AttachError> {
    use std::sync::atomic::{AtomicBool, Ordering};

    eprintln!(
        "{} Attaching to session {}...",
        Colors::dim("[attach]"),
        Colors::session_id(session_id)
    );
    eprintln!(
        "{} Press {} to detach.",
        Colors::success("Connected!"),
        Colors::bold("Ctrl+\\")
    );
    eprintln!();

    enable_raw_mode().map_err(AttachError::Terminal)?;

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);

    let (cols, rows) = terminal::size().map_err(AttachError::Terminal)?;
    {
        let mut sess = mutex_lock_or_recover(&session);
        let _ = sess.resize(cols, rows);
    }

    let session_for_output = Arc::clone(&session);
    let output_thread = thread::spawn(move || {
        let mut buf = [0u8; 4096];
        let mut stdout = io::stdout();

        while running_clone.load(Ordering::Relaxed) {
            let n = {
                let sess = mutex_lock_or_recover(&session_for_output);
                sess.pty_try_read(&mut buf, 50).unwrap_or_default()
            };

            if n > 0 {
                if stdout.write_all(&buf[..n]).is_err() {
                    break;
                }
                if stdout.flush().is_err() {
                    break;
                }
            }

            {
                let mut sess = mutex_lock_or_recover(&session_for_output);
                if !sess.is_running() {
                    break;
                }
            }
        }
    });

    loop {
        if event::poll(Duration::from_millis(100)).unwrap_or(false) {
            match event::read() {
                Ok(Event::Key(key_event)) => {
                    if key_event.modifiers.contains(KeyModifiers::CONTROL)
                        && key_event.code == KeyCode::Char('\\')
                    {
                        break;
                    }

                    if let Some(bytes) = key_event_to_bytes(&key_event) {
                        let sess = mutex_lock_or_recover(&session);
                        if sess.pty_write(&bytes).is_err() {
                            break;
                        }
                    }
                }
                Ok(Event::Resize(cols, rows)) => {
                    let mut sess = mutex_lock_or_recover(&session);
                    let _ = sess.resize(cols, rows);
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }

        {
            let mut sess = mutex_lock_or_recover(&session);
            if !sess.is_running() {
                eprintln!();
                eprintln!(
                    "{} Session {} exited",
                    Colors::dim("[attach]"),
                    Colors::session_id(session_id)
                );
                break;
            }
        }
    }

    running.store(false, Ordering::Relaxed);
    let _ = output_thread.join();

    let _ = disable_raw_mode();

    eprintln!();
    eprintln!(
        "{} Detached from session {}",
        Colors::dim("[attach]"),
        Colors::session_id(session_id)
    );

    Ok(())
}

/// IPC-based attach to a session using daemon pty_read/pty_write calls
///
/// This provides interactive terminal access via the daemon's IPC protocol,
/// avoiding the issue of CLI process having an empty local SessionManager.
///
/// Detach with Ctrl+\ (sends SIGQUIT-like sequence)
pub fn attach_ipc(client: &mut DaemonClient, session_id: &str) -> Result<(), AttachError> {
    eprintln!(
        "{} Attaching to session {}...",
        Colors::dim("[attach]"),
        Colors::session_id(session_id)
    );
    eprintln!(
        "{} Press {} to detach.",
        Colors::success("Connected!"),
        Colors::bold("Ctrl+\\")
    );
    eprintln!();

    enable_raw_mode().map_err(AttachError::Terminal)?;

    let (cols, rows) = terminal::size().map_err(AttachError::Terminal)?;
    let resize_params = json!({
        "cols": cols,
        "rows": rows,
        "session": session_id
    });
    let _ = client.call("resize", Some(resize_params));

    let result = attach_ipc_loop(client, session_id);

    let _ = disable_raw_mode();

    eprintln!();
    eprintln!(
        "{} Detached from session {}",
        Colors::dim("[attach]"),
        Colors::session_id(session_id)
    );

    result
}

fn attach_ipc_loop(client: &mut DaemonClient, session_id: &str) -> Result<(), AttachError> {
    let mut stdout = io::stdout();

    loop {
        if event::poll(Duration::from_millis(10)).unwrap_or(false) {
            match event::read() {
                Ok(Event::Key(key_event)) => {
                    if key_event.modifiers.contains(KeyModifiers::CONTROL)
                        && key_event.code == KeyCode::Char('\\')
                    {
                        break;
                    }

                    if let Some(bytes) = key_event_to_bytes(&key_event) {
                        let data_b64 = STANDARD.encode(&bytes);
                        let params = json!({
                            "session": session_id,
                            "data": data_b64
                        });
                        if client.call("pty_write", Some(params)).is_err() {
                            break;
                        }
                    }
                }
                Ok(Event::Resize(cols, rows)) => {
                    let params = json!({
                        "cols": cols,
                        "rows": rows,
                        "session": session_id
                    });
                    let _ = client.call("resize", Some(params));
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }

        let read_params = json!({
            "session": session_id,
            "timeout_ms": 50
        });
        match client.call("pty_read", Some(read_params)) {
            Ok(result) => {
                if let Some(data_b64) = result.get("data").and_then(serde_json::Value::as_str) {
                    if let Ok(data) = STANDARD.decode(data_b64) {
                        if !data.is_empty() {
                            if stdout.write_all(&data).is_err() {
                                break;
                            }
                            if stdout.flush().is_err() {
                                break;
                            }
                        }
                    }
                }
            }
            Err(_) => {
                break;
            }
        }
    }

    Ok(())
}

/// Convert a crossterm key event to bytes to send to the PTY
fn key_event_to_bytes(key_event: &event::KeyEvent) -> Option<Vec<u8>> {
    use KeyCode::*;

    let ctrl = key_event.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key_event.modifiers.contains(KeyModifiers::ALT);

    match key_event.code {
        Char(c) => {
            if ctrl {
                // Ctrl+A through Ctrl+Z map to 0x01-0x1A
                if c.is_ascii_lowercase() {
                    Some(vec![c as u8 - b'a' + 1])
                } else if c.is_ascii_uppercase() {
                    Some(vec![c as u8 - b'A' + 1])
                } else {
                    // Other ctrl combinations
                    match c {
                        '[' | '3' => Some(vec![0x1b]),       // Escape
                        '\\' | '4' => Some(vec![0x1c]),      // Ctrl+\ (File separator)
                        ']' | '5' => Some(vec![0x1d]),       // Ctrl+] (Group separator)
                        '^' | '6' => Some(vec![0x1e]),       // Ctrl+^ (Record separator)
                        '_' | '7' => Some(vec![0x1f]),       // Ctrl+_ (Unit separator)
                        '?' | '8' => Some(vec![0x7f]),       // Delete
                        ' ' | '2' | '@' => Some(vec![0x00]), // Ctrl+Space (NUL)
                        _ => None,
                    }
                }
            } else if alt {
                // Alt+key sends ESC followed by the key
                Some(vec![0x1b, c as u8])
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                Some(s.as_bytes().to_vec())
            }
        }
        Enter => Some(vec![b'\r']),
        Tab => {
            if key_event.modifiers.contains(KeyModifiers::SHIFT) {
                Some(vec![0x1b, b'[', b'Z']) // Shift+Tab (backtab)
            } else {
                Some(vec![b'\t'])
            }
        }
        Backspace => Some(vec![0x7f]),
        Delete => Some(vec![0x1b, b'[', b'3', b'~']),
        Esc => Some(vec![0x1b]),
        Up => Some(vec![0x1b, b'[', b'A']),
        Down => Some(vec![0x1b, b'[', b'B']),
        Right => Some(vec![0x1b, b'[', b'C']),
        Left => Some(vec![0x1b, b'[', b'D']),
        Home => Some(vec![0x1b, b'[', b'H']),
        End => Some(vec![0x1b, b'[', b'F']),
        PageUp => Some(vec![0x1b, b'[', b'5', b'~']),
        PageDown => Some(vec![0x1b, b'[', b'6', b'~']),
        Insert => Some(vec![0x1b, b'[', b'2', b'~']),
        F(n) => {
            // Function keys
            let seq = match n {
                1 => vec![0x1b, b'O', b'P'],
                2 => vec![0x1b, b'O', b'Q'],
                3 => vec![0x1b, b'O', b'R'],
                4 => vec![0x1b, b'O', b'S'],
                5 => vec![0x1b, b'[', b'1', b'5', b'~'],
                6 => vec![0x1b, b'[', b'1', b'7', b'~'],
                7 => vec![0x1b, b'[', b'1', b'8', b'~'],
                8 => vec![0x1b, b'[', b'1', b'9', b'~'],
                9 => vec![0x1b, b'[', b'2', b'0', b'~'],
                10 => vec![0x1b, b'[', b'2', b'1', b'~'],
                11 => vec![0x1b, b'[', b'2', b'3', b'~'],
                12 => vec![0x1b, b'[', b'2', b'4', b'~'],
                _ => return None,
            };
            Some(seq)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_event_to_bytes_char() {
        let event = event::KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&event), Some(vec![b'a']));
    }

    #[test]
    fn test_key_event_to_bytes_ctrl() {
        let event = event::KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(key_event_to_bytes(&event), Some(vec![0x03])); // Ctrl+C
    }

    #[test]
    fn test_key_event_to_bytes_enter() {
        let event = event::KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&event), Some(vec![b'\r']));
    }

    #[test]
    fn test_key_event_to_bytes_arrow() {
        let event = event::KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&event), Some(vec![0x1b, b'[', b'A']));
    }

    #[test]
    fn test_key_event_to_bytes_f1() {
        let event = event::KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&event), Some(vec![0x1b, b'O', b'P']));
    }
}
