use std::io;
use std::io::Read;
use std::io::Write;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::adapters::RpcValue;
use crate::adapters::rpc::params;
use crate::app::rpc_client::{RpcStream, call_stream_with_params, call_with_params};
use crate::common::Colors;
use crate::infra::ipc::ClientError;
use crate::infra::ipc::DaemonClient;
use crate::infra::terminal::key_to_escape_sequence;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use crossbeam_channel as channel;
use crossterm::cursor;
use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use crossterm::execute;
use crossterm::queue;
use crossterm::style;
use crossterm::terminal;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;

pub use crate::app::error::AttachError;

/// Restores terminal state on drop to avoid leaving the user's shell in a broken mode.
#[must_use = "TerminalGuard must be held for the duration of the attach session"]
struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Result<Self, AttachError> {
        enable_raw_mode().map_err(AttachError::Terminal)?;
        let mut stdout = io::stdout();
        let _ = prepare_terminal(&mut stdout);
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

#[derive(Debug, Clone)]
pub struct DetachKeys {
    sequence: Vec<u8>,
    display: String,
}

impl DetachKeys {
    pub fn is_disabled(&self) -> bool {
        self.sequence.is_empty()
    }

    pub fn bytes(&self) -> &[u8] {
        &self.sequence
    }

    pub fn display(&self) -> &str {
        &self.display
    }

    fn disabled() -> Self {
        Self {
            sequence: Vec::new(),
            display: "disabled".to_string(),
        }
    }
}

impl Default for DetachKeys {
    fn default() -> Self {
        Self {
            sequence: vec![0x10, 0x11],
            display: "Ctrl-P Ctrl-Q".to_string(),
        }
    }
}

impl FromStr for DetachKeys {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err("detach keys cannot be empty".to_string());
        }

        if trimmed.eq_ignore_ascii_case("none") {
            return Ok(Self::disabled());
        }

        let tokens: Vec<&str> = trimmed
            .split(',')
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .collect();

        if tokens.is_empty() {
            return Err("detach keys cannot be empty".to_string());
        }

        let mut sequence = Vec::with_capacity(tokens.len());
        let mut display_tokens = Vec::with_capacity(tokens.len());
        for token in tokens {
            let (byte, display) = parse_detach_key_token(token)?;
            sequence.push(byte);
            display_tokens.push(display);
        }

        Ok(Self {
            sequence,
            display: display_tokens.join(" "),
        })
    }
}

pub fn attach_ipc<C: DaemonClient>(
    client: &mut C,
    session_id: &str,
    mode: AttachMode,
    detach_keys: DetachKeys,
) -> Result<(), AttachError> {
    eprintln!(
        "{} Attaching to session {}...",
        Colors::dim("[attach]"),
        Colors::session_id(session_id)
    );

    match mode {
        AttachMode::Tty => {
            if detach_keys.is_disabled() {
                eprintln!(
                    "{} Detach keys disabled (use --detach-keys to enable).",
                    Colors::success("Connected!")
                );
            } else {
                eprintln!(
                    "{} Press {} to detach.",
                    Colors::success("Connected!"),
                    Colors::bold(detach_keys.display())
                );
            }
            eprintln!();

            let term_guard = TerminalGuard::new()?;

            let (cols, rows) = terminal::size().map_err(AttachError::Terminal)?;
            let resize_params = params::ResizeParams {
                cols,
                rows,
                session: Some(session_id.to_string()),
            };
            let _ = call_with_params(client, "resize", resize_params);

            let stdout = Arc::new(Mutex::new(io::stdout()));
            if let Ok(mut guard) = stdout.lock() {
                render_initial_screen(client, session_id, &mut *guard);
            }

            let result = attach_ipc_loop(client, session_id, &detach_keys, stdout);

            drop(term_guard);

            result
        }
        AttachMode::Stream => {
            let stdout = Arc::new(Mutex::new(io::stdout()));
            attach_stream_loop(client, session_id, stdout)
        }
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
    let params = params::SnapshotParams {
        session: Some(session_id.to_string()),
        include_cursor: true,
        include_render: true,
        ..Default::default()
    };

    let snapshot = match call_with_params(client, "snapshot", params) {
        Ok(snapshot) => snapshot,
        Err(_) => return,
    };

    let rendered = snapshot.get("rendered").and_then(|v| v.as_str());
    let screenshot = snapshot.get("screenshot").and_then(|v| v.as_str());

    let screen = match rendered.or(screenshot) {
        Some(screen) => screen,
        None => return,
    };

    if screen.is_empty() {
        return;
    }

    let _ = queue!(
        stdout,
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0),
        style::SetAttribute(style::Attribute::Reset),
        style::ResetColor,
        style::Print(screen)
    );

    if let Some(cursor) = snapshot.get("cursor") {
        let row = cursor
            .get("row")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            .min(u16::MAX as u64) as u16;
        let col = cursor
            .get("col")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            .min(u16::MAX as u64) as u16;
        let visible = cursor
            .get("visible")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let _ = queue!(stdout, cursor::MoveTo(col, row));
        if visible {
            let _ = queue!(stdout, cursor::Show);
        } else {
            let _ = queue!(stdout, cursor::Hide);
        }
    }

    let _ = stdout.flush();
}

