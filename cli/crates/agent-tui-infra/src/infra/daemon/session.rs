//! Daemon session runtime.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::RwLock;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use crossterm::cursor;
use crossterm::queue;
use crossterm::style;
use crossterm::terminal;
use tracing::warn;

use bytes::Bytes;
use chrono::DateTime;
use chrono::TimeZone;
use chrono::Utc;
use crossbeam_channel as channel;
use serde::Deserialize;
use serde::Serialize;
use sysinfo::Pid;
use sysinfo::ProcessRefreshKind;
use sysinfo::ProcessesToUpdate;
use sysinfo::System;
use sysinfo::UpdateKind;
use uuid::Uuid;

use crate::common::join_thread_and_warn_on_panic;
use crate::common::mutex_lock_or_recover;
use crate::common::rwlock_read_or_recover;
use crate::common::rwlock_write_or_recover;
use crate::infra::terminal::CursorPosition;
use crate::infra::terminal::PtyHandle;
use crate::infra::terminal::ReadEvent;
use crate::infra::terminal::VirtualTerminal;
use crate::infra::terminal::key_to_escape_sequence;
use crate::infra::terminal::render_screen;
use crate::infra::terminal::render_screen_trimmed;
use crate::usecases::ports::LivePreviewSnapshot;
use crate::usecases::ports::SpawnErrorKind;
use crate::usecases::ports::StreamCursor;
use crate::usecases::ports::StreamRead;
use crate::usecases::ports::StreamWaiter;
use crate::usecases::ports::StreamWaiterHandle;

use crate::domain::RestartOutput;

pub use crate::domain::session_types::SessionId;
pub use crate::domain::session_types::SessionInfo;
use crate::domain::session_types::TerminalSize;
pub use crate::infra::daemon::SessionError;

const DEFAULT_STREAM_MAX_BUFFER_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const PUMP_FLUSH_TIMEOUT: Duration = Duration::from_millis(50);
const SESSION_QUERY_LOCK_TIMEOUT: Duration = Duration::from_millis(100);
const STARTUP_TERMINATE_TIMEOUT: Duration = Duration::from_millis(500);
const STARTUP_KILL_TIMEOUT: Duration = Duration::from_millis(500);
const STARTUP_KILL_POLL_INTERVAL: Duration = Duration::from_millis(25);
const STARTUP_PID_START_TOLERANCE_SECS: i64 = 30;

pub fn generate_session_id() -> SessionId {
    SessionId::new_unchecked(Uuid::new_v4().to_string()[..8].to_string())
}

fn spawned_process_id(pty: &mut PtyHandle) -> Result<u32, SessionError> {
    if let Some(pid) = pty.pid() {
        return Ok(pid);
    }

    let _ = pty.kill();
    Err(SessionError::Terminal(
        crate::usecases::ports::TerminalError::Spawn {
            reason: "spawned process did not expose a process ID".to_string(),
            kind: SpawnErrorKind::Other,
        },
    ))
}

#[derive(Debug, Clone)]
struct SessionLaunchSpec {
    command: String,
    args: Vec<String>,
    cwd: Option<String>,
    env: Option<HashMap<String, String>>,
}

struct StreamState {
    buffer: VecDeque<Bytes>,
    buffer_len: usize,
    base_seq: u64,
    next_seq: u64,
    dropped_bytes: u64,
    status: StreamStatus,
}

enum StreamStatus {
    Open,
    Closed,
    Failed(String),
}

enum StreamEnd {
    Closed,
    Failed(String),
}

impl StreamStatus {
    fn is_closed(&self) -> bool {
        !matches!(self, Self::Open)
    }
}

type WaitNotifiers = Arc<Mutex<Vec<(u64, channel::Sender<()>)>>>;

struct StreamBuffer {
    state: RwLock<StreamState>,
    wait_lock: Mutex<()>,
    cv: Condvar,
    notifiers: WaitNotifiers,
    next_notifier_id: AtomicU64,
    max_bytes: usize,
}

#[derive(Clone)]
pub struct StreamReader {
    inner: Arc<StreamBuffer>,
}

impl StreamReader {
    fn new(inner: Arc<StreamBuffer>) -> Self {
        Self { inner }
    }

    pub fn read(
        &self,
        cursor: &mut StreamCursor,
        max_bytes: usize,
        timeout_ms: i32,
    ) -> Result<StreamRead, SessionError> {
        self.inner.read(cursor, max_bytes, timeout_ms)
    }

    pub fn subscribe(&self) -> StreamWaiterHandle {
        self.inner.subscribe()
    }
}

struct StreamWaiterImpl {
    receiver: channel::Receiver<()>,
    notifiers: WaitNotifiers,
    notifier_id: u64,
}

impl StreamWaiter for StreamWaiterImpl {
    fn wait(&self, timeout: Option<Duration>) -> bool {
        match timeout {
            Some(timeout) => self.receiver.recv_timeout(timeout).is_ok(),
            None => self.receiver.recv().is_ok(),
        }
    }
}

impl Drop for StreamWaiterImpl {
    fn drop(&mut self) {
        let mut notifiers = self
            .notifiers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        notifiers.retain(|(id, _)| *id != self.notifier_id);
    }
}

enum PumpCommand {
    Flush(channel::Sender<()>),
    Shutdown,
}

impl StreamBuffer {
    fn new(max_bytes: usize) -> Self {
        Self {
            state: RwLock::new(StreamState {
                buffer: VecDeque::new(),
                buffer_len: 0,
                base_seq: 0,
                next_seq: 0,
                dropped_bytes: 0,
                status: StreamStatus::Open,
            }),
            wait_lock: Mutex::new(()),
            cv: Condvar::new(),
            notifiers: Arc::new(Mutex::new(Vec::new())),
            next_notifier_id: AtomicU64::new(1),
            max_bytes,
        }
    }

    #[cfg(test)]
    fn push(&self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        self.push_bytes(Bytes::copy_from_slice(data));
    }

    fn push_bytes(&self, data: Bytes) {
        if data.is_empty() {
            return;
        }
        let _wait_guard = self
            .wait_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut state = self
            .state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.buffer_len = state.buffer_len.saturating_add(data.len());
        state.next_seq = state.next_seq.saturating_add(data.len() as u64);
        state.buffer.push_back(data);

        while state.buffer_len > self.max_bytes {
            let excess = state.buffer_len - self.max_bytes;
            let Some(chunk) = state.buffer.pop_front() else {
                break;
            };
            if chunk.len() <= excess {
                let len = chunk.len();
                state.buffer_len = state.buffer_len.saturating_sub(len);
                state.base_seq = state.base_seq.saturating_add(len as u64);
                state.dropped_bytes = state.dropped_bytes.saturating_add(len as u64);
                continue;
            }

            let keep = chunk.slice(excess..);
            state.buffer.push_front(keep);
            state.buffer_len = state.buffer_len.saturating_sub(excess);
            state.base_seq = state.base_seq.saturating_add(excess as u64);
            state.dropped_bytes = state.dropped_bytes.saturating_add(excess as u64);
            break;
        }
        drop(state);
        self.notify_listeners();
        self.cv.notify_all();
    }

    fn finish(&self, end: StreamEnd) {
        let _wait_guard = self
            .wait_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut state = self
            .state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.status = match end {
            StreamEnd::Closed => StreamStatus::Closed,
            StreamEnd::Failed(error) => StreamStatus::Failed(error),
        };
        drop(state);
        self.notify_listeners();
        self.cv.notify_all();
    }

    fn notify(&self) {
        let _wait_guard = self
            .wait_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.notify_listeners();
        self.cv.notify_all();
    }

    fn subscribe(&self) -> StreamWaiterHandle {
        let (tx, rx) = channel::bounded(1);
        let notifier_id = self.next_notifier_id.fetch_add(1, Ordering::Relaxed);
        self.notify_listeners();
        {
            let mut notifiers = self
                .notifiers
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            notifiers.push((notifier_id, tx));
        }
        Arc::new(StreamWaiterImpl {
            receiver: rx,
            notifiers: Arc::clone(&self.notifiers),
            notifier_id,
        })
    }

