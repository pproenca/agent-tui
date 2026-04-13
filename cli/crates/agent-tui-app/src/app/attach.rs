#![expect(clippy::print_stderr, reason = "CLI/TUI output after terminal restore")]

//! Attach command implementation.

use std::io;
use std::io::Read;
use std::io::Write;
use std::panic;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use crate::adapters::RpcValue;
use crate::adapters::rpc::params;
use crate::app::rpc_client::RpcStream;
use crate::app::rpc_client::call_stream_with_params;
use crate::app::rpc_client::call_with_params;
use crate::common::Colors;
use crate::domain::session_types::TerminalSize;
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
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use crossterm::execute;
use crossterm::queue;
use crossterm::style;
use crossterm::terminal;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;

pub use crate::app::error::AttachError;

type PanicHook = Box<dyn Fn(&panic::PanicHookInfo<'_>) + Sync + Send + 'static>;
type SharedPanicHook = Arc<Mutex<Option<PanicHook>>>;

/// Restores terminal state on drop to avoid leaving the user's shell in a broken mode.
#[must_use = "TerminalGuard must be held for the duration of the attach session"]
struct TerminalGuard {
    panic_hook_guard: TerminalPanicHookGuard,
}

impl TerminalGuard {
    fn new() -> Result<Self, AttachError> {
        enable_raw_mode().map_err(AttachError::Terminal)?;
        let panic_hook_guard = TerminalPanicHookGuard::install();
        let mut stdout = io::stdout();
        prepare_terminal_with_rollback(&mut stdout, prepare_terminal)
            .map_err(AttachError::Terminal)?;
        Ok(Self { panic_hook_guard })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal_silently();
    }
}

#[must_use = "TerminalPanicHookGuard must be held for the duration of the attach session"]
struct TerminalPanicHookGuard {
    previous_hook: Option<SharedPanicHook>,
}

impl TerminalPanicHookGuard {
    fn install() -> Self {
        Self {
            previous_hook: Some(install_terminal_panic_hook()),
        }
    }

    fn restore(&mut self) {
        if thread::panicking() {
            return;
        }

        let Some(previous_hook) = self.previous_hook.take() else {
            return;
        };

        let current_hook = panic::take_hook();
        drop(current_hook);

        let previous_hook = {
            let mut previous_hook_guard = previous_hook
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            previous_hook_guard.take()
        };

        if let Some(previous_hook) = previous_hook {
            panic::set_hook(previous_hook);
        }
    }

    #[cfg(test)]
    fn has_previous_hook(&self) -> bool {
        self.previous_hook.is_some()
    }
}

impl Drop for TerminalPanicHookGuard {
    fn drop(&mut self) {
        self.restore();
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
            sequence: vec![0x10, 0x02],
            display: "Ctrl-P Ctrl-B".to_string(),
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
            let stdout = Arc::new(Mutex::new(io::stdout()));

            let initial_resize_warning = match terminal::size().map_err(AttachError::Terminal) {
                Ok((cols, rows)) => TerminalSize::try_new(cols, rows).ok().and_then(|size| {
                    sync_attach_resize(client, session_id, size)
                        .err()
                        .map(|error| attach_resize_warning(&error))
                }),
                Err(error) => return Err(error),
            };
            if let Ok(mut guard) = stdout.lock() {
                render_initial_screen(client, session_id, &mut *guard);
                if let Some(message) = initial_resize_warning.as_deref() {
                    render_status_line(&mut *guard, Some(message));
                }
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
    let mut paste_burst = PasteBurstState::default();
    let mut hint_active = false;

    let stream = call_stream_with_params(
        client,
        "attach_stream",
        params::SessionParams {
            session: Some(session_id.to_string()),
        },
    )
    .map_err(|e| AttachError::PtyRead(format_client_error(&e)))?;
    let mut output_worker = start_attach_stream_output(stream, Arc::clone(&stdout), false)?;
    let event_worker = spawn_event_reader();
    let paste_flush_tick = channel::tick(ATTACH_PASTE_BURST_CHAR_INTERVAL);

    loop {
        channel::select! {
            recv(output_worker.receiver()) -> result => {
                match result {
                    Ok(Ok(())) => break,
                    Ok(Err(err)) => return Err(err),
                    Err(_) => break,
                }
            }
            recv(paste_flush_tick) -> _ => {
                if let Some(input) = paste_burst.flush_ready(Instant::now())
                    && process_attach_input(
                        client,
                        session_id,
                        detach_keys,
                        &mut detach_detector,
                        &mut hint_active,
                        &stdout,
                        input,
                    )?
                {
                    finish_detach(&mut output_worker, &event_worker);
                    break;
                }
            }
            recv(event_worker.receiver()) -> msg => {
                match msg {
                    Ok(EventMessage::Event(Event::Key(key_event))) => {
                        let now = Instant::now();
                        if let Some(character) = plain_char_for_paste_burst(&key_event) {
                            if let Some(input) = paste_burst.on_plain_char(character, now)
                                && process_attach_input(
                                    client,
                                    session_id,
                                    detach_keys,
                                    &mut detach_detector,
                                    &mut hint_active,
                                    &stdout,
                                    input,
                                )?
                            {
                                finish_detach(&mut output_worker, &event_worker);
                                break;
                            }
                            continue;
                        }

                        if let Some(input) = paste_burst.flush_all()
                            && process_attach_input(
                                client,
                                session_id,
                                detach_keys,
                                &mut detach_detector,
                                &mut hint_active,
                                &stdout,
                                input,
                            )?
                        {
                            finish_detach(&mut output_worker, &event_worker);
                            break;
                        }

                        if let Some(bytes) = key_event_to_bytes(&key_event)
                            && process_attach_input(
                                client,
                                session_id,
                                detach_keys,
                                &mut detach_detector,
                                &mut hint_active,
                                &stdout,
                                BufferedAttachInput::normal(bytes),
                            )?
                        {
                            finish_detach(&mut output_worker, &event_worker);
                            break;
                        }
                    }
                    Ok(EventMessage::Event(Event::Paste(data))) => {
                        if let Some(input) = paste_burst.flush_all()
                            && process_attach_input(
                                client,
                                session_id,
                                detach_keys,
                                &mut detach_detector,
                                &mut hint_active,
                                &stdout,
                                input,
                            )?
                        {
                            finish_detach(&mut output_worker, &event_worker);
                            break;
                        }

                        if !data.is_empty()
                            && process_attach_input(
                                client,
                                session_id,
                                detach_keys,
                                &mut detach_detector,
                                &mut hint_active,
                                &stdout,
                                BufferedAttachInput::bypass(data.into_bytes()),
                            )?
                        {
                            finish_detach(&mut output_worker, &event_worker);
                            break;
                        }
                    }
                    Ok(EventMessage::Event(Event::Resize(cols, rows))) => {
                        if let Some(input) = paste_burst.flush_all()
                            && process_attach_input(
                                client,
                                session_id,
                                detach_keys,
                                &mut detach_detector,
                                &mut hint_active,
                                &stdout,
                                input,
                            )?
                        {
                            finish_detach(&mut output_worker, &event_worker);
                            break;
                        }

                        if let Ok(size) = TerminalSize::try_new(cols, rows)
                            && let Err(error) = sync_attach_resize(client, session_id, size)
                        {
                            announce_attach_warning(&stdout, &attach_resize_warning(&error));
                        }
                    }
                    Ok(EventMessage::Event(_)) => {
                        if let Some(input) = paste_burst.flush_all()
                            && process_attach_input(
                                client,
                                session_id,
                                detach_keys,
                                &mut detach_detector,
                                &mut hint_active,
                                &stdout,
                                input,
                            )?
                        {
                            finish_detach(&mut output_worker, &event_worker);
                            break;
                        }
                    }
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
    let output_worker = start_attach_stream_output(stream, Arc::clone(&stdout), true)?;
    let stdin_worker = spawn_stdin_reader();
    let mut stdin_active = true;

    loop {
        if stdin_active {
            channel::select! {
                recv(output_worker.receiver()) -> result => {
                    match result {
                        Ok(Ok(())) => return Ok(()),
                        Ok(Err(err)) => return Err(err),
                        Err(_) => return Ok(()),
                    }
                }
                recv(stdin_worker.receiver()) -> msg => {
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
                            stdin_worker.cancel();
                        }
                        Err(_) => {
                            stdin_active = false;
                            stdin_worker.cancel();
                        }
                    }
                }
            }
        } else {
            match output_worker.receiver().recv() {
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
        msg.push_str(&format!(" ({suggestion})"));
    }
    msg
}

fn prepare_terminal<W: Write>(stdout: &mut W) -> io::Result<()> {
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

fn prepare_terminal_with_rollback<W, F>(stdout: &mut W, prepare: F) -> io::Result<()>
where
    W: Write,
    F: FnOnce(&mut W) -> io::Result<()>,
{
    if let Err(err) = prepare(stdout) {
        let _ = disable_raw_mode();
        let _ = reset_terminal_modes(stdout);
        return Err(err);
    }

    Ok(())
}

fn restore_terminal_silently() {
    let _ = disable_raw_mode();
    let mut stdout = io::stdout();
    let _ = reset_terminal_modes(&mut stdout);
}

fn install_terminal_panic_hook() -> SharedPanicHook {
    let previous_hook: SharedPanicHook = Arc::new(Mutex::new(Some(panic::take_hook())));
    let previous_hook_for_panic = Arc::clone(&previous_hook);
    panic::set_hook(Box::new(move |panic_info| {
        restore_terminal_silently();
        let previous_hook_guard = previous_hook_for_panic
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(previous_hook) = previous_hook_guard.as_ref() {
            previous_hook(panic_info);
        }
    }));

    previous_hook
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

    fn cancel_partial_match(&mut self) -> Vec<u8> {
        if self.matched == 0 {
            return Vec::new();
        }

        let pending = self.sequence[..self.matched].to_vec();
        self.matched = 0;
        pending
    }
}

#[derive(Debug, PartialEq, Eq)]
struct BufferedAttachInput {
    bytes: Vec<u8>,
    bypass_detach: bool,
}

impl BufferedAttachInput {
    fn normal(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            bypass_detach: false,
        }
    }

    fn bypass(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            bypass_detach: true,
        }
    }
}

#[derive(Debug, Default)]
struct PasteBurstState {
    pending_first_char: Option<(char, Instant)>,
    buffer: String,
    last_plain_char_at: Option<Instant>,
}

impl PasteBurstState {
    fn on_plain_char(&mut self, character: char, now: Instant) -> Option<BufferedAttachInput> {
        if !self.buffer.is_empty() {
            if self
                .last_plain_char_at
                .is_some_and(|last| now.duration_since(last) <= ATTACH_PASTE_BURST_CHAR_INTERVAL)
            {
                self.buffer.push(character);
                self.last_plain_char_at = Some(now);
                return None;
            }

            let flushed = self.take_buffer();
            self.pending_first_char = Some((character, now));
            return Some(flushed);
        }

        if let Some((pending, pending_at)) = self.pending_first_char {
            if now.duration_since(pending_at) <= ATTACH_PASTE_BURST_CHAR_INTERVAL {
                self.pending_first_char = None;
                self.buffer.push(pending);
                self.buffer.push(character);
                self.last_plain_char_at = Some(now);
                return None;
            }

            self.pending_first_char = Some((character, now));
            return Some(Self::single_char_input(pending));
        }

        self.pending_first_char = Some((character, now));
        None
    }

    fn flush_ready(&mut self, now: Instant) -> Option<BufferedAttachInput> {
        if !self.buffer.is_empty() {
            if self
                .last_plain_char_at
                .is_some_and(|last| now.duration_since(last) > ATTACH_PASTE_BURST_CHAR_INTERVAL)
            {
                return Some(self.take_buffer());
            }
            return None;
        }

        if let Some((pending, pending_at)) = self.pending_first_char
            && now.duration_since(pending_at) > ATTACH_PASTE_BURST_CHAR_INTERVAL
        {
            self.pending_first_char = None;
            return Some(Self::single_char_input(pending));
        }

        None
    }

    fn flush_all(&mut self) -> Option<BufferedAttachInput> {
        if !self.buffer.is_empty() {
            return Some(self.take_buffer());
        }

        self.pending_first_char
            .take()
            .map(|(pending, _)| Self::single_char_input(pending))
    }

    fn single_char_input(character: char) -> BufferedAttachInput {
        let mut buf = [0u8; 4];
        let bytes = character.encode_utf8(&mut buf).as_bytes().to_vec();
        BufferedAttachInput::normal(bytes)
    }

    fn take_buffer(&mut self) -> BufferedAttachInput {
        self.last_plain_char_at = None;
        let bytes = self.buffer.as_bytes().to_vec();
        self.buffer.clear();
        BufferedAttachInput::bypass(bytes)
    }
}

fn render_status_line(stdout: &mut impl Write, message: Option<&str>) {
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

fn sync_detach_hint(
    stdout: &Arc<Mutex<io::Stdout>>,
    detach_keys: &DetachKeys,
    detach_detector: &DetachDetector,
    hint_active: &mut bool,
) {
    if detach_keys.is_disabled() {
        return;
    }

    let now_active = detach_detector.is_partial_match();
    if now_active == *hint_active {
        return;
    }

    if let Ok(mut guard) = stdout.lock() {
        render_status_line(
            &mut *guard,
            if now_active {
                Some("Detach: sequence started, press remaining keys to detach")
            } else {
                None
            },
        );
    }
    *hint_active = now_active;
}

fn finish_detach(
    output_worker: &mut AttachOutputWorker,
    event_worker: &AttachReaderWorker<EventMessage>,
) {
    output_worker.abort();
    let _ = output_worker
        .receiver()
        .recv_timeout(ATTACH_OUTPUT_SHUTDOWN_WAIT);
    event_worker.cancel();
}

fn write_session_bytes<C: DaemonClient>(
    client: &mut C,
    session_id: &str,
    data: &[u8],
) -> Result<(), AttachError> {
    if data.is_empty() {
        return Ok(());
    }

    let data_b64 = STANDARD.encode(data);
    let params = params::PtyWriteParams {
        session: Some(session_id.to_string()),
        data: data_b64,
    };
    call_with_params(client, "pty_write", params)
        .map(|_| ())
        .map_err(|e| AttachError::PtyWrite(format_client_error(&e)))
}

fn sync_attach_resize<C: DaemonClient>(
    client: &mut C,
    session_id: &str,
    size: TerminalSize,
) -> Result<(), ClientError> {
    let params = params::ResizeParams {
        size,
        session: Some(session_id.to_string()),
    };
    call_with_params(client, "resize", params).map(|_| ())
}

fn attach_resize_warning(error: &ClientError) -> String {
    format!("Resize sync failed: {}", format_client_error(error))
}

fn announce_attach_warning(stdout: &Arc<Mutex<io::Stdout>>, message: &str) {
    tracing::warn!(warning = %message, "Attach warning");
    if let Ok(mut guard) = stdout.lock() {
        render_status_line(&mut *guard, Some(message));
    }
}

fn process_attach_input<C: DaemonClient>(
    client: &mut C,
    session_id: &str,
    detach_keys: &DetachKeys,
    detach_detector: &mut DetachDetector,
    hint_active: &mut bool,
    stdout: &Arc<Mutex<io::Stdout>>,
    input: BufferedAttachInput,
) -> Result<bool, AttachError> {
    let (to_send, detach) = if input.bypass_detach {
        let mut bytes = detach_detector.cancel_partial_match();
        bytes.extend_from_slice(&input.bytes);
        (bytes, false)
    } else {
        detach_detector.consume(&input.bytes)
    };

    sync_detach_hint(stdout, detach_keys, detach_detector, hint_active);

    if detach {
        return Ok(true);
    }

    write_session_bytes(client, session_id, &to_send)?;
    Ok(false)
}

enum StdinMessage {
    Data(Vec<u8>),
    Eof,
    Error,
}

const ATTACH_STDIN_CHANNEL_CAPACITY: usize = 64;
const ATTACH_INPUT_POLL_INTERVAL: Duration = Duration::from_millis(50);
const ATTACH_THREAD_JOIN_TIMEOUT: Duration = Duration::from_millis(500);
const ATTACH_OUTPUT_SHUTDOWN_WAIT: Duration = Duration::from_millis(500);
const ATTACH_PASTE_BURST_CHAR_INTERVAL: Duration = Duration::from_millis(8);

struct AttachReaderWorker<T> {
    rx: channel::Receiver<T>,
    cancelled: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
    name: &'static str,
}

impl<T> AttachReaderWorker<T> {
    fn receiver(&self) -> &channel::Receiver<T> {
        &self.rx
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    fn shutdown(&mut self, timeout: Duration) {
        self.cancel();
        join_thread_with_timeout(&mut self.join, timeout, self.name);
    }
}

impl<T> Drop for AttachReaderWorker<T> {
    fn drop(&mut self) {
        self.shutdown(ATTACH_THREAD_JOIN_TIMEOUT);
    }
}

struct AttachOutputWorker {
    done_rx: channel::Receiver<Result<(), AttachError>>,
    join: Option<thread::JoinHandle<()>>,
    shutdown_signal: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl AttachOutputWorker {
    fn receiver(&self) -> &channel::Receiver<Result<(), AttachError>> {
        &self.done_rx
    }

    fn abort(&mut self) {
        if let Some(shutdown_signal) = self.shutdown_signal.take() {
            shutdown_signal();
        }
    }

    fn shutdown(&mut self, timeout: Duration) {
        self.abort();
        join_thread_with_timeout(&mut self.join, timeout, "attach-stream-output");
    }
}

impl Drop for AttachOutputWorker {
    fn drop(&mut self) {
        self.shutdown(ATTACH_THREAD_JOIN_TIMEOUT);
    }
}

fn join_thread_with_timeout(
    join: &mut Option<thread::JoinHandle<()>>,
    timeout: Duration,
    name: &'static str,
) {
    let Some(handle) = join.take() else {
        return;
    };
    let deadline = Instant::now() + timeout;
    while !handle.is_finished() {
        if Instant::now() >= deadline {
            tracing::warn!(
                thread = name,
                timeout_ms = timeout.as_millis(),
                "Timed out joining attach helper thread"
            );
            return;
        }
        thread::park_timeout(Duration::from_millis(10));
    }
    let _ = handle.join();
}

fn spawn_stdin_reader() -> AttachReaderWorker<StdinMessage> {
    let (tx, rx) = channel::bounded(ATTACH_STDIN_CHANNEL_CAPACITY);
    let span = tracing::debug_span!("attach_stdin_reader");
    let builder = thread::Builder::new().name("attach-stdin".to_string());
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancelled_for_thread = Arc::clone(&cancelled);
    let tx_thread = tx.clone();
    let join = match builder.spawn(move || {
        let _guard = span.enter();
        let mut stdin = io::stdin();
        let mut buf = [0u8; 4096];
        loop {
            if cancelled_for_thread.load(Ordering::Relaxed) {
                break;
            }
            #[cfg(unix)]
            {
                match stdin_ready(ATTACH_INPUT_POLL_INTERVAL) {
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(_) => {
                        let _ = tx_thread.send(StdinMessage::Error);
                        break;
                    }
                }
            }
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
        Ok(handle) => Some(handle),
        Err(err) => {
            tracing::warn!(error = %err, "Failed to spawn stdin reader");
            let _ = tx.send(StdinMessage::Error);
            None
        }
    };
    AttachReaderWorker {
        rx,
        cancelled,
        join,
        name: "attach-stdin",
    }
}

enum EventMessage {
    Event(Event),
    Error,
}

const ATTACH_EVENT_CHANNEL_CAPACITY: usize = 256;

#[cfg(unix)]
fn stdin_ready(timeout: Duration) -> io::Result<bool> {
    use std::os::fd::AsRawFd;
    let mut fds = [libc::pollfd {
        fd: io::stdin().as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    }];
    let timeout_ms = timeout.as_millis().min(i32::MAX as u128) as i32;
    loop {
        // SAFETY: `poll` is called with a valid pointer to a stack-allocated array.
        let rc = unsafe { libc::poll(fds.as_mut_ptr(), 1, timeout_ms) };
        if rc > 0 {
            return Ok(fds[0].revents & libc::POLLIN != 0);
        }
        if rc == 0 {
            return Ok(false);
        }
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::Interrupted {
            continue;
        }
        return Err(err);
    }
}

fn spawn_event_reader() -> AttachReaderWorker<EventMessage> {
    let (tx, rx) = channel::bounded(ATTACH_EVENT_CHANNEL_CAPACITY);
    let span = tracing::debug_span!("attach_event_reader");
    let builder = thread::Builder::new().name("attach-events".to_string());
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancelled_for_thread = Arc::clone(&cancelled);
    let tx_thread = tx.clone();
    let join = match builder.spawn(move || {
        let _guard = span.enter();
        loop {
            if cancelled_for_thread.load(Ordering::Relaxed) {
                break;
            }
            match event::poll(ATTACH_INPUT_POLL_INTERVAL) {
                Ok(true) => {}
                Ok(false) => continue,
                Err(_) => {
                    let _ = tx_thread.send(EventMessage::Error);
                    break;
                }
            }
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
        Ok(handle) => Some(handle),
        Err(err) => {
            tracing::warn!(error = %err, "Failed to spawn event reader");
            let _ = tx.send(EventMessage::Error);
            None
        }
    };
    AttachReaderWorker {
        rx,
        cancelled,
        join,
        name: "attach-events",
    }
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
) -> Result<AttachOutputWorker, AttachError> {
    let (tx, rx) = channel::bounded(1);
    let shutdown_signal = stream
        .abort_handle()
        .map(|handle| Arc::new(move || handle.abort()) as Arc<dyn Fn() + Send + Sync>);
    let builder = thread::Builder::new().name("attach-stream-output".to_string());
    let join = builder
        .spawn(move || {
            let result = stream_output_loop(stream, stdout, report_drops);
            let _ = tx.send(result);
        })
        .map_err(|err| {
            AttachError::PtyRead(format!("Failed to spawn attach output thread: {err}"))
        })?;
    Ok(AttachOutputWorker {
        done_rx: rx,
        join: Some(join),
        shutdown_signal,
    })
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
                if let (true, Ok(mut guard)) = (!data.is_empty(), stdout.lock()) {
                    guard.write_all(&data).map_err(AttachError::Terminal)?;
                    guard.flush().map_err(AttachError::Terminal)?;
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

fn plain_char_for_paste_burst(key_event: &event::KeyEvent) -> Option<char> {
    if key_event.kind == KeyEventKind::Release || key_event.modifiers != KeyModifiers::NONE {
        return None;
    }

    match key_event.code {
        KeyCode::Char(character) => Some(character),
        _ => None,
    }
}

fn key_event_to_bytes(key_event: &event::KeyEvent) -> Option<Vec<u8>> {
    if key_event.kind == KeyEventKind::Release {
        return None;
    }

    match key_event.code {
        KeyCode::Char(c) => key_char_to_bytes(c, key_event.modifiers),
        KeyCode::F(n) => {
            let key = format!("F{n}");
            key_with_modifiers_to_bytes(&key, key_event.modifiers)
        }
        KeyCode::BackTab => key_to_escape_sequence("Shift+Tab"),
        _ => {
            let base = keycode_to_name(key_event.code)?;
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

fn keycode_to_name(code: KeyCode) -> Option<&'static str> {
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
        let split_pos = match trimmed.find('-') {
            Some(pos) => pos,
            None => {
                return Err("detach keys: ctrl- requires a key (e.g. ctrl-p)".to_string());
            }
        };
        let rest = trimmed[split_pos + 1..].trim();
        if rest.is_empty() {
            return Err("detach keys: ctrl- requires a key (e.g. ctrl-p)".to_string());
        }

        let ch = if rest.eq_ignore_ascii_case("space") {
            ' '
        } else {
            let mut chars = rest.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) => ch,
                _ => return Err(format!("detach keys: unsupported ctrl key '{rest}'")),
            }
        };

        let byte = ctrl_char_to_byte(ch)
            .ok_or_else(|| format!("detach keys: unsupported ctrl key '{rest}'"))?;
        let display = format!("Ctrl-{}", display_char(ch));
        return Ok((byte, display));
    }

    if lower == "space" {
        return Ok((b' ', "Space".to_string()));
    }

    let mut chars = trimmed.chars();
    if let (Some(ch), None) = (chars.next(), chars.next()) {
        if !ch.is_ascii() {
            return Err("detach keys must be ASCII".to_string());
        }
        let display = display_char(ch);
        return Ok((ch as u8, display));
    }

    Err(format!("detach keys: unsupported token '{trimmed}'"))
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
#[path = "attach_tests.rs"]
mod tests;