fn attach_ipc_loop<C: DaemonClient>(
    client: &mut C,
    session_id: &str,
    detach_keys: &DetachKeys,
    stdout: Arc<Mutex<io::Stdout>>,
) -> Result<(), AttachError> {
    let mut detach_detector = DetachDetector::new(detach_keys);
    let mut hint_active = false;

    let stream = call_stream_with_params(
        client,
        "attach_stream",
        params::SessionParams {
            session: Some(session_id.to_string()),
        },
    )
    .map_err(|e| AttachError::PtyRead(format_client_error(&e)))?;
    let abort_handle = stream.abort_handle();
    let output_done_rx = start_attach_stream_output(stream, Arc::clone(&stdout), false)?;
    let event_rx = spawn_event_reader();

    loop {
        channel::select! {
            recv(output_done_rx) -> result => {
                match result {
                    Ok(Ok(())) => break,
                    Ok(Err(err)) => return Err(err),
                    Err(_) => break,
                }
            }
            recv(event_rx) -> msg => {
                match msg {
                    Ok(EventMessage::Event(Event::Key(key_event))) => {
                        if let Some(bytes) = key_event_to_bytes(&key_event) {
                            let (to_send, detach) = detach_detector.consume(&bytes);
                            if !detach_keys.is_disabled() {
                                let now_active = detach_detector.is_partial_match();
                                if now_active != hint_active {
                                    if let Ok(mut guard) = stdout.lock() {
                                        render_detach_hint(
                                            &mut *guard,
                                            if now_active {
                                                Some(
                                                    "Detach: sequence started, press remaining keys to detach",
                                                )
                                            } else {
                                                None
                                            },
                                        );
                                    }
                                    hint_active = now_active;
                                }
                            }
                            if detach {
                                if hint_active {
                                    if let Ok(mut guard) = stdout.lock() {
                                        render_detach_hint(&mut *guard, None);
                                    }
                                }
                                if let Some(handle) = abort_handle.as_ref() {
                                    handle.abort();
                                }
                                let _ = output_done_rx.recv_timeout(Duration::from_millis(500));
                                break;
                            }

                            if !to_send.is_empty() {
                                let data_b64 = STANDARD.encode(&to_send);
                                let params = params::PtyWriteParams {
                                    session: Some(session_id.to_string()),
                                    data: data_b64,
                                };
                                if let Err(e) = call_with_params(client, "pty_write", params) {
                                    return Err(AttachError::PtyWrite(format_client_error(&e)));
                                }
                            }
                        }
                    }
                    Ok(EventMessage::Event(Event::Paste(data))) => {
                        if !data.is_empty() {
                            let data_b64 = STANDARD.encode(data.as_bytes());
                            let params = params::PtyWriteParams {
                                session: Some(session_id.to_string()),
                                data: data_b64,
                            };
                            if let Err(e) = call_with_params(client, "pty_write", params) {
                                return Err(AttachError::PtyWrite(format_client_error(&e)));
                            }
                        }
                    }
                    Ok(EventMessage::Event(Event::Resize(cols, rows))) => {
                        let params = params::ResizeParams {
                            cols,
                            rows,
                            session: Some(session_id.to_string()),
                        };
                        let _ = call_with_params(client, "resize", params);
                    }
                    Ok(EventMessage::Event(_)) => {}
                    Ok(EventMessage::Error) => return Err(AttachError::EventRead),
                    Err(_) => return Err(AttachError::EventRead),
                }
            }
        }
    }

    Ok(())
}