    fn latest_seq(&self) -> u64 {
        let state = self
            .state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.next_seq
    }

    fn notify_listeners(&self) {
        let mut notifiers = self
            .notifiers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        notifiers.retain(|(_, sender)| match sender.try_send(()) {
            Ok(()) => true,
            Err(channel::TrySendError::Full(_)) => true,
            Err(channel::TrySendError::Disconnected(_)) => false,
        });
    }

    #[cfg(test)]
    fn notifier_count(&self) -> usize {
        self.notifiers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }

    fn read(
        &self,
        cursor: &mut StreamCursor,
        max_bytes: usize,
        timeout_ms: i32,
    ) -> Result<StreamRead, SessionError> {
        let max_bytes = max_bytes.max(1);
        let timeout = if timeout_ms < 0 {
            None
        } else {
            Some(Duration::from_millis(timeout_ms as u64))
        };

        let mut guard = self
            .wait_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            let state = self
                .state
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.next_seq > cursor.seq || state.status.is_closed() {
                break;
            }
            drop(state);

            if let Some(wait) = timeout {
                let (new_guard, result) = self
                    .cv
                    .wait_timeout(guard, wait)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                guard = new_guard;
                if result.timed_out() {
                    break;
                }
            } else {
                guard = self
                    .cv
                    .wait(guard)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        }
        drop(guard);

        let state = self
            .state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let StreamStatus::Failed(error) = &state.status {
            return Err(SessionError::Terminal(
                crate::usecases::ports::TerminalError::Read {
                    reason: error.clone(),
                    source: None,
                },
            ));
        }

        let latest_cursor = StreamCursor {
            seq: state.next_seq,
        };
        let closed = state.status.is_closed();
        let dropped_bytes = state.base_seq.saturating_sub(cursor.seq);

        if cursor.seq < state.base_seq {
            cursor.seq = state.base_seq;
        }

        let offset = (cursor.seq - state.base_seq) as usize;
        let available = state.buffer_len.saturating_sub(offset);
        let read_len = available.min(max_bytes);

        let mut data = Vec::with_capacity(read_len);
        if read_len > 0 {
            let mut remaining = read_len;
            let mut skip = offset;
            for chunk in state.buffer.iter() {
                if remaining == 0 {
                    break;
                }
                if skip >= chunk.len() {
                    skip -= chunk.len();
                    continue;
                }
                let start = skip;
                let take = (chunk.len() - start).min(remaining);
                data.extend_from_slice(&chunk[start..start + take]);
                remaining -= take;
                skip = 0;
            }
        }

        cursor.seq = cursor.seq.saturating_add(read_len as u64);

        Ok(StreamRead {
            data,
            next_cursor: *cursor,
            latest_cursor,
            dropped_bytes,
            closed,
        })
    }
}

fn render_live_preview_init(
    buffer: &crate::infra::terminal::ScreenBuffer,
    cursor: CursorPosition,
) -> String {
    let mut out = Vec::new();
    let _ = queue!(
        out,
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0),
        style::SetAttribute(style::Attribute::Reset),
        style::ResetColor
    );
    let body = render_screen(buffer);
    let _ = queue!(out, style::Print(body));
    let _ = queue!(out, cursor::MoveTo(cursor.col, cursor.row));
    if cursor.visible {
        let _ = queue!(out, cursor::Show);
    } else {
        let _ = queue!(out, cursor::Hide);
    }
    String::from_utf8(out).unwrap_or_default()
}

fn spawn_pump(
    session: Arc<Mutex<Session>>,
    thread_name: String,
) -> (channel::Sender<PumpCommand>, thread::JoinHandle<()>) {
    const PUMP_COMMAND_CHANNEL_CAPACITY: usize = 64;
    let (tx, rx) = channel::bounded(PUMP_COMMAND_CHANNEL_CAPACITY);
    let pty_rx = {
        let mut sess = mutex_lock_or_recover(&session);
        sess.take_pty_rx()
    }
    .unwrap_or_else(|| {
        let (_tx, rx) = channel::bounded(1);
        rx
    });
    let payload = Arc::new(Mutex::new(Some((session, pty_rx, rx))));
    let payload_for_thread = Arc::clone(&payload);
    let join = match thread::Builder::new().name(thread_name).spawn(move || {
        let Some((session, pty_rx, rx)) = payload_for_thread
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
        else {
            warn!("Session pump payload missing; pump thread exiting");
            return;
        };
        pump_loop(session, pty_rx, rx);
    }) {
        Ok(handle) => handle,
        Err(err) => {
            warn!(
                error = %err,
                "Failed to spawn named session pump thread; falling back to unnamed thread"
            );
            match payload
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take()
            {
                Some((session, pty_rx, rx)) => {
                    thread::spawn(move || pump_loop(session, pty_rx, rx))
                }
                None => thread::spawn(|| {}),
            }
        }
    };
    (tx, join)
}

