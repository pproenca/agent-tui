use std::io;
use std::io::Read;
use std::io::Write;
use std::sync::mpsc;
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
use crate::infra::ipc::ClientError;
use crate::infra::ipc::DaemonClient;

pub use crate::app::error::AttachError;

/// Restores terminal state on drop to avoid leaving the user's shell in a broken mode.
#[must_use = "TerminalGuard must be held for the duration of the attach session"]
struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Result<Self, AttachError> {
        enable_raw_mode().map_err(AttachError::Terminal)?;
        let mut stdout = io::stdout();
        let _ = reset_terminal_modes(&mut stdout);
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = reset_terminal_modes(&mut stdout);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AttachMode {
    Tty,
    Stream,
}

pub fn attach_ipc<C: DaemonClient>(
    client: &mut C,
    session_id: &str,
    mode: AttachMode,
) -> Result<(), AttachError> {
    eprintln!(
        "{} Attaching to session {}...",
        Colors::dim("[attach]"),
        Colors::session_id(session_id)
    );

    match mode {
        AttachMode::Tty => {
            eprintln!(
                "{} Press {} to detach.",
                Colors::success("Connected!"),
                Colors::bold("Ctrl-P Ctrl-Q")
            );
            eprintln!();

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

            result
        }
        AttachMode::Stream => attach_stream_loop(client, session_id),
    }?;

    eprintln!();
    eprintln!(
        "{} Detached from session {}",
        Colors::dim("[attach]"),
        Colors::session_id(session_id)
    );

    Ok(())
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

    let screenshot = match snapshot
        .get("screenshot")
        .and_then(serde_json::Value::as_str)
    {
        Some(screenshot) => screenshot,
        None => return,
    };

    if screenshot.is_empty() {
        return;
    }

    if stdout.write_all(b"\x1b[2J\x1b[H").is_err() {
        return;
    }

    if stdout.write_all(screenshot.as_bytes()).is_err() {
        return;
    }

    if let Some(cursor) = snapshot.get("cursor") {
        let row = cursor
            .get("row")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            .saturating_add(1)
            .min(u16::MAX as u64) as u16;
        let col = cursor
            .get("col")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            .saturating_add(1)
            .min(u16::MAX as u64) as u16;
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
    let mut detach_detector = DetachDetector::default();

    loop {
        if event::poll(Duration::from_millis(10)).unwrap_or(false) {
            match event::read() {
                Ok(Event::Key(key_event)) => {
                    if let Some(bytes) = key_event_to_bytes(&key_event) {
                        let (to_send, detach) = detach_detector.consume(&bytes);
                        if detach {
                            break;
                        }

                        if !to_send.is_empty() {
                            let data_b64 = STANDARD.encode(&to_send);
                            let params = json!({
                                "session": session_id,
                                "data": data_b64
                            });
                            if let Err(e) = client.call("pty_write", Some(params)) {
                                return Err(AttachError::PtyWrite(format_client_error(&e)));
                            }
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

fn attach_stream_loop<C: DaemonClient>(
    client: &mut C,
    session_id: &str,
) -> Result<(), AttachError> {
    let mut stdout = io::stdout();
    let mut stdin_active = true;
    let stdin_rx = spawn_stdin_reader();

    loop {
        if stdin_active {
            loop {
                match stdin_rx.try_recv() {
                    Ok(StdinMessage::Data(data)) => {
                        if !data.is_empty() {
                            let data_b64 = STANDARD.encode(&data);
                            let params = json!({
                                "session": session_id,
                                "data": data_b64
                            });
                            if let Err(e) = client.call("pty_write", Some(params)) {
                                return Err(AttachError::PtyWrite(format_client_error(&e)));
                            }
                        }
                    }
                    Ok(StdinMessage::Eof) => {
                        stdin_active = false;
                    }
                    Ok(StdinMessage::Error) => {
                        stdin_active = false;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        stdin_active = false;
                        break;
                    }
                }
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

fn reset_terminal_modes(stdout: &mut impl Write) -> io::Result<()> {
    stdout.write_all(b"\x1b[0m\x1b(B\x1b[?25h\x1b[?7h\x1b[?6l\x1b[r\x1b[?1l\x1b>\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1015l\x1b[?2004l")?;
    stdout.flush()
}

#[derive(Default, Debug)]
struct DetachDetector {
    pending_ctrl_p: bool,
}

impl DetachDetector {
    fn consume(&mut self, bytes: &[u8]) -> (Vec<u8>, bool) {
        let mut output = Vec::new();
        for &byte in bytes {
            if self.pending_ctrl_p {
                if byte == 0x11 {
                    self.pending_ctrl_p = false;
                    return (output, true);
                }
                output.push(0x10);
                output.push(byte);
                self.pending_ctrl_p = false;
                continue;
            }

            if byte == 0x10 {
                self.pending_ctrl_p = true;
                continue;
            }

            output.push(byte);
        }
        (output, false)
    }
}

enum StdinMessage {
    Data(Vec<u8>),
    Eof,
    Error,
}

fn spawn_stdin_reader() -> mpsc::Receiver<StdinMessage> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut stdin = io::stdin();
        let mut buf = [0u8; 4096];
        loop {
            match stdin.read(&mut buf) {
                Ok(0) => {
                    let _ = tx.send(StdinMessage::Eof);
                    break;
                }
                Ok(n) => {
                    if tx.send(StdinMessage::Data(buf[..n].to_vec())).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = tx.send(StdinMessage::Error);
                    break;
                }
            }
        }
    });
    rx
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
                "screenshot": "hello\nworld",
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

    #[test]
    fn test_detach_detector_ctrl_p_ctrl_q_detaches() {
        let mut detector = DetachDetector::default();
        let (out, detach) = detector.consume(&[0x10]);
        assert!(out.is_empty());
        assert!(!detach);

        let (out, detach) = detector.consume(&[0x11]);
        assert!(out.is_empty());
        assert!(detach);
    }

    #[test]
    fn test_detach_detector_passes_through_non_sequence() {
        let mut detector = DetachDetector::default();
        let (out, detach) = detector.consume(b"ab");
        assert_eq!(out, b"ab");
        assert!(!detach);
    }

    #[test]
    fn test_detach_detector_ctrl_p_followed_by_key_sends_both() {
        let mut detector = DetachDetector::default();
        let (out, detach) = detector.consume(&[0x10, b'a']);
        assert_eq!(out, vec![0x10, b'a']);
        assert!(!detach);
    }

    #[test]
    fn test_detach_detector_ctrl_p_ctrl_p_sends_two() {
        let mut detector = DetachDetector::default();
        let (out, detach) = detector.consume(&[0x10, 0x10]);
        assert_eq!(out, vec![0x10, 0x10]);
        assert!(!detach);
    }
}