fn attach_stream_loop<C: DaemonClient>(
    client: &mut C,
    session_id: &str,
    stdout: Arc<Mutex<io::Stdout>>,
) -> Result<(), AttachError> {
    let stream = call_stream_with_params(
        client,
        "attach_stream",
        params::SessionParams {
            session: Some(session_id.to_string()),
        },
    )
    .map_err(|e| AttachError::PtyRead(format_client_error(&e)))?;
    let output_done_rx = start_attach_stream_output(stream, Arc::clone(&stdout), true)?;
    let stdin_rx = spawn_stdin_reader();
    let mut stdin_active = true;

    loop {
        if stdin_active {
            channel::select! {
                recv(output_done_rx) -> result => {
                    match result {
                        Ok(Ok(())) => return Ok(()),
                        Ok(Err(err)) => return Err(err),
                        Err(_) => return Ok(()),
                    }
                }
                recv(stdin_rx) -> msg => {
                    match msg {
                        Ok(StdinMessage::Data(data)) => {
                            if !data.is_empty() {
                                let data_b64 = STANDARD.encode(&data);
                                let params = params::PtyWriteParams {
                                    session: Some(session_id.to_string()),
                                    data: data_b64,
                                };
                                if let Err(e) = call_with_params(client, "pty_write", params) {
                                    return Err(AttachError::PtyWrite(format_client_error(&e)));
                                }
                            }
                        }
                        Ok(StdinMessage::Eof) | Ok(StdinMessage::Error) => {
                            stdin_active = false;
                        }
                        Err(_) => {
                            stdin_active = false;
                        }
                    }
                }
            }
        } else {
            match output_done_rx.recv() {
                Ok(Ok(())) => return Ok(()),
                Ok(Err(err)) => return Err(err),
                Err(_) => return Ok(()),
            }
        }
    }
}

fn format_client_error(error: &ClientError) -> String {
    let mut msg = error.to_string();
    if let Some(suggestion) = error.suggestion() {
        msg.push_str(&format!(" ({})", suggestion));
    }
    msg
}

fn prepare_terminal(stdout: &mut impl Write) -> io::Result<()> {
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0),
        style::SetAttribute(style::Attribute::Reset),
        style::ResetColor,
        terminal::EnableLineWrap,
        cursor::Show,
        event::DisableMouseCapture,
        event::DisableFocusChange,
        event::EnableBracketedPaste
    )?;
    stdout.flush()
}

fn reset_terminal_modes(stdout: &mut impl Write) -> io::Result<()> {
    execute!(
        stdout,
        style::SetAttribute(style::Attribute::Reset),
        style::ResetColor,
        cursor::Show,
        terminal::EnableLineWrap,
        event::DisableMouseCapture,
        event::DisableFocusChange,
        event::DisableBracketedPaste,
        terminal::LeaveAlternateScreen
    )?;
    stdout.flush()
}

#[derive(Debug)]
struct DetachDetector {
    sequence: Vec<u8>,
    matched: usize,
}

impl DetachDetector {
    fn new(detach_keys: &DetachKeys) -> Self {
        Self {
            sequence: detach_keys.bytes().to_vec(),
            matched: 0,
        }
    }

    fn is_partial_match(&self) -> bool {
        self.matched > 0
    }

    fn consume(&mut self, bytes: &[u8]) -> (Vec<u8>, bool) {
        let mut output = Vec::new();
        for &byte in bytes {
            if self.consume_byte(byte, &mut output) {
                return (output, true);
            }
        }
        (output, false)
    }

    fn consume_byte(&mut self, byte: u8, output: &mut Vec<u8>) -> bool {
        if self.sequence.is_empty() {
            output.push(byte);
            return false;
        }

        if byte == self.sequence[self.matched] {
            self.matched += 1;
            if self.matched == self.sequence.len() {
                self.matched = 0;
                return true;
            }
            return false;
        }

        if self.matched > 0 {
            output.extend_from_slice(&self.sequence[..self.matched]);
            self.matched = 0;
            output.push(byte);
            return false;
        }

        output.push(byte);
        false
    }
}