fn pump_loop(
    session: Arc<Mutex<Session>>,
    pty_rx: channel::Receiver<ReadEvent>,
    rx: channel::Receiver<PumpCommand>,
) {
    loop {
        channel::select! {
            recv(rx) -> cmd => match cmd {
                Ok(PumpCommand::Flush(ack)) => {
                    let mut should_continue = true;
                    if let Ok(mut sess) = session.lock() {
                        should_continue = sess.pump_drain_events(&pty_rx);
                    }
                    let _ = ack.send(());
                    if !should_continue {
                        return;
                    }
                }
                Ok(PumpCommand::Shutdown) | Err(_) => {
                    if let Ok(sess) = session.lock() {
                        sess.stream.finish(StreamEnd::Closed);
                    }
                    return;
                }
            },
            recv(pty_rx) -> event => match event {
                Ok(event) => {
                    let mut should_continue = true;
                    if let Ok(mut sess) = session.lock() {
                        should_continue = sess.handle_read_event(event);
                    }
                    if !should_continue {
                        return;
                    }
                }
                Err(_) => {
                    if let Ok(sess) = session.lock() {
                        sess.stream.finish(StreamEnd::Closed);
                    }
                    return;
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum ModifierKey {
    Ctrl,
    Alt,
    Shift,
    Meta,
}

impl ModifierKey {
    fn from_str(key: &str) -> Option<Self> {
        match key.to_lowercase().as_str() {
            "ctrl" | "control" => Some(Self::Ctrl),
            "alt" => Some(Self::Alt),
            "shift" => Some(Self::Shift),
            "meta" | "cmd" | "command" | "win" | "super" => Some(Self::Meta),
            _ => None,
        }
    }
}

#[derive(Default)]
struct ModifierState {
    ctrl: bool,
    alt: bool,
    shift: bool,
    meta: bool,
}

impl ModifierState {
    fn set(&mut self, key: ModifierKey, value: bool) {
        match key {
            ModifierKey::Ctrl => self.ctrl = value,
            ModifierKey::Alt => self.alt = value,
            ModifierKey::Shift => self.shift = value,
            ModifierKey::Meta => self.meta = value,
        }
    }

    fn is_empty(&self) -> bool {
        !(self.ctrl || self.alt || self.shift || self.meta)
    }

    fn has_alt_like(&self) -> bool {
        self.alt || self.meta
    }

    fn keystroke_bytes(&self, key: &str) -> Result<Vec<u8>, SessionError> {
        if self.is_empty() || key.contains('+') {
            return key_to_escape_sequence(key)
                .ok_or_else(|| SessionError::InvalidKey(key.to_string()));
        }

        if key.chars().count() == 1 {
            let key_char = key
                .chars()
                .next()
                .ok_or_else(|| SessionError::InvalidKey(key.to_string()))?;
            return Ok(self.modified_key_char_bytes(key_char));
        }

        let base_key = if self.shift && key.eq_ignore_ascii_case("tab") {
            "Shift+Tab"
        } else {
            key
        };

        let sequence = key_to_escape_sequence(base_key)
            .ok_or_else(|| SessionError::InvalidKey(key.to_string()))?;
        Ok(prefix_escape_if_needed(self.has_alt_like(), sequence))
    }

    fn typed_bytes(&self, text: &str) -> Vec<u8> {
        if self.is_empty() {
            return text.as_bytes().to_vec();
        }

        let mut bytes = Vec::with_capacity(text.len());
        for ch in text.chars() {
            bytes.extend(self.modified_text_char_bytes(ch));
        }
        bytes
    }

    fn modified_key_char_bytes(&self, ch: char) -> Vec<u8> {
        let shifted = if self.shift { shifted_key_char(ch) } else { ch };
        self.modified_char_bytes(shifted)
    }

    fn modified_text_char_bytes(&self, ch: char) -> Vec<u8> {
        let shifted = if self.shift && ch.is_ascii_alphabetic() {
            ch.to_ascii_uppercase()
        } else {
            ch
        };
        self.modified_char_bytes(shifted)
    }

    fn modified_char_bytes(&self, ch: char) -> Vec<u8> {
        let bytes = if self.ctrl {
            control_byte_for_char(ch)
                .map(|byte| vec![byte])
                .unwrap_or_else(|| ch.to_string().into_bytes())
        } else {
            ch.to_string().into_bytes()
        };
        prefix_escape_if_needed(self.has_alt_like(), bytes)
    }
}

fn prefix_escape_if_needed(needs_escape_prefix: bool, bytes: Vec<u8>) -> Vec<u8> {
    if !needs_escape_prefix {
        return bytes;
    }

    let mut prefixed = Vec::with_capacity(bytes.len() + 1);
    prefixed.push(0x1b);
    prefixed.extend(bytes);
    prefixed
}

fn shifted_key_char(ch: char) -> char {
    match ch {
        'a'..='z' => ch.to_ascii_uppercase(),
        '1' => '!',
        '2' => '@',
        '3' => '#',
        '4' => '$',
        '5' => '%',
        '6' => '^',
        '7' => '&',
        '8' => '*',
        '9' => '(',
        '0' => ')',
        '-' => '_',
        '=' => '+',
        '[' => '{',
        ']' => '}',
        '\\' => '|',
        ';' => ':',
        '\'' => '"',
        ',' => '<',
        '.' => '>',
        '/' => '?',
        '`' => '~',
        _ => ch,
    }
}

fn control_byte_for_char(ch: char) -> Option<u8> {
    match ch {
        'a'..='z' => Some((ch as u8) - b'a' + 1),
        'A'..='Z' => Some((ch as u8) - b'A' + 1),
        '@' | ' ' => Some(0),
        '[' => Some(27),
        '\\' => Some(28),
        ']' => Some(29),
        '^' => Some(30),
        '_' => Some(31),
        '?' => Some(127),
        _ => None,
    }
}

pub struct Session {
    pub id: SessionId,
    pub created_at: DateTime<Utc>,
    launch: Arc<SessionLaunchSpec>,
    pid: u32,
    pty: PtyHandle,
    terminal: VirtualTerminal,
    held_modifiers: ModifierState,
    stream: Arc<StreamBuffer>,
    pty_rx: Option<channel::Receiver<ReadEvent>>,
    pump_tx: Option<channel::Sender<PumpCommand>>,
    pump_join: Option<thread::JoinHandle<()>>,
}

impl Session {
    fn new(
        id: SessionId,
        launch: Arc<SessionLaunchSpec>,
        pid: u32,
        pty: PtyHandle,
        size: TerminalSize,
        stream_max_buffer_bytes: usize,
    ) -> Self {
        let stream = Arc::new(StreamBuffer::new(stream_max_buffer_bytes));
        let mut pty = pty;
        let pty_rx = pty.take_read_rx();
        Self {
            id,
            created_at: Utc::now(),
            launch,
            pid,
            pty,
            terminal: VirtualTerminal::new(size),
            held_modifiers: ModifierState::default(),
            stream,
            pty_rx,
            pump_tx: None,
            pump_join: None,
        }
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn is_running(&mut self) -> bool {
        self.pty.is_running()
    }

    pub fn size(&self) -> TerminalSize {
        self.terminal.size()
    }

    fn launch_spec(&self) -> Arc<SessionLaunchSpec> {
        Arc::clone(&self.launch)
    }

    pub fn request_flush(&self) -> Option<channel::Receiver<()>> {
        if let Some(tx) = self.pump_tx.as_ref() {
            let (ack_tx, ack_rx) = channel::bounded(1);
            if tx.send(PumpCommand::Flush(ack_tx)).is_ok() {
                return Some(ack_rx);
            }
        }
        None
    }

    pub fn screen_text(&self) -> String {
        self.terminal.screen_text()
    }

    pub fn screen_render(&self) -> String {
        let buffer = self.terminal.screen_buffer();
        render_screen(&buffer)
    }

    pub fn screen_render_compact(&self) -> String {
        let buffer = self.terminal.screen_buffer();
        render_screen_trimmed(&buffer)
    }

    pub fn cursor(&self) -> CursorPosition {
        self.terminal.cursor()
    }

    pub fn keystroke(&mut self, key: &str) -> Result<(), SessionError> {
        let seq = self.held_modifiers.keystroke_bytes(key)?;
        self.pty
            .write(&seq)
            .map_err(|err| SessionError::Terminal(err.into_port_error()))?;
        Ok(())
    }

    pub fn keydown(&mut self, key: &str) -> Result<(), SessionError> {
        let modifier = ModifierKey::from_str(key).ok_or_else(|| {
            SessionError::InvalidKey(format!(
                "{key}. Only modifier keys (Ctrl, Alt, Shift, Meta) can be held"
            ))
        })?;
        self.held_modifiers.set(modifier, true);
        Ok(())
    }

    pub fn keyup(&mut self, key: &str) -> Result<(), SessionError> {
        let modifier = ModifierKey::from_str(key).ok_or_else(|| {
            SessionError::InvalidKey(format!(
                "{key}. Only modifier keys (Ctrl, Alt, Shift, Meta) can be released"
            ))
        })?;
        self.held_modifiers.set(modifier, false);
        Ok(())
    }

    pub fn type_text(&mut self, text: &str) -> Result<(), SessionError> {
        if self.held_modifiers.is_empty() {
            self.pty
                .write_str(text)
                .map_err(|err| SessionError::Terminal(err.into_port_error()))?;
        } else {
            let bytes = self.held_modifiers.typed_bytes(text);
            self.pty
                .write(&bytes)
                .map_err(|err| SessionError::Terminal(err.into_port_error()))?;
        }
        Ok(())
    }

    pub fn resize(&mut self, size: TerminalSize) -> Result<(), SessionError> {
        self.pty
            .resize(size)
            .map_err(|err| SessionError::Terminal(err.into_port_error()))?;
        self.terminal.resize(size);
        self.stream.notify();
        Ok(())
    }

    pub fn kill(&mut self) -> Result<(), SessionError> {
        self.pty
            .kill()
            .map_err(|err| SessionError::Terminal(err.into_port_error()))?;
        Ok(())
    }

    pub fn pty_write(&mut self, data: &[u8]) -> Result<(), SessionError> {
        self.pty
            .write(data)
            .map_err(|err| SessionError::Terminal(err.into_port_error()))?;
        Ok(())
    }

    pub fn stream_read(
        &self,
        cursor: &mut StreamCursor,
        max_bytes: usize,
        timeout_ms: i32,
    ) -> Result<StreamRead, SessionError> {
        self.stream.read(cursor, max_bytes, timeout_ms)
    }

    pub fn stream_reader(&self) -> StreamReader {
        StreamReader::new(Arc::clone(&self.stream))
    }

    pub fn stream_subscribe(&self) -> StreamWaiterHandle {
        self.stream.subscribe()
    }

    fn take_pty_rx(&mut self) -> Option<channel::Receiver<ReadEvent>> {
        self.pty_rx.take()
    }

    fn handle_read_event(&mut self, event: ReadEvent) -> bool {
        match event {
            ReadEvent::Data(data) => {
                self.terminal.process(&data);
                self.stream.push_bytes(Bytes::from(data));
                true
            }
            ReadEvent::Eof => {
                self.stream.finish(StreamEnd::Closed);
                let _ = self.pty.is_running();
                false
            }
            ReadEvent::Error(error) => {
                self.stream.finish(StreamEnd::Failed(error));
                let _ = self.pty.is_running();
                false
            }
        }
    }

    fn pump_drain_events(&mut self, pty_rx: &channel::Receiver<ReadEvent>) -> bool {
        while let Ok(event) = pty_rx.try_recv() {
            if !self.handle_read_event(event) {
                return false;
            }
        }
        true
    }

    fn attach_pump(&mut self, tx: channel::Sender<PumpCommand>, join: thread::JoinHandle<()>) {
        self.pump_tx = Some(tx);
        self.pump_join = Some(join);
    }

    fn shutdown_pump(&mut self) -> Option<thread::JoinHandle<()>> {
        if let Some(tx) = self.pump_tx.take() {
            let _ = tx.send(PumpCommand::Shutdown);
        }
        self.pump_join.take()
    }

    pub fn live_preview_snapshot(&self) -> LivePreviewSnapshot {
        let size = self.terminal.size();
        let buffer = self.terminal.screen_buffer();
        let cursor = self.terminal.cursor();
        let seq = render_live_preview_init(&buffer, cursor);
        let stream_seq = self.stream.latest_seq();
        LivePreviewSnapshot {
            cols: size.cols(),
            rows: size.rows(),
            seq,
            stream_seq,
        }
    }
}

pub struct SessionManager {
    sessions: RwLock<HashMap<SessionId, Arc<Mutex<Session>>>>,
    active_session: RwLock<Option<SessionId>>,
    persistence: SessionPersistence,
    max_sessions: usize,
    stream_max_buffer_bytes: usize,
}

pub const DEFAULT_MAX_SESSIONS: usize = 16;

impl SessionManager {
    pub fn new() -> Result<Self, SessionError> {
        Self::with_max_sessions(DEFAULT_MAX_SESSIONS)
    }

    pub fn with_max_sessions(max_sessions: usize) -> Result<Self, SessionError> {
        Self::with_limits(max_sessions, DEFAULT_STREAM_MAX_BUFFER_BYTES)
    }

    fn with_limits(
        max_sessions: usize,
        stream_max_buffer_bytes: usize,
    ) -> Result<Self, SessionError> {
        let persistence = SessionPersistence::new();
        persistence.cleanup_stale_sessions()?;

        Ok(Self {
            sessions: RwLock::new(HashMap::new()),
            active_session: RwLock::new(None),
            persistence,
            max_sessions,
            stream_max_buffer_bytes,
        })
    }

    pub fn with_test_limits(
        max_sessions: usize,
        stream_max_buffer_bytes: usize,
    ) -> Result<Self, SessionError> {
        Self::with_limits(max_sessions, stream_max_buffer_bytes)
    }

    fn next_session_id(&self) -> SessionId {
        loop {
            let candidate = generate_session_id();
            let sessions = rwlock_read_or_recover(&self.sessions);
            if !sessions.contains_key(&candidate) {
                return candidate;
            }
        }
    }

    fn persisted_session(
        session_id: &SessionId,
        command: &str,
        pid: u32,
        size: TerminalSize,
    ) -> PersistedSession {
        PersistedSession {
            id: session_id.to_string(),
            command: command.to_string(),
            pid,
            created_at: Utc::now().to_rfc3339(),
            size,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        &self,
        command: &str,
        args: &[String],
        cwd: Option<&str>,
        env: Option<&HashMap<String, String>>,
        session_id: Option<SessionId>,
        size: TerminalSize,
    ) -> Result<(SessionId, u32), SessionError> {
        if let Some(ref requested_id) = session_id {
            let sessions = rwlock_read_or_recover(&self.sessions);
            if sessions.contains_key(requested_id) {
                return Err(SessionError::AlreadyExists(requested_id.to_string()));
            }
        }

        {
            let sessions = rwlock_read_or_recover(&self.sessions);
            if sessions.len() >= self.max_sessions {
                return Err(SessionError::LimitReached(self.max_sessions));
            }
        }

        let id = session_id.unwrap_or_else(|| self.next_session_id());
        let launch = Arc::new(SessionLaunchSpec {
            command: command.to_string(),
            args: args.to_vec(),
            cwd: cwd.map(str::to_string),
            env: env.cloned(),
        });

        let mut pty = PtyHandle::spawn(
            &launch.command,
            &launch.args,
            launch.cwd.as_deref(),
            launch.env.as_ref(),
            size,
        )
        .map_err(|e| SessionError::Terminal(e.into_port_error()))?;
        let pid = spawned_process_id(&mut pty)?;

        let persisted = Self::persisted_session(&id, &launch.command, pid, size);
        if let Err(err) = self.persistence.add_session(persisted) {
            let _ = pty.kill();
            return Err(err);
        }

        let session = Session::new(
            id.clone(),
            launch,
            pid,
            pty,
            size,
            self.stream_max_buffer_bytes,
        );
        let session = Arc::new(Mutex::new(session));
        let thread_name = format!("session-pump-{}", id.as_str());
        let (pump_tx, pump_join) = spawn_pump(Arc::clone(&session), thread_name);
        {
            let mut sess = mutex_lock_or_recover(&session);
            sess.attach_pump(pump_tx, pump_join);
        }

        {
            let mut sessions = rwlock_write_or_recover(&self.sessions);
            sessions.insert(id.clone(), Arc::clone(&session));
        }

        {
            let mut active = rwlock_write_or_recover(&self.active_session);
            *active = Some(id.clone());
        }

        Ok((id, pid))
    }

    pub fn get(&self, session_id: &SessionId) -> Result<Arc<Mutex<Session>>, SessionError> {
        let sessions = rwlock_read_or_recover(&self.sessions);
        sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))
    }

    pub fn active(&self) -> Result<Arc<Mutex<Session>>, SessionError> {
        let active_id = {
            let active = rwlock_read_or_recover(&self.active_session);
            active.clone()
        };

        match active_id {
            Some(id) => self.get(&id),
            None => Err(SessionError::NoActiveSession),
        }
    }

    pub fn resolve(
        &self,
        session_id: Option<&SessionId>,
    ) -> Result<Arc<Mutex<Session>>, SessionError> {
        match session_id {
            Some(id) => self.get(id),
            None => {
                if let Ok(active_session) = self.active() {
                    match self.session_running_state(&active_session) {
                        Some(true) | None => return Ok(active_session),
                        Some(false) => {}
                    }
                }

                if let Some((fallback_id, fallback_session)) = self.most_recent_running_session() {
                    let mut active = rwlock_write_or_recover(&self.active_session);
                    *active = Some(fallback_id);
                    return Ok(fallback_session);
                }

                let mut active = rwlock_write_or_recover(&self.active_session);
                *active = None;
                Err(SessionError::NoActiveSession)
            }
        }
    }

    pub fn set_active(&self, session_id: &SessionId) -> Result<(), SessionError> {
        let sessions = rwlock_read_or_recover(&self.sessions);
        if !sessions.contains_key(session_id) {
            return Err(SessionError::NotFound(session_id.to_string()));
        }
        let mut active = rwlock_write_or_recover(&self.active_session);
        *active = Some(session_id.clone());
        Ok(())
    }

    pub fn restart(&self, session_id: Option<&SessionId>) -> Result<RestartOutput, SessionError> {
        let session = self.resolve(session_id)?;
        let (old_session_id, launch, size) = {
            let sess = mutex_lock_or_recover(&session);
            (sess.id.clone(), sess.launch_spec(), sess.size())
        };

        let new_session_id = self.next_session_id();
        let mut replacement_pty = PtyHandle::spawn(
            &launch.command,
            &launch.args,
            launch.cwd.as_deref(),
            launch.env.as_ref(),
            size,
        )
        .map_err(|e| SessionError::Terminal(e.into_port_error()))?;
        let pid = spawned_process_id(&mut replacement_pty)?;
        let persisted = Self::persisted_session(&new_session_id, &launch.command, pid, size);
        if let Err(err) = self.persistence.add_session(persisted) {
            let _ = replacement_pty.kill();
            return Err(err);
        }

        let old_join = {
            let mut sess = mutex_lock_or_recover(&session);
            let join = sess.shutdown_pump();
            if let Err(err) = sess.kill() {
                drop(sess);
                if let Some(join) = join {
                    join_thread_and_warn_on_panic(join, "session pump");
                }
                let _ = replacement_pty.kill();
                if let Err(cleanup_err) = self.persistence.remove_session(&new_session_id) {
                    warn!(
                        session_id = %new_session_id,
                        error = %cleanup_err,
                        "Failed to remove pre-persisted replacement session after restart abort",
                    );
                }
                return Err(err);
            }
            join
        };
        if let Some(join) = old_join {
            join_thread_and_warn_on_panic(join, "session pump");
        }

        let new_session = Arc::new(Mutex::new(Session::new(
            new_session_id.clone(),
            Arc::clone(&launch),
            pid,
            replacement_pty,
            size,
            self.stream_max_buffer_bytes,
        )));
        let thread_name = format!("session-pump-{}", new_session_id.as_str());
        let (pump_tx, pump_join) = spawn_pump(Arc::clone(&new_session), thread_name);
        {
            let mut sess = mutex_lock_or_recover(&new_session);
            sess.attach_pump(pump_tx, pump_join);
        }

        {
            let mut sessions = rwlock_write_or_recover(&self.sessions);
            if sessions.remove(&old_session_id).is_none() {
                drop(sessions);
                let join = { mutex_lock_or_recover(&new_session).shutdown_pump() };
                let _ = mutex_lock_or_recover(&new_session).kill();
                if let Some(join) = join {
                    join_thread_and_warn_on_panic(join, "session pump");
                }
                if let Err(cleanup_err) = self.persistence.remove_session(&new_session_id) {
                    warn!(
                        session_id = %new_session_id,
                        error = %cleanup_err,
                        "Failed to remove replacement session metadata after restart lookup failure",
                    );
                }
                return Err(SessionError::NotFound(old_session_id.to_string()));
            }
            sessions.insert(new_session_id.clone(), Arc::clone(&new_session));
        }

        {
            let mut active = rwlock_write_or_recover(&self.active_session);
            *active = Some(new_session_id.clone());
        }

        self.persistence.remove_session(&old_session_id)?;

        Ok(RestartOutput {
            old_session_id,
            new_session_id,
            command: launch.command.clone(),
            pid,
        })
    }

    pub fn list(&self) -> Vec<SessionInfo> {
        use super::lock_helpers::acquire_session_lock;

        let persisted_sessions = self
            .persistence
            .load()
            .into_iter()
            .map(|session| (session.id.clone(), session))
            .collect::<HashMap<_, _>>();

        let session_refs: Vec<(SessionId, Arc<Mutex<Session>>)> = {
            let sessions = rwlock_read_or_recover(&self.sessions);
            sessions
                .iter()
                .map(|(id, session)| (id.clone(), Arc::clone(session)))
                .collect()
        };

        session_refs
            .into_iter()
            .filter_map(|(id, session)| {
                if let Some(mut sess) = acquire_session_lock(&session, SESSION_QUERY_LOCK_TIMEOUT) {
                    Some(SessionInfo {
                        id,
                        command: sess.launch.command.clone(),
                        pid: sess.pid(),
                        running: sess.is_running(),
                        created_at: sess.created_at.to_rfc3339(),
                        size: sess.size(),
                    })
                } else {
                    let persisted = persisted_sessions.get(id.as_str())?;
                    Some(SessionInfo {
                        id,
                        command: persisted.command.clone(),
                        pid: persisted.pid,
                        running: is_process_running(persisted.pid),
                        created_at: persisted.created_at.clone(),
                        size: persisted.size,
                    })
                }
            })
            .collect()
    }

    pub fn kill(&self, session_id: &SessionId) -> Result<(), SessionError> {
        let (session, was_active) = {
            let sessions = rwlock_read_or_recover(&self.sessions);
            let session = sessions
                .get(session_id)
                .cloned()
                .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;
            let active = rwlock_read_or_recover(&self.active_session);
            let was_active = active.as_ref() == Some(session_id);
            (session, was_active)
        };

        {
            let mut sess = mutex_lock_or_recover(&session);
            let join = sess.shutdown_pump();
            sess.kill()?;
            drop(sess);
            if let Some(join) = join {
                join_thread_and_warn_on_panic(join, "session pump");
            }
        }

        self.persistence.remove_session(session_id)?;

        {
            let mut sessions = rwlock_write_or_recover(&self.sessions);
            let _ = sessions.remove(session_id);
        }

        if was_active {
            let fallback_id = self.most_recent_running_session().map(|(id, _)| id);
            let mut active = rwlock_write_or_recover(&self.active_session);
            *active = fallback_id;
        }

        Ok(())
    }

    pub fn session_count(&self) -> usize {
        rwlock_read_or_recover(&self.sessions).len()
    }

    pub fn active_session_id(&self) -> Option<SessionId> {
        rwlock_read_or_recover(&self.active_session).clone()
    }

    fn session_running_state(&self, session: &Arc<Mutex<Session>>) -> Option<bool> {
        use super::lock_helpers::acquire_session_lock;

        acquire_session_lock(session, SESSION_QUERY_LOCK_TIMEOUT).map(|mut sess| sess.is_running())
    }

    fn most_recent_running_session(&self) -> Option<(SessionId, Arc<Mutex<Session>>)> {
        use super::lock_helpers::acquire_session_lock;

        let session_refs: Vec<(SessionId, Arc<Mutex<Session>>)> = {
            let sessions = rwlock_read_or_recover(&self.sessions);
            sessions
                .iter()
                .map(|(id, session)| (id.clone(), Arc::clone(session)))
                .collect()
        };

        let mut selected: Option<(i64, SessionId, Arc<Mutex<Session>>)> = None;

        for (id, session) in session_refs {
            let created_at = {
                let Some(mut sess) = acquire_session_lock(&session, SESSION_QUERY_LOCK_TIMEOUT)
                else {
                    continue;
                };

                if !sess.is_running() {
                    None
                } else {
                    Some(sess.created_at.timestamp_micros())
                }
            };

            let Some(created_at) = created_at else {
                continue;
            };
            let replace = match selected.as_ref() {
                Some((best_created, best_id, _)) => {
                    created_at > *best_created
                        || (created_at == *best_created && id.as_str() > best_id.as_str())
                }
                None => true,
            };

            if replace {
                selected = Some((created_at, id, session));
            }
        }

        selected.map(|(_, id, session)| (id, session))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSession {
    pub id: String,
    pub command: String,
    pub pid: u32,
    pub created_at: String,
    #[serde(flatten)]
    pub size: TerminalSize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum SessionEvent {
    Upsert { session: PersistedSession },
    Remove { session_id: String },
}

#[derive(Default)]
struct SessionLogState {
    sessions: HashMap<String, PersistedSession>,
    unknown_records: usize,
}

impl SessionLogState {
    fn into_sessions(self) -> Vec<PersistedSession> {
        self.sessions.into_values().collect()
    }
}

pub struct SessionPersistence {
    path: PathBuf,
    lock_path: PathBuf,
}

const SESSION_STORE_COMPACT_THRESHOLD_BYTES: u64 = 1_048_576;

impl SessionPersistence {
    pub fn new() -> Self {
        let path = Self::sessions_file_path();
        let lock_path = path.with_extension("lock");
        Self { path, lock_path }
    }

    fn sessions_file_path() -> PathBuf {
        if let Ok(path) = std::env::var("AGENT_TUI_SESSION_STORE") {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed);
            }
        }
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir());
        let dir = home.join(".agent-tui");
        dir.join("sessions.jsonl")
    }

    fn legacy_sessions_file_path(&self) -> PathBuf {
        self.path.with_extension("json")
    }

    fn io_to_persistence(operation: &str, e: std::io::Error) -> SessionError {
        let reason = e.to_string();
        SessionError::Persistence {
            operation: operation.to_string(),
            reason,
            source: Some(Box::new(e)),
        }
    }

    fn ensure_dir(&self) -> Result<(), SessionError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                Self::io_to_persistence(
                    "create_dir",
                    std::io::Error::new(
                        e.kind(),
                        format!("Failed to create directory '{}': {}", parent.display(), e),
                    ),
                )
            })?;
        }
        Ok(())
    }

    fn acquire_lock(&self) -> Result<File, SessionError> {
        const PERSISTENCE_LOCK_TIMEOUT: Duration = Duration::from_secs(5);

        self.ensure_dir()?;
        let lock_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.lock_path)
            .map_err(|e| Self::io_to_persistence("open_lock", e))?;

        let fd = lock_file.as_raw_fd();
        let start = Instant::now();
        let mut backoff = Duration::from_millis(1);

        loop {
            // SAFETY: `flock` is safe to call with a valid file descriptor obtained from
            // `as_raw_fd()`. The file remains open throughout this loop, ensuring the fd
            // is valid. LOCK_EX | LOCK_NB requests an exclusive, non-blocking lock.
            let result = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
            if result == 0 {
                return Ok(lock_file);
            }

            let err = std::io::Error::last_os_error();
            if err.raw_os_error() != Some(libc::EWOULDBLOCK)
                && err.raw_os_error() != Some(libc::EAGAIN)
            {
                return Err(Self::io_to_persistence("flock", err));
            }

            if start.elapsed() > PERSISTENCE_LOCK_TIMEOUT {
                return Err(SessionError::Persistence {
                    operation: "acquire_lock".to_string(),
                    reason: "lock acquisition timed out after 5 seconds".to_string(),
                    source: None,
                });
            }

            std::thread::park_timeout(backoff);
            backoff = (backoff * 2).min(Duration::from_millis(100));
        }
    }

    fn migrate_legacy_if_needed_locked(&self) -> Result<(), SessionError> {
        if self.path.exists() {
            return Ok(());
        }
        let legacy_path = self.legacy_sessions_file_path();
        if !legacy_path.exists() {
            return Ok(());
        }
        let legacy_file = File::open(&legacy_path).map_err(|e| SessionError::Persistence {
            operation: "open_legacy".to_string(),
            reason: format!(
                "Failed to open legacy sessions file '{}': {}",
                legacy_path.display(),
                e
            ),
            source: Some(Box::new(e)),
        })?;
        let reader = BufReader::new(legacy_file);
        let sessions: Vec<PersistedSession> = match serde_json::from_reader(reader) {
            Ok(parsed) => parsed,
            Err(e) => {
                warn!(
                    path = %legacy_path.display(),
                    error = %e,
                    "Failed to parse legacy sessions file; skipping migration"
                );
                return Ok(());
            }
        };

        self.ensure_dir()?;
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)
            .map_err(|e| Self::io_to_persistence("create_jsonl", e))?;

        for session in sessions {
            let event = SessionEvent::Upsert { session };
            let line = serde_json::to_string(&event).map_err(|e| SessionError::Persistence {
                operation: "serialize_event".to_string(),
                reason: format!("Failed to serialize session event: {e}"),
                source: Some(Box::new(e)),
            })?;
            writeln!(file, "{line}").map_err(|e| Self::io_to_persistence("write_event", e))?;
        }

        let backup_path = legacy_path.with_extension("json.bak");
        fs::rename(&legacy_path, &backup_path).map_err(|e| SessionError::Persistence {
            operation: "rename_legacy".to_string(),
            reason: format!(
                "Failed to rename legacy sessions file '{}' to '{}': {}",
                legacy_path.display(),
                backup_path.display(),
                e
            ),
            source: Some(Box::new(e)),
        })?;
        Ok(())
    }

    fn load_log_state_unlocked(&self) -> SessionLogState {
        if !self.path.exists() {
            return SessionLogState::default();
        }
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(e) => {
                warn!(
                    path = %self.path.display(),
                    error = %e,
                    "Failed to open sessions log"
                );
                return SessionLogState::default();
            }
        };

        let mut state = SessionLogState::default();
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = match line {
                Ok(line) => line,
                Err(e) => {
                    warn!(error = %e, "Failed to read session log line");
                    state.unknown_records += 1;
                    continue;
                }
            };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let event: SessionEvent = match serde_json::from_str(trimmed) {
                Ok(event) => event,
                Err(e) => {
                    warn!(error = %e, "Failed to parse session log entry");
                    state.unknown_records += 1;
                    continue;
                }
            };
            match event {
                SessionEvent::Upsert { session } => {
                    state.sessions.insert(session.id.clone(), session);
                }
                SessionEvent::Remove { session_id } => {
                    state.sessions.remove(&session_id);
                }
            }
        }

        state
    }

    fn load_unlocked(&self) -> Vec<PersistedSession> {
        self.load_log_state_unlocked().into_sessions()
    }

    fn write_event_unlocked(&self, event: &SessionEvent) -> Result<(), SessionError> {
        self.ensure_dir()?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| Self::io_to_persistence("open_jsonl", e))?;
        let line = serde_json::to_string(event).map_err(|e| SessionError::Persistence {
            operation: "serialize_event".to_string(),
            reason: format!("Failed to serialize session event: {e}"),
            source: Some(Box::new(e)),
        })?;
        writeln!(file, "{line}").map_err(|e| Self::io_to_persistence("write_event", e))?;
        Ok(())
    }

    fn save_unlocked(&self, sessions: &[PersistedSession]) -> Result<(), SessionError> {
        let temp_path = self.path.with_extension("jsonl.tmp");
        let file = File::create(&temp_path).map_err(|e| SessionError::Persistence {
            operation: "create_temp".to_string(),
            reason: format!(
                "Failed to create temp file '{}': {}",
                temp_path.display(),
                e
            ),
            source: Some(Box::new(e)),
        })?;
        let mut writer = BufWriter::new(file);
        for session in sessions {
            let event = SessionEvent::Upsert {
                session: session.clone(),
            };
            let line = serde_json::to_string(&event).map_err(|e| SessionError::Persistence {
                operation: "serialize_event".to_string(),
                reason: format!("Failed to serialize session event: {e}"),
                source: Some(Box::new(e)),
            })?;
            writeln!(writer, "{line}").map_err(|e| Self::io_to_persistence("write_event", e))?;
        }
        writer
            .flush()
            .map_err(|e| Self::io_to_persistence("flush_jsonl", e))?;
        fs::rename(&temp_path, &self.path).map_err(|e| SessionError::Persistence {
            operation: "rename".to_string(),
            reason: format!(
                "Failed to rename '{}' to '{}': {}",
                temp_path.display(),
                self.path.display(),
                e
            ),
            source: Some(Box::new(e)),
        })?;
        Ok(())
    }

    fn maybe_compact_unlocked(&self) -> Result<(), SessionError> {
        let size = match fs::metadata(&self.path) {
            Ok(meta) => meta.len(),
            Err(_) => return Ok(()),
        };
        if size < SESSION_STORE_COMPACT_THRESHOLD_BYTES {
            return Ok(());
        }
        let state = self.load_log_state_unlocked();
        if state.unknown_records > 0 {
            warn!(
                path = %self.path.display(),
                unknown_records = state.unknown_records,
                "Skipping session log compaction because unknown records must be preserved"
            );
            return Ok(());
        }
        self.save_unlocked(&state.into_sessions())?;
        Ok(())
    }

    pub fn load(&self) -> Vec<PersistedSession> {
        match self.acquire_lock() {
            Ok(_lock) => {
                if let Err(error) = self.migrate_legacy_if_needed_locked() {
                    warn!(
                        path = %self.path.display(),
                        error = %error,
                        "Failed to migrate legacy session metadata while loading sessions"
                    );
                }
                self.load_unlocked()
            }
            Err(e) => {
                warn!(error = %e, "Failed to acquire lock for loading sessions");
                self.load_unlocked()
            }
        }
    }

    pub fn add_session(&self, session: PersistedSession) -> Result<(), SessionError> {
        let _lock = self.acquire_lock()?;
        self.migrate_legacy_if_needed_locked()?;
        self.write_event_unlocked(&SessionEvent::Upsert { session })?;
        self.maybe_compact_unlocked()
    }

    pub fn remove_session(&self, session_id: &str) -> Result<(), SessionError> {
        let _lock = self.acquire_lock()?;
        self.migrate_legacy_if_needed_locked()?;
        self.write_event_unlocked(&SessionEvent::Remove {
            session_id: session_id.to_string(),
        })?;
        self.maybe_compact_unlocked()
    }

    pub fn cleanup_stale_sessions(&self) -> Result<usize, SessionError> {
        let _lock = self.acquire_lock()?;
        self.migrate_legacy_if_needed_locked()?;
        let state = self.load_log_state_unlocked();
        let unknown_records = state.unknown_records;
        let sessions = state.into_sessions();
        let mut cleaned = 0;

        let mut active_sessions = Vec::new();
        let mut removed_session_ids = Vec::new();
        for session in sessions {
            if session.pid == 0 {
                cleaned += 1;
                removed_session_ids.push(session.id);
                continue;
            }

            reap_child_if_any(session.pid);
            if !is_process_running(session.pid) {
                cleaned += 1;
                removed_session_ids.push(session.id);
                continue;
            }

            match verify_persisted_session_identity(&session) {
                ProcessIdentity::Match => {
                    let _ = terminate_process_group(session.pid);
                    reap_child_if_any(session.pid);
                    if !is_process_running(session.pid) {
                        cleaned += 1;
                        removed_session_ids.push(session.id);
                        continue;
                    }

                    warn!(
                        session_id = %session.id,
                        pid = session.pid,
                        "Failed to terminate persisted session; leaving entry"
                    );
                    active_sessions.push(session);
                }
                ProcessIdentity::Mismatch => {
                    warn!(
                        session_id = %session.id,
                        pid = session.pid,
                        "Persisted PID does not match session identity; removing entry without terminating"
                    );
                    cleaned += 1;
                    removed_session_ids.push(session.id);
                }
                ProcessIdentity::Unknown => {
                    warn!(
                        session_id = %session.id,
                        pid = session.pid,
                        "Unable to verify persisted PID identity; skipping termination"
                    );
                    active_sessions.push(session);
                }
            }
        }

        if unknown_records > 0 {
            for session_id in removed_session_ids {
                self.write_event_unlocked(&SessionEvent::Remove { session_id })?;
            }
        } else {
            self.save_unlocked(&active_sessions)?;
        }
        Ok(cleaned)
    }
}

