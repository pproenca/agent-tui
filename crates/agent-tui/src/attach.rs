use std::io;
use std::io::Write;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use crossterm::terminal;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use serde_json::json;

use crate::common::Colors;
use crate::ipc::ClientError;
use crate::ipc::DaemonClient;

pub use crate::error::AttachError;

/// RAII guard that ignores a signal during its lifetime and restores default behavior on drop.
struct SignalGuard {
    signal: libc::c_int,
}

impl SignalGuard {
    /// Creates a new SignalGuard that ignores the specified signal.
    ///
    /// # Safety
    /// Uses libc signal handling which is inherently unsafe.
    fn new(signal: libc::c_int) -> Self {
        unsafe {
            libc::signal(signal, libc::SIG_IGN);
        }
        Self { signal }
    }
}

impl Drop for SignalGuard {
    fn drop(&mut self) {
        unsafe {
            libc::signal(self.signal, libc::SIG_DFL);
        }
    }
}

pub fn attach_ipc<C: DaemonClient>(client: &mut C, session_id: &str) -> Result<(), AttachError> {
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

    // Ignore SIGQUIT (Ctrl+\) so we can capture it for detachment
    let _sigquit_guard = SignalGuard::new(libc::SIGQUIT);

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

fn attach_ipc_loop<C: DaemonClient>(client: &mut C, session_id: &str) -> Result<(), AttachError> {
    let mut stdout = io::stdout();

    loop {
        if event::poll(Duration::from_millis(10)).unwrap_or(false) {
            match event::read() {
                Ok(Event::Key(key_event)) => {
                    // Detect Ctrl+\ for detachment. Handle multiple representations:
                    // - KeyCode::Char('\\') with CONTROL modifier (standard)
                    // - KeyCode::Char('\x1c') (raw ASCII 28 = FS, which is Ctrl+\)
                    let is_ctrl_backslash = key_event.modifiers.contains(KeyModifiers::CONTROL)
                        && key_event.code == KeyCode::Char('\\');
                    let is_raw_ctrl_backslash = key_event.code == KeyCode::Char('\x1c');

                    if is_ctrl_backslash || is_raw_ctrl_backslash {
                        break;
                    }

                    if let Some(bytes) = key_event_to_bytes(&key_event) {
                        let data_b64 = STANDARD.encode(&bytes);
                        let params = json!({
                            "session": session_id,
                            "data": data_b64
                        });
                        if let Err(e) = client.call("pty_write", Some(params)) {
                            return Err(AttachError::PtyWrite(format_client_error(&e)));
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
                Err(_) => return Err(AttachError::EventRead),
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
            Err(e) => {
                return Err(AttachError::PtyRead(format_client_error(&e)));
            }
        }
    }

    Ok(())
}

fn format_client_error(error: &ClientError) -> String {
    let mut msg = error.to_string();
    if let Some(suggestion) = error.suggestion() {
        msg.push_str(&format!(" ({})", suggestion));
    }
    msg
}

fn key_event_to_bytes(key_event: &event::KeyEvent) -> Option<Vec<u8>> {
    use KeyCode::*;

    let ctrl = key_event.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key_event.modifiers.contains(KeyModifiers::ALT);

    match key_event.code {
        Char(c) => {
            if ctrl {
                if c.is_ascii_lowercase() {
                    Some(vec![c as u8 - b'a' + 1])
                } else if c.is_ascii_uppercase() {
                    Some(vec![c as u8 - b'A' + 1])
                } else {
                    match c {
                        '[' | '3' => Some(vec![0x1b]),
                        '\\' | '4' => Some(vec![0x1c]),
                        ']' | '5' => Some(vec![0x1d]),
                        '^' | '6' => Some(vec![0x1e]),
                        '_' | '7' => Some(vec![0x1f]),
                        '?' | '8' => Some(vec![0x7f]),
                        ' ' | '2' | '@' => Some(vec![0x00]),
                        _ => None,
                    }
                }
            } else if alt {
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
                Some(vec![0x1b, b'[', b'Z'])
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
        assert_eq!(key_event_to_bytes(&event), Some(vec![0x03]));
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