fn render_detach_hint(stdout: &mut impl Write, message: Option<&str>) {
    let (cols, rows) = match terminal::size() {
        Ok(size) => size,
        Err(_) => return,
    };
    let row = rows.saturating_sub(1);
    let mut line = message.unwrap_or("").to_string();
    let max_len = cols as usize;
    if line.len() > max_len {
        line.truncate(max_len);
    }
    if line.len() < max_len {
        let pad = max_len - line.len();
        line.reserve(pad);
        line.extend(std::iter::repeat_n(' ', pad));
    }
    let _ = queue!(
        stdout,
        cursor::SavePosition,
        cursor::MoveTo(0, row),
        style::SetAttribute(style::Attribute::Reset),
        style::ResetColor,
        style::Print(line),
        cursor::RestorePosition
    );
    let _ = stdout.flush();
}

enum StdinMessage {
    Data(Vec<u8>),
    Eof,
    Error,
}

const ATTACH_STDIN_CHANNEL_CAPACITY: usize = 64;

fn spawn_stdin_reader() -> channel::Receiver<StdinMessage> {
    let (tx, rx) = channel::bounded(ATTACH_STDIN_CHANNEL_CAPACITY);
    let span = tracing::debug_span!("attach_stdin_reader");
    let builder = std::thread::Builder::new().name("attach-stdin".to_string());
    let tx_thread = tx.clone();
    if let Err(err) = builder.spawn(move || {
        let _guard = span.enter();
        let mut stdin = io::stdin();
        let mut buf = [0u8; 4096];
        loop {
            match stdin.read(&mut buf) {
                Ok(0) => {
                    let _ = tx_thread.send(StdinMessage::Eof);
                    break;
                }
                Ok(n) => {
                    if tx_thread
                        .send(StdinMessage::Data(buf[..n].to_vec()))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => {
                    let _ = tx_thread.send(StdinMessage::Error);
                    break;
                }
            }
        }
    }) {
        tracing::warn!(error = %err, "Failed to spawn stdin reader");
        let _ = tx.send(StdinMessage::Error);
    }
    rx
}

enum EventMessage {
    Event(Event),
    Error,
}

const ATTACH_EVENT_CHANNEL_CAPACITY: usize = 256;

fn spawn_event_reader() -> channel::Receiver<EventMessage> {
    let (tx, rx) = channel::bounded(ATTACH_EVENT_CHANNEL_CAPACITY);
    let span = tracing::debug_span!("attach_event_reader");
    let builder = std::thread::Builder::new().name("attach-events".to_string());
    let tx_thread = tx.clone();
    if let Err(err) = builder.spawn(move || {
        let _guard = span.enter();
        loop {
            match event::read() {
                Ok(ev) => {
                    if tx_thread.send(EventMessage::Event(ev)).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = tx_thread.send(EventMessage::Error);
                    break;
                }
            }
        }
    }) {
        tracing::warn!(error = %err, "Failed to spawn event reader");
        let _ = tx.send(EventMessage::Error);
    }
    rx
}

enum AttachStreamEvent {
    Output { data: Vec<u8>, dropped_bytes: u64 },
    Dropped(u64),
    Closed,
}

fn parse_stream_event(value: RpcValue) -> Result<Option<AttachStreamEvent>, AttachError> {
    let event = value.get("event").and_then(|v| v.as_str());
    let Some(event) = event else {
        return Ok(None);
    };

    match event {
        "output" => {
            let data_b64 = value.get("data").and_then(|v| v.as_str()).unwrap_or("");
            if data_b64.is_empty() {
                return Ok(None);
            }
            let data = STANDARD
                .decode(data_b64)
                .map_err(|e| AttachError::PtyRead(e.to_string()))?;
            let dropped_bytes = value
                .get("dropped_bytes")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            Ok(Some(AttachStreamEvent::Output {
                data,
                dropped_bytes,
            }))
        }
        "dropped" => {
            let dropped_bytes = value
                .get("dropped_bytes")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            Ok(Some(AttachStreamEvent::Dropped(dropped_bytes)))
        }
        "closed" => Ok(Some(AttachStreamEvent::Closed)),
        "ready" | "heartbeat" => Ok(None),
        _ => Ok(None),
    }
}

fn start_attach_stream_output(
    stream: RpcStream,
    stdout: Arc<Mutex<io::Stdout>>,
    report_drops: bool,
) -> Result<channel::Receiver<Result<(), AttachError>>, AttachError> {
    let (tx, rx) = channel::bounded(1);
    let builder = std::thread::Builder::new().name("attach-stream-output".to_string());
    builder
        .spawn(move || {
            let result = stream_output_loop(stream, stdout, report_drops);
            let _ = tx.send(result);
        })
        .map_err(|err| {
            AttachError::PtyRead(format!("Failed to spawn attach output thread: {}", err))
        })?;
    Ok(rx)
}

fn stream_output_loop(
    mut stream: RpcStream,
    stdout: Arc<Mutex<io::Stdout>>,
    report_drops: bool,
) -> Result<(), AttachError> {
    loop {
        let next = stream
            .next_result()
            .map_err(|e| AttachError::PtyRead(format_client_error(&e)))?;
        let Some(value) = next else {
            return Ok(());
        };
        match parse_stream_event(value)? {
            Some(AttachStreamEvent::Output {
                data,
                dropped_bytes,
            }) => {
                if !data.is_empty() {
                    if let Ok(mut guard) = stdout.lock() {
                        guard.write_all(&data).map_err(AttachError::Terminal)?;
                        guard.flush().map_err(AttachError::Terminal)?;
                    }
                }
                if report_drops && dropped_bytes > 0 {
                    eprintln!(
                        "{} Dropped {} bytes from stream buffer.",
                        Colors::warning("[attach]"),
                        dropped_bytes
                    );
                }
            }
            Some(AttachStreamEvent::Dropped(dropped_bytes)) => {
                if report_drops && dropped_bytes > 0 {
                    eprintln!(
                        "{} Dropped {} bytes from stream buffer.",
                        Colors::warning("[attach]"),
                        dropped_bytes
                    );
                }
            }
            Some(AttachStreamEvent::Closed) => return Ok(()),
            None => {}
        }
    }
}

fn key_event_to_bytes(key_event: &event::KeyEvent) -> Option<Vec<u8>> {
    match key_event.code {
        KeyCode::Char(c) => key_char_to_bytes(c, key_event.modifiers),
        KeyCode::F(n) => {
            let key = format!("F{n}");
            key_with_modifiers_to_bytes(&key, key_event.modifiers)
        }
        KeyCode::BackTab => key_to_escape_sequence("Shift+Tab"),
        _ => {
            let base = keycode_to_name(&key_event.code)?;
            key_with_modifiers_to_bytes(base, key_event.modifiers)
        }
    }
}

fn key_char_to_bytes(c: char, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    if modifiers.contains(KeyModifiers::CONTROL) {
        let key = format_modified_char("Ctrl", c);
        return key_to_escape_sequence(&key);
    }

    if modifiers.contains(KeyModifiers::ALT) {
        let key = format_modified_char("Alt", c);
        return key_to_escape_sequence(&key);
    }

    let mut buf = [0u8; 4];
    let s = c.encode_utf8(&mut buf);
    Some(s.as_bytes().to_vec())
}

fn key_with_modifiers_to_bytes(base: &str, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    if modifiers.contains(KeyModifiers::SHIFT) && base == "Tab" {
        return key_to_escape_sequence("Shift+Tab");
    }

    if modifiers.contains(KeyModifiers::CONTROL) {
        let key = format_modified_key("Ctrl", base);
        return key_to_escape_sequence(&key);
    }

    if modifiers.contains(KeyModifiers::ALT) {
        let key = format_modified_key("Alt", base);
        return key_to_escape_sequence(&key);
    }

    key_to_escape_sequence(base)
}

fn format_modified_key(prefix: &str, base: &str) -> String {
    let mut key = String::with_capacity(prefix.len() + 1 + base.len());
    key.push_str(prefix);
    key.push('+');
    key.push_str(base);
    key
}

fn format_modified_char(prefix: &str, c: char) -> String {
    let mut key = String::with_capacity(prefix.len() + 2);
    key.push_str(prefix);
    key.push('+');
    key.push(c);
    key
}

fn keycode_to_name(code: &KeyCode) -> Option<&'static str> {
    match code {
        KeyCode::Enter => Some("Enter"),
        KeyCode::Tab => Some("Tab"),
        KeyCode::Backspace => Some("Backspace"),
        KeyCode::Delete => Some("Delete"),
        KeyCode::Esc => Some("Escape"),
        KeyCode::Up => Some("ArrowUp"),
        KeyCode::Down => Some("ArrowDown"),
        KeyCode::Right => Some("ArrowRight"),
        KeyCode::Left => Some("ArrowLeft"),
        KeyCode::Home => Some("Home"),
        KeyCode::End => Some("End"),
        KeyCode::PageUp => Some("PageUp"),
        KeyCode::PageDown => Some("PageDown"),
        KeyCode::Insert => Some("Insert"),
        _ => None,
    }
}