#[cfg(test)]
mod stream_tests {
    use super::StreamBuffer;
    use super::StreamCursor;
    use super::StreamEnd;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn stream_read_returns_data_and_advances_cursor() {
        let buffer = StreamBuffer::new(16);
        let mut cursor = StreamCursor::default();

        buffer.push(b"hello");
        let read = buffer
            .read(&mut cursor, 16, 0)
            .expect("stream read should succeed");

        assert_eq!(read.data, b"hello");
        assert_eq!(cursor.seq, 5);
        assert_eq!(read.dropped_bytes, 0);
        assert_eq!(read.latest_cursor.seq, 5);
        assert!(!read.closed);
    }

    #[test]
    fn stream_read_reports_drops_and_returns_latest_bytes() {
        let buffer = StreamBuffer::new(4);
        let mut cursor = StreamCursor::default();

        buffer.push(b"abcdef");
        let read = buffer
            .read(&mut cursor, 10, 0)
            .expect("stream read should succeed");

        assert_eq!(read.dropped_bytes, 2);
        assert_eq!(read.data, b"cdef");
        assert_eq!(cursor.seq, 6);
        assert_eq!(read.latest_cursor.seq, 6);
        assert!(!read.closed);
    }

    #[test]
    fn stream_read_waits_until_data_or_timeout() {
        let buffer = Arc::new(StreamBuffer::new(16));
        let mut cursor = StreamCursor::default();

        let buffer_clone = Arc::clone(&buffer);
        thread::spawn(move || {
            thread::park_timeout(Duration::from_millis(50));
            buffer_clone.push(b"ok");
        });

        let read = buffer
            .read(&mut cursor, 16, 200)
            .expect("stream read should succeed");
        assert_eq!(read.data, b"ok");
        assert_eq!(cursor.seq, 2);
        assert_eq!(read.latest_cursor.seq, 2);
        assert!(!read.closed);
    }

