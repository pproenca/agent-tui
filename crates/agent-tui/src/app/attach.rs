use std::io;
use std::io::Write;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use crossterm::execute;
use crossterm::terminal;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use serde_json::json;

use crate::common::Colors;
use crate::infra::ipc::ClientError;
use crate::infra::ipc::DaemonClient;

pub use crate::app::error::AttachError;

/// RAII guard that ignores a signal during its lifetime and restores default behavior on drop.
///
/// # Thread Safety
/// Signal handlers are process-global state, not thread-local. Creating multiple `SignalGuard`
/// instances for the same signal from different threads leads to undefined behavior. This type
/// should only be used from a single thread (typically the main thread) when controlling signal
/// disposition for interactive terminal sessions.
#[must_use = "SignalGuard must be held for the duration of signal ignoring; dropping it restores the default handler"]
struct SignalGuard {
    signal: libc::c_int,
}

impl SignalGuard {
    /// Creates a new SignalGuard that ignores the specified signal.
    fn new(signal: libc::c_int) -> Self {
        // SAFETY: `libc::signal` is safe to call with any valid signal number and SIG_IGN.
        // SIG_IGN is a valid signal disposition. The previous handler is intentionally
        // discarded since we restore SIG_DFL on drop, which is the correct default for
        // most signals. This matches POSIX semantics where ignoring signal return values
        // is acceptable when setting to SIG_IGN.
        unsafe {
            libc::signal(signal, libc::SIG_IGN);
        }
        Self { signal }
    }
}

impl Drop for SignalGuard {
    fn drop(&mut self) {
        // SAFETY: `libc::signal` is safe to call with any valid signal number and SIG_DFL.
        // SIG_DFL is the default disposition for signals. Restoring to SIG_DFL is always
        // safe and ensures the process returns to normal signal handling behavior.
        unsafe {
            libc::signal(self.signal, libc::SIG_DFL);
        }
    }
}

/// Restores terminal state on drop to avoid leaving the user's shell in a broken mode.
#[must_use = "TerminalGuard must be held for the duration of the attach session"]
struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Result<Self, AttachError> {
        enable_raw_mode().map_err(AttachError::Terminal)?;
        let mut stdout = io::stdout();
        if let Err(err) = execute!(stdout, terminal::EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(AttachError::Terminal(err));
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, terminal::LeaveAlternateScreen);
        let _ = stdout.write_all(b"\x1b[0m\x1b(B\x1b[?25h\x1b[?1l\x1b>");
        let _ = stdout.flush();
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

    let term_guard = TerminalGuard::new()?;

    let (cols, rows) = terminal::size().map_err(AttachError::Terminal)?;
    let resize_params = json!({
        "cols": cols,
        "rows": rows,
        "session": session_id
    });
    let _ = client.call("resize", Some(resize_params));

    let mut stdout = io::stdout();
    render_initial_screen(client, session_id, &mut stdout);

    let result = attach_ipc_loop(client, session_id);

    drop(term_guard);

    eprintln!();
    eprintln!(
        "{} Detached from session {}",
        Colors::dim("[attach]"),
        Colors::session_id(session_id)
    );

    result
}

fn render_initial_screen<C: DaemonClient>(
    client: &mut C,
    session_id: &str,
    stdout: &mut impl Write,
) {
    let params = json!({
        "session": session_id,
        "include_cursor": true,
        "strip_ansi": true
    });

    let snapshot = match client.call("snapshot", Some(params)) {
        Ok(snapshot) => snapshot,
        Err(_) => return,
    };

    let screen = match snapshot.get("screen").and_then(serde_json::Value::as_str) {
        Some(screen) => screen,
        None => return,
    };

    if screen.is_empty() {
        return;
    }

    if stdout.write_all(b"\x1b[2J\x1b[H").is_err() {
        return;
    }

    if stdout.write_all(screen.as_bytes()).is_err() {
        return;
    }

    if let Some(cursor) = snapshot.get("cursor") {
        let row = cursor
            .get("row")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            .saturating_add(1);
        let col = cursor
            .get("col")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            .saturating_add(1);
        let visible = cursor
            .get("visible")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);

        let _ = stdout.write_all(format!("\x1b[{row};{col}H").as_bytes());
        let _ = stdout.write_all(if visible { b"\x1b[?25h" } else { b"\x1b[?25l" });
    }

    let _ = stdout.flush();
}

fn attach_ipc_loop<C: DaemonClient>(client: &mut C, session_id: &str) -> Result<(), AttachError> {
    let mut stdout = io::stdout();

    loop {
        if event::poll(Duration::from_millis(10)).unwrap_or(false) {
            match event::read() {
                Ok(Event::Key(key_event)) => {
                    if let Some(bytes) = key_event_to_bytes(&key_event) {
                        // Detect Ctrl+\ (ASCII FS / 0x1c) across terminal encodings.
                        if bytes == [0x1c] {
                            break;
                        }

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
    use crate::infra::ipc::MockClient;
    use serde_json::json;

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

    #[test]
    fn test_render_initial_screen_writes_snapshot() {
        let mut client = MockClient::new_strict();
        client.set_response(
            "snapshot",
            json!({
                "screen": "hello\nworld",
                "cursor": { "row": 1, "col": 2, "visible": true }
            }),
        );

        let mut buffer = Vec::new();
        render_initial_screen(&mut client, "sess1", &mut buffer);

        let output = String::from_utf8_lossy(&buffer);
        assert!(output.contains("\x1b[2J\x1b[H"));
        assert!(output.contains("hello\nworld"));
        assert!(output.contains("\x1b[2;3H"));
        assert!(output.contains("\x1b[?25h"));

        assert_eq!(client.call_count("snapshot"), 1);
        let mut params = client.params_for("snapshot");
        assert_eq!(params.len(), 1);
        let params = params.pop().unwrap().unwrap();
        assert_eq!(params["session"], "sess1");
        assert_eq!(params["include_cursor"], true);
        assert_eq!(params["strip_ansi"], true);
    }
}