fn parse_detach_key_token(token: &str) -> Result<(u8, String), String> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return Err("detach keys cannot be empty".to_string());
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("ctrl-") || lower.starts_with("control-") {
        let split_pos = trimmed.find('-').unwrap_or(0);
        let rest = trimmed[split_pos + 1..].trim();
        if rest.is_empty() {
            return Err("detach keys: ctrl- requires a key (e.g. ctrl-p)".to_string());
        }

        let ch = if rest.eq_ignore_ascii_case("space") {
            ' '
        } else if rest.chars().count() == 1 {
            rest.chars().next().unwrap()
        } else {
            return Err(format!("detach keys: unsupported ctrl key '{}'", rest));
        };

        let byte = ctrl_char_to_byte(ch)
            .ok_or_else(|| format!("detach keys: unsupported ctrl key '{}'", rest))?;
        let display = format!("Ctrl-{}", display_char(ch));
        return Ok((byte, display));
    }

    if lower == "space" {
        return Ok((b' ', "Space".to_string()));
    }

    if trimmed.chars().count() == 1 {
        let ch = trimmed.chars().next().unwrap();
        if !ch.is_ascii() {
            return Err("detach keys must be ASCII".to_string());
        }
        let display = display_char(ch);
        return Ok((ch as u8, display));
    }

    Err(format!("detach keys: unsupported token '{}'", trimmed))
}