    #[test]
    fn stream_read_is_independent_per_cursor() {
        let buffer = StreamBuffer::new(16);
        let mut cursor_a = StreamCursor::default();
        let mut cursor_b = StreamCursor::default();

        buffer.push(b"hello");

        let read_a = buffer
            .read(&mut cursor_a, 2, 0)
            .expect("first stream read should succeed");
        let read_b = buffer
            .read(&mut cursor_b, 16, 0)
            .expect("second stream read should succeed");

        assert_eq!(read_a.data, b"he");
        assert_eq!(read_b.data, b"hello");
        assert_eq!(cursor_a.seq, 2);
        assert_eq!(cursor_b.seq, 5);
        assert_eq!(read_a.latest_cursor.seq, 5);
        assert_eq!(read_b.latest_cursor.seq, 5);
    }

    #[test]
    fn stream_subscribe_notifies_on_push() {
        let buffer = StreamBuffer::new(16);
        let subscription = buffer.subscribe();
        buffer.push(b"ping");
        assert!(subscription.wait(Some(Duration::from_millis(50))));
    }

    #[test]
    fn stream_subscribe_notifies_on_close() {
        let buffer = StreamBuffer::new(16);
        let subscription = buffer.subscribe();
        buffer.finish(StreamEnd::Closed);
        assert!(subscription.wait(Some(Duration::from_millis(50))));
    }