fn ctrl_char_to_byte(ch: char) -> Option<u8> {
    if ch.is_ascii_lowercase() {
        return Some(ch as u8 - b'a' + 1);
    }
    if ch.is_ascii_uppercase() {
        return Some(ch as u8 - b'A' + 1);
    }

    match ch {
        '[' => Some(0x1b),
        '\\' => Some(0x1c),
        ']' => Some(0x1d),
        '^' => Some(0x1e),
        '_' => Some(0x1f),
        '?' => Some(0x7f),
        ' ' | '@' => Some(0x00),
        _ => None,
    }
}

fn display_char(ch: char) -> String {
    if ch == ' ' {
        return "Space".to_string();
    }
    if ch.is_ascii_alphabetic() {
        return ch.to_ascii_uppercase().to_string();
    }
    ch.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::ipc::MockClient;

    #[test]
    fn test_key_event_to_bytes_char() {
        let event = event::KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&event), key_to_escape_sequence("a"));
    }

    #[test]
    fn test_key_event_to_bytes_ctrl() {
        let event = event::KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(key_event_to_bytes(&event), key_to_escape_sequence("Ctrl+C"));
    }

    #[test]
    fn test_key_event_to_bytes_enter() {
        let event = event::KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&event), key_to_escape_sequence("Enter"));
    }

    #[test]
    fn test_key_event_to_bytes_arrow() {
        let event = event::KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(
            key_event_to_bytes(&event),
            key_to_escape_sequence("ArrowUp")
        );
    }

    #[test]
    fn test_key_event_to_bytes_f1() {
        let event = event::KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
        assert_eq!(key_event_to_bytes(&event), key_to_escape_sequence("F1"));
    }

    #[test]
    fn test_render_initial_screen_writes_snapshot() {
        let mut client = MockClient::new_strict();
        client.set_response(
            "snapshot",
            serde_json::from_str(
                r#"{"screenshot":"hello\nworld","cursor":{"row":1,"col":2,"visible":true}}"#,
            )
            .unwrap(),
        );

        let mut buffer = Vec::new();
        render_initial_screen(&mut client, "sess1", &mut buffer);

        let output = String::from_utf8_lossy(&buffer);
        let mut expected_prefix = Vec::new();
        queue!(
            expected_prefix,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0),
            style::SetAttribute(style::Attribute::Reset),
            style::ResetColor
        )
        .unwrap();
        let expected_prefix = String::from_utf8_lossy(&expected_prefix);
        assert!(output.contains(expected_prefix.as_ref()));
        assert!(output.contains("hello\nworld"));
        let mut expected_cursor = Vec::new();
        queue!(expected_cursor, cursor::MoveTo(2, 1), cursor::Show).unwrap();
        let expected_cursor = String::from_utf8_lossy(&expected_cursor);
        assert!(output.contains(expected_cursor.as_ref()));

        assert_eq!(client.call_count("snapshot"), 1);
        let mut params = client.params_for("snapshot");
        assert_eq!(params.len(), 1);
        let params = params.pop().unwrap().unwrap();
        assert_eq!(params["session"], "sess1");
        assert_eq!(params["include_cursor"], true);
        assert_eq!(params["include_render"], true);
    }

    #[test]
    fn test_detach_detector_ctrl_p_ctrl_q_detaches() {
        let detach_keys = DetachKeys::default();
        let mut detector = DetachDetector::new(&detach_keys);
        let (out, detach) = detector.consume(&[0x10]);
        assert!(out.is_empty());
        assert!(!detach);

        let (out, detach) = detector.consume(&[0x11]);
        assert!(out.is_empty());
        assert!(detach);
    }

    #[test]
    fn test_detach_detector_passes_through_non_sequence() {
        let detach_keys = DetachKeys::default();
        let mut detector = DetachDetector::new(&detach_keys);
        let (out, detach) = detector.consume(b"ab");
        assert_eq!(out, b"ab");
        assert!(!detach);
    }

    #[test]
    fn test_detach_detector_ctrl_p_followed_by_key_sends_both() {
        let detach_keys = DetachKeys::default();
        let mut detector = DetachDetector::new(&detach_keys);
        let (out, detach) = detector.consume(&[0x10, b'a']);
        assert_eq!(out, vec![0x10, b'a']);
        assert!(!detach);
    }

    #[test]
    fn test_detach_detector_ctrl_p_ctrl_p_sends_two() {
        let detach_keys = DetachKeys::default();
        let mut detector = DetachDetector::new(&detach_keys);
        let (out, detach) = detector.consume(&[0x10, 0x10]);
        assert_eq!(out, vec![0x10, 0x10]);
        assert!(!detach);
    }

    #[test]
    fn test_detach_keys_from_str_default() {
        let keys = "ctrl-p,ctrl-q".parse::<DetachKeys>().unwrap();
        assert_eq!(keys.bytes(), &[0x10, 0x11]);
        assert_eq!(keys.display(), "Ctrl-P Ctrl-Q");
    }

    #[test]
    fn test_detach_keys_from_str_none() {
        let keys = "none".parse::<DetachKeys>().unwrap();
        assert!(keys.is_disabled());
    }

    #[test]
    fn test_detach_keys_invalid_token() {
        let err = "ctrl-".parse::<DetachKeys>().unwrap_err();
        assert!(err.contains("ctrl-"));
    }

    #[test]
    fn test_parse_stream_event_output() {
        let payload = STANDARD.encode(b"hello");
        let value = RpcValue::new(
            serde_json::from_str(&format!(
                r#"{{"event":"output","data":"{}","dropped_bytes":2}}"#,
                payload
            ))
            .unwrap(),
        );
        let event = parse_stream_event(value).unwrap().unwrap();
        match event {
            AttachStreamEvent::Output {
                data,
                dropped_bytes,
            } => {
                assert_eq!(data, b"hello");
                assert_eq!(dropped_bytes, 2);
            }
            _ => panic!("expected output event"),
        }
    }

    #[test]
    fn test_parse_stream_event_dropped() {
        let value = RpcValue::new(
            serde_json::from_str(r#"{"event":"dropped","dropped_bytes":128}"#).unwrap(),
        );
        let event = parse_stream_event(value).unwrap().unwrap();
        match event {
            AttachStreamEvent::Dropped(bytes) => assert_eq!(bytes, 128),
            _ => panic!("expected dropped event"),
        }
    }

    #[test]
    fn test_parse_stream_event_closed() {
        let value = RpcValue::new(serde_json::from_str(r#"{"event":"closed"}"#).unwrap());
        let event = parse_stream_event(value).unwrap().unwrap();
        match event {
            AttachStreamEvent::Closed => {}
            _ => panic!("expected closed event"),
        }
    }
}