    #[test]
    fn stream_subscribe_drop_removes_notifier_without_extra_events() {
        let buffer = StreamBuffer::new(16);
        let subscription = buffer.subscribe();
        assert_eq!(buffer.notifier_count(), 1);
        drop(subscription);
        assert_eq!(
            buffer.notifier_count(),
            0,
            "dropped subscription should be removed immediately"
        );
    }
}

#[cfg(test)]
mod pump_tests {
    use super::PUMP_FLUSH_TIMEOUT;
    use super::Session;
    use super::StreamCursor;
    use super::spawn_pump;
    use crate::common::mutex_lock_or_recover;
    use crate::domain::SessionId;
    use crate::domain::TerminalSize;
    use crate::infra::terminal::PtyHandle;
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::time::Duration;
    use std::time::Instant;

    #[cfg(unix)]
    fn run_pump_stream_output_case() {
        const OUTPUT_MARKER: &str = "hi";
        let args = vec!["-c".to_string(), "printf 'hi\\n'; sleep 0.02".to_string()];
        let shell = if Path::new("/bin/sh").exists() {
            "/bin/sh"
        } else {
            "sh"
        };
        let pty = PtyHandle::spawn(shell, &args, Some("/tmp"), None, TerminalSize::default())
            .expect("PTY should spawn");
        let pid = pty.pid().expect("spawned test process should have a PID");
        let session = Session::new(
            SessionId::try_new("test-session").expect("valid session id"),
            Arc::new(super::SessionLaunchSpec {
                command: "sh".to_string(),
                args,
                cwd: Some("/tmp".to_string()),
                env: None,
            }),
            pid,
            pty,
            TerminalSize::default(),
            super::DEFAULT_STREAM_MAX_BUFFER_BYTES,
        );
        let session = Arc::new(Mutex::new(session));

        let (tx, join) = spawn_pump(Arc::clone(&session), "test-pump".to_string());
        {
            let mut guard = mutex_lock_or_recover(&session);
            guard.attach_pump(tx, join);
        }

        let reader = { mutex_lock_or_recover(&session).stream_reader() };
        let mut cursor = StreamCursor::default();
        let deadline = Instant::now() + Duration::from_millis(1500);
        let mut collected = Vec::new();

        while Instant::now() < deadline {
            let ack = { mutex_lock_or_recover(&session).request_flush() };
            if let Some(ack) = ack {
                let _ = ack.recv_timeout(PUMP_FLUSH_TIMEOUT);
            }

            let read = reader
                .read(&mut cursor, 4096, 50)
                .expect("pump reader should succeed");
            if !read.data.is_empty() {
                collected.extend_from_slice(&read.data);
            }
            if String::from_utf8_lossy(&collected).contains(OUTPUT_MARKER) {
                break;
            }
            if read.closed && cursor.seq >= read.latest_cursor.seq {
                break;
            }
        }

        assert!(
            String::from_utf8_lossy(&collected).contains(OUTPUT_MARKER),
            "stream buffer did not contain expected marker; collected={:?}",
            String::from_utf8_lossy(&collected)
        );

        let join = { mutex_lock_or_recover(&session).shutdown_pump() };
        let _ = mutex_lock_or_recover(&session).kill();
        if let Some(join) = join {
            let _ = join.join();
        }
    }

    #[cfg(unix)]
    #[test]
    fn session_pump_streams_output_into_buffer() {
        run_pump_stream_output_case();
    }

    #[cfg(unix)]
    #[test]
    fn session_pump_streams_output_into_buffer_stress() {
        for _ in 0..20 {
            run_pump_stream_output_case();
        }
    }
}

impl Default for SessionPersistence {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessIdentity {
    Match,
    Mismatch,
    Unknown,
}

struct ProcessInfo {
    start_time: Option<DateTime<Utc>>,
    cmdline: Option<String>,
}

fn verify_persisted_session_identity(session: &PersistedSession) -> ProcessIdentity {
    if session.pid == std::process::id() {
        return ProcessIdentity::Mismatch;
    }

    let created_at = match DateTime::parse_from_rfc3339(&session.created_at) {
        Ok(parsed) => parsed.with_timezone(&Utc),
        Err(_) => return ProcessIdentity::Unknown,
    };

    let info = match process_info(session.pid) {
        Some(info) => info,
        None => return ProcessIdentity::Unknown,
    };

    let start_time = match info.start_time {
        Some(start_time) => start_time,
        None => return ProcessIdentity::Unknown,
    };

    let delta_seconds = (start_time - created_at).num_seconds().abs();
    if delta_seconds > STARTUP_PID_START_TOLERANCE_SECS {
        return ProcessIdentity::Mismatch;
    }

    if matches!(
        (info.cmdline.as_ref(), expected_command(&session.command)),
        (Some(cmdline), Some(expected)) if !cmdline.contains(expected)
    ) {
        return ProcessIdentity::Unknown;
    }

    ProcessIdentity::Match
}

fn expected_command(command: &str) -> Option<&str> {
    let trimmed = command.trim();
    if trimmed.is_empty() || trimmed == "(locked)" {
        None
    } else {
        Some(trimmed)
    }
}

fn process_info(pid: u32) -> Option<ProcessInfo> {
    #[cfg(target_os = "linux")]
    {
        if let Some(info) = process_info_from_proc(pid) {
            return Some(info);
        }
    }

    process_info_from_sysinfo(pid)
}

#[cfg(target_os = "linux")]
fn process_info_from_proc(pid: u32) -> Option<ProcessInfo> {
    let stat_path = format!("/proc/{pid}/stat");
    let stat = fs::read_to_string(stat_path).ok()?;
    let start_ticks = parse_proc_start_time(&stat)?;

    let ticks_per_second = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if ticks_per_second <= 0 {
        return None;
    }

    let uptime = fs::read_to_string("/proc/uptime").ok()?;
    let uptime_secs: f64 = uptime.split_whitespace().next()?.parse().ok()?;
    let now = Utc::now();
    let boot_time = now - chrono::Duration::milliseconds((uptime_secs * 1000.0) as i64);
    let start_secs = start_ticks as f64 / ticks_per_second as f64;
    let start_time = boot_time + chrono::Duration::milliseconds((start_secs * 1000.0) as i64);

    let cmdline_path = format!("/proc/{pid}/cmdline");
    let cmdline = fs::read(cmdline_path)
        .ok()
        .and_then(|bytes| parse_cmdline_bytes(&bytes));

    Some(ProcessInfo {
        start_time: Some(start_time),
        cmdline,
    })
}

#[cfg(target_os = "linux")]
fn parse_proc_start_time(stat: &str) -> Option<u64> {
    let end = stat.rfind(')')?;
    let after = stat.get(end + 2..)?;
    let mut fields = after.split_whitespace();
    let start_time = fields.nth(19)?;
    start_time.parse().ok()
}

fn process_info_from_sysinfo(pid: u32) -> Option<ProcessInfo> {
    let pid = Pid::from_u32(pid);
    let refresh = ProcessRefreshKind::nothing()
        .with_cmd(UpdateKind::Always)
        .with_exe(UpdateKind::Always);
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, refresh);
    let process = system.process(pid)?;

    let cmd = process.cmd();
    let cmdline = if !cmd.is_empty() {
        let mut cmdline = String::new();
        for arg in cmd {
            if !cmdline.is_empty() {
                cmdline.push(' ');
            }
            cmdline.push_str(&arg.to_string_lossy());
        }
        Some(cmdline)
    } else {
        process
            .exe()
            .map(|path| path.to_string_lossy().into_owned())
    };

    let start_time = if process.start_time() > 0 {
        let boot_time = System::boot_time();
        let timestamp = boot_time.saturating_add(process.start_time());
        Utc.timestamp_opt(timestamp as i64, 0).single()
    } else {
        None
    };

    Some(ProcessInfo {
        start_time,
        cmdline,
    })
}

#[cfg(target_os = "linux")]
fn parse_cmdline_bytes(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let decoded = String::from_utf8_lossy(bytes);
    let trimmed = decoded.trim_matches(|ch: char| ch == '\0' || ch.is_whitespace());
    if trimmed.is_empty() {
        None
    } else {
        let mut cmdline = String::with_capacity(trimmed.len());
        for ch in trimmed.chars() {
            if ch == '\0' {
                cmdline.push(' ');
            } else {
                cmdline.push(ch);
            }
        }
        Some(cmdline)
    }
}

fn is_process_running(pid: u32) -> bool {
    // SAFETY: `kill` with signal 0 performs a permission check without sending any signal.
    // This is a standard POSIX idiom to check if a process exists. The pid is validated
    // before converting to pid_t.
    let pid_t: libc::pid_t = match pid.try_into() {
        Ok(pid_t) => pid_t,
        Err(_) => return false,
    };
    unsafe { libc::kill(pid_t, 0) == 0 }
}

#[cfg(unix)]
fn reap_child_if_any(pid: u32) {
    let pid_t: libc::pid_t = match pid.try_into() {
        Ok(pid_t) => pid_t,
        Err(_) => return,
    };
    let mut status: libc::c_int = 0;
    loop {
        // SAFETY: waitpid is safe with a valid pid and status pointer.
        let rc = unsafe { libc::waitpid(pid_t, &mut status, libc::WNOHANG) };
        if rc == -1 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
        }
        break;
    }
}

#[cfg(not(unix))]
fn reap_child_if_any(_pid: u32) {}

fn wait_for_process_exit(pid: u32, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if !is_process_running(pid) {
            return true;
        }
        if Instant::now() >= deadline {
            return !is_process_running(pid);
        }
        std::thread::park_timeout(STARTUP_KILL_POLL_INTERVAL);
    }
}

fn terminate_process_group(pid: u32) -> bool {
    let pid_t: libc::pid_t = match pid.try_into() {
        Ok(pid_t) => pid_t,
        Err(_) => return false,
    };

    // SAFETY: negative pid targets the process group created for the session leader.
    let rc = unsafe { libc::kill(-pid_t, libc::SIGTERM) };
    if rc == 0 && wait_for_process_exit(pid, STARTUP_TERMINATE_TIMEOUT) {
        return true;
    }
    if rc != 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ESRCH) {
            return true;
        }
    }

    let rc = unsafe { libc::kill(-pid_t, libc::SIGKILL) };
    if rc == 0 {
        return wait_for_process_exit(pid, STARTUP_KILL_TIMEOUT);
    }
    if rc != 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ESRCH) {
            return true;
        }
    }

    false
}

impl From<&SessionInfo> for PersistedSession {
    fn from(info: &SessionInfo) -> Self {
        PersistedSession {
            id: info.id.to_string(),
            command: info.command.clone(),
            pid: info.pid,
            created_at: info.created_at.clone(),
            size: info.size,
        }
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
