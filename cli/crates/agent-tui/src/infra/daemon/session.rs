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

use crate::common::mutex_lock_or_recover;
use crate::common::rwlock_read_or_recover;
use crate::common::rwlock_write_or_recover;
use crate::infra::terminal::CursorPosition;
use crate::infra::terminal::PtyHandle;
use crate::infra::terminal::ReadEvent;
use crate::infra::terminal::key_to_escape_sequence;
use crate::infra::terminal::render_screen;
use crate::usecases::ports::LivePreviewSnapshot;
use crate::usecases::ports::StreamCursor;
use crate::usecases::ports::StreamRead;
use crate::usecases::ports::StreamWaiter;
use crate::usecases::ports::StreamWaiterHandle;

use super::pty_session::PtySession;
use crate::infra::daemon::TerminalState;

pub use crate::domain::session_types::SessionId;
pub use crate::domain::session_types::SessionInfo;
use crate::domain::session_types::TerminalSize;
pub use crate::infra::daemon::SessionError;

const STREAM_MAX_BUFFER_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const PUMP_FLUSH_TIMEOUT: Duration = Duration::from_millis(50);
const STARTUP_TERMINATE_TIMEOUT: Duration = Duration::from_millis(500);
const STARTUP_KILL_TIMEOUT: Duration = Duration::from_millis(500);
const STARTUP_KILL_POLL_INTERVAL: Duration = Duration::from_millis(25);
const STARTUP_PID_START_TOLERANCE_SECS: i64 = 30;

pub fn generate_session_id() -> SessionId {
    SessionId::new(Uuid::new_v4().to_string()[..8].to_string())
}

struct StreamState {
    buffer: VecDeque<Bytes>,
    buffer_len: usize,
    base_seq: u64,
    next_seq: u64,
    dropped_bytes: u64,
    closed: bool,
    error: Option<String>,
}

struct StreamBuffer {
    state: RwLock<StreamState>,
    wait_lock: Mutex<()>,
    cv: Condvar,
    notifiers: Mutex<Vec<channel::Sender<()>>>,
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
}

impl StreamWaiter for StreamWaiterImpl {
    fn wait(&self, timeout: Option<Duration>) -> bool {
        match timeout {
            Some(timeout) => self.receiver.recv_timeout(timeout).is_ok(),
            None => self.receiver.recv().is_ok(),
        }
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
                closed: false,
                error: None,
            }),
            wait_lock: Mutex::new(()),
            cv: Condvar::new(),
            notifiers: Mutex::new(Vec::new()),
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
        let _wait_guard = self.wait_lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut state = self.state.write().unwrap_or_else(|e| e.into_inner());
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

    fn close(&self, error: Option<String>) {
        let _wait_guard = self.wait_lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut state = self.state.write().unwrap_or_else(|e| e.into_inner());
        state.closed = true;
        state.error = error;
        drop(state);
        self.notify_listeners();
        self.cv.notify_all();
    }

    fn notify(&self) {
        let _wait_guard = self.wait_lock.lock().unwrap_or_else(|e| e.into_inner());
        self.notify_listeners();
        self.cv.notify_all();
    }

    fn subscribe(&self) -> StreamWaiterHandle {
        let (tx, rx) = channel::bounded(1);
        self.notify_listeners();
        {
            let mut notifiers = self.notifiers.lock().unwrap_or_else(|e| e.into_inner());
            notifiers.push(tx);
        }
        Arc::new(StreamWaiterImpl { receiver: rx })
    }

    fn latest_seq(&self) -> u64 {
        let state = self.state.read().unwrap_or_else(|e| e.into_inner());
        state.next_seq
    }

    fn notify_listeners(&self) {
        let mut notifiers = self.notifiers.lock().unwrap_or_else(|e| e.into_inner());
        notifiers.retain(|sender| match sender.try_send(()) {
            Ok(()) => true,
            Err(channel::TrySendError::Full(_)) => true,
            Err(channel::TrySendError::Disconnected(_)) => false,
        });
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

        let mut guard = self.wait_lock.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            let state = self.state.read().unwrap_or_else(|e| e.into_inner());
            if state.next_seq > cursor.seq || state.closed {
                break;
            }
            drop(state);

            if let Some(wait) = timeout {
                let (new_guard, result) = self
                    .cv
                    .wait_timeout(guard, wait)
                    .unwrap_or_else(|e| e.into_inner());
                guard = new_guard;
                if result.timed_out() {
                    break;
                }
            } else {
                guard = self.cv.wait(guard).unwrap_or_else(|e| e.into_inner());
            }
        }
        drop(guard);

        let state = self.state.read().unwrap_or_else(|e| e.into_inner());
        if let Some(error) = state.error.clone() {
            return Err(SessionError::Terminal(
                crate::usecases::ports::TerminalError::Read {
                    reason: error,
                    source: None,
                },
            ));
        }

        let latest_cursor = StreamCursor {
            seq: state.next_seq,
        };
        let closed = state.closed;
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
    cursor: &CursorPosition,
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
            .unwrap_or_else(|e| e.into_inner())
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
            match payload.lock().unwrap_or_else(|e| e.into_inner()).take() {
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
                        sess.stream.close(None);
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
                        sess.stream.close(None);
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
}

pub struct Session {
    pub id: SessionId,
    pub command: String,
    pub created_at: DateTime<Utc>,
    pty: PtySession,
    terminal: TerminalState,
    held_modifiers: ModifierState,
    stream: Arc<StreamBuffer>,
    pty_rx: Option<channel::Receiver<ReadEvent>>,
    pty_cursor: Arc<Mutex<StreamCursor>>,
    pump_tx: Option<channel::Sender<PumpCommand>>,
    pump_join: Option<thread::JoinHandle<()>>,
}

impl Session {
    fn new(id: SessionId, command: String, pty: PtyHandle, cols: u16, rows: u16) -> Self {
        let stream = Arc::new(StreamBuffer::new(STREAM_MAX_BUFFER_BYTES));
        let mut pty = PtySession::new(pty);
        let pty_rx = pty.take_read_rx();
        Self {
            id,
            command,
            created_at: Utc::now(),
            pty,
            terminal: TerminalState::new(cols, rows),
            held_modifiers: ModifierState::default(),
            stream,
            pty_rx,
            pty_cursor: Arc::new(Mutex::new(StreamCursor::default())),
            pump_tx: None,
            pump_join: None,
        }
    }

    pub fn pid(&self) -> Option<u32> {
        self.pty.pid()
    }

    pub fn is_running(&mut self) -> bool {
        self.pty.is_running()
    }

    pub fn size(&self) -> (u16, u16) {
        self.terminal.size()
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

    pub fn cursor(&self) -> CursorPosition {
        self.terminal.cursor()
    }

    pub fn keystroke(&self, key: &str) -> Result<(), SessionError> {
        let seq =
            key_to_escape_sequence(key).ok_or_else(|| SessionError::InvalidKey(key.to_string()))?;
        self.pty.write(&seq)?;
        Ok(())
    }

    pub fn keydown(&mut self, key: &str) -> Result<(), SessionError> {
        let modifier = ModifierKey::from_str(key).ok_or_else(|| {
            SessionError::InvalidKey(format!(
                "{}. Only modifier keys (Ctrl, Alt, Shift, Meta) can be held",
                key
            ))
        })?;
        self.held_modifiers.set(modifier, true);
        Ok(())
    }

    pub fn keyup(&mut self, key: &str) -> Result<(), SessionError> {
        let modifier = ModifierKey::from_str(key).ok_or_else(|| {
            SessionError::InvalidKey(format!(
                "{}. Only modifier keys (Ctrl, Alt, Shift, Meta) can be released",
                key
            ))
        })?;
        self.held_modifiers.set(modifier, false);
        Ok(())
    }

    pub fn type_text(&self, text: &str) -> Result<(), SessionError> {
        self.pty.write_str(text)?;
        Ok(())
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), SessionError> {
        self.pty.resize(cols, rows)?;
        self.terminal.resize(cols, rows);
        self.stream.notify();
        Ok(())
    }

    pub fn kill(&mut self) -> Result<(), SessionError> {
        self.pty.kill()?;
        Ok(())
    }

    pub fn pty_write(&self, data: &[u8]) -> Result<(), SessionError> {
        self.pty.write(data)?;
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

    pub fn pty_cursor_handle(&self) -> Arc<Mutex<StreamCursor>> {
        Arc::clone(&self.pty_cursor)
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
                self.stream.close(None);
                let _ = self.pty.is_running();
                false
            }
            ReadEvent::Error(error) => {
                self.stream.close(Some(error));
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
        let (cols, rows) = self.terminal.size();
        let buffer = self.terminal.screen_buffer();
        let cursor = self.terminal.cursor();
        let seq = render_live_preview_init(&buffer, &cursor);
        let stream_seq = self.stream.latest_seq();
        LivePreviewSnapshot {
            cols,
            rows,
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
}

pub const DEFAULT_MAX_SESSIONS: usize = 16;

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self::with_max_sessions(DEFAULT_MAX_SESSIONS)
    }

    pub fn with_max_sessions(max_sessions: usize) -> Self {
        let persistence = SessionPersistence::new();
        if let Err(e) = persistence.cleanup_stale_sessions() {
            warn!(error = %e, "Failed to cleanup stale sessions");
        }

        Self {
            sessions: RwLock::new(HashMap::new()),
            active_session: RwLock::new(None),
            persistence,
            max_sessions,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        &self,
        command: &str,
        args: &[String],
        cwd: Option<&str>,
        env: Option<&HashMap<String, String>>,
        session_id: Option<String>,
        cols: u16,
        rows: u16,
    ) -> Result<(SessionId, u32), SessionError> {
        if let Some(ref requested_id) = session_id {
            let sessions = rwlock_read_or_recover(&self.sessions);
            let id = SessionId::new(requested_id.clone());
            if sessions.contains_key(&id) {
                return Err(SessionError::AlreadyExists(requested_id.clone()));
            }
        }

        {
            let sessions = rwlock_read_or_recover(&self.sessions);
            if sessions.len() >= self.max_sessions {
                return Err(SessionError::LimitReached(self.max_sessions));
            }
        }

        let id = session_id
            .map(SessionId::new)
            .unwrap_or_else(generate_session_id);

        let pty = PtyHandle::spawn(command, args, cwd, env, cols, rows)
            .map_err(|e| SessionError::Terminal(e.into_port_error()))?;
        let pid = pty.pid().unwrap_or(0);

        let session = Session::new(id.clone(), command.to_string(), pty, cols, rows);
        let session = Arc::new(Mutex::new(session));

        let created_at = Utc::now().to_rfc3339();
        let persisted = PersistedSession {
            id: id.to_string(),
            command: command.to_string(),
            pid,
            created_at,
            cols,
            rows,
        };

        {
            let mut sessions = rwlock_write_or_recover(&self.sessions);
            sessions.insert(id.clone(), Arc::clone(&session));
        }

        {
            let mut active = rwlock_write_or_recover(&self.active_session);
            *active = Some(id.clone());
        }

        let thread_name = format!("session-pump-{}", id.as_str());
        let (pump_tx, pump_join) = spawn_pump(Arc::clone(&session), thread_name);
        {
            let mut sess = mutex_lock_or_recover(&session);
            sess.attach_pump(pump_tx, pump_join);
        }

        if let Err(e) = self.persistence.add_session(persisted) {
            warn!(error = %e, "Failed to persist session metadata");
        }

        Ok((id, pid))
    }

    pub fn get(&self, session_id: &str) -> Result<Arc<Mutex<Session>>, SessionError> {
        let sessions = rwlock_read_or_recover(&self.sessions);
        let id = SessionId::new(session_id);
        sessions
            .get(&id)
            .cloned()
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))
    }

    pub fn active(&self) -> Result<Arc<Mutex<Session>>, SessionError> {
        let active_id = {
            let active = rwlock_read_or_recover(&self.active_session);
            active.clone()
        };

        match active_id {
            Some(id) => self.get(id.as_str()),
            None => Err(SessionError::NoActiveSession),
        }
    }

    pub fn resolve(&self, session_id: Option<&str>) -> Result<Arc<Mutex<Session>>, SessionError> {
        match session_id {
            Some(id) => self.get(id),
            None => self.active(),
        }
    }

    pub fn set_active(&self, session_id: &str) -> Result<(), SessionError> {
        let id = SessionId::new(session_id);
        let sessions = rwlock_read_or_recover(&self.sessions);
        if !sessions.contains_key(&id) {
            return Err(SessionError::NotFound(session_id.to_string()));
        }
        let mut active = rwlock_write_or_recover(&self.active_session);
        *active = Some(id);
        Ok(())
    }

    pub fn list(&self) -> Vec<SessionInfo> {
        use super::lock_helpers::acquire_session_lock;

        let session_refs: Vec<(SessionId, Arc<Mutex<Session>>)> = {
            let sessions = rwlock_read_or_recover(&self.sessions);
            sessions
                .iter()
                .map(|(id, session)| (id.clone(), Arc::clone(session)))
                .collect()
        };

        session_refs
            .into_iter()
            .map(
                |(id, session)| match acquire_session_lock(&session, Duration::from_millis(100)) {
                    Some(mut sess) => {
                        let (cols, rows) = sess.size();
                        SessionInfo {
                            id: id.clone(),
                            command: sess.command.clone(),
                            pid: sess.pid().unwrap_or(0),
                            running: sess.is_running(),
                            created_at: sess.created_at.to_rfc3339(),
                            size: TerminalSize::try_new(cols, rows).unwrap_or_default(),
                        }
                    }
                    None => SessionInfo {
                        id: id.clone(),
                        command: "(locked)".to_string(),
                        pid: 0,
                        running: false,
                        created_at: "".to_string(),
                        size: TerminalSize::default(),
                    },
                },
            )
            .collect()
    }

    pub fn kill(&self, session_id: &str) -> Result<(), SessionError> {
        let id = SessionId::new(session_id);

        let session = {
            let mut sessions = rwlock_write_or_recover(&self.sessions);
            let mut active = rwlock_write_or_recover(&self.active_session);

            let session = sessions
                .remove(&id)
                .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;

            if active.as_ref() == Some(&id) {
                *active = None;
            }

            session
        };

        {
            let mut sess = mutex_lock_or_recover(&session);
            let join = sess.shutdown_pump();
            sess.kill()?;
            drop(sess);
            if let Some(join) = join {
                let _ = join.join();
            }
        }

        if let Err(e) = self.persistence.remove_session(session_id) {
            warn!(session_id = session_id, error = %e, "Failed to remove session from persistence");
        }

        Ok(())
    }

    pub fn session_count(&self) -> usize {
        rwlock_read_or_recover(&self.sessions).len()
    }

    pub fn active_session_id(&self) -> Option<SessionId> {
        rwlock_read_or_recover(&self.active_session).clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSession {
    pub id: String,
    pub command: String,
    pub pid: u32,
    pub created_at: String,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum SessionEvent {
    Upsert { session: PersistedSession },
    Remove { session_id: String },
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
            .unwrap_or_else(|_| PathBuf::from("/tmp"));
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

            std::thread::sleep(backoff);
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
                reason: format!("Failed to serialize session event: {}", e),
                source: Some(Box::new(e)),
            })?;
            writeln!(file, "{}", line).map_err(|e| Self::io_to_persistence("write_event", e))?;
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

    fn load_unlocked(&self) -> Vec<PersistedSession> {
        if !self.path.exists() {
            return Vec::new();
        }
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(e) => {
                warn!(
                    path = %self.path.display(),
                    error = %e,
                    "Failed to open sessions log"
                );
                return Vec::new();
            }
        };

        let mut sessions = HashMap::new();
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = match line {
                Ok(line) => line,
                Err(e) => {
                    warn!(error = %e, "Failed to read session log line");
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
                    continue;
                }
            };
            match event {
                SessionEvent::Upsert { session } => {
                    sessions.insert(session.id.clone(), session);
                }
                SessionEvent::Remove { session_id } => {
                    sessions.remove(&session_id);
                }
            }
        }

        sessions.into_values().collect()
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
            reason: format!("Failed to serialize session event: {}", e),
            source: Some(Box::new(e)),
        })?;
        writeln!(file, "{}", line).map_err(|e| Self::io_to_persistence("write_event", e))?;
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
                reason: format!("Failed to serialize session event: {}", e),
                source: Some(Box::new(e)),
            })?;
            writeln!(writer, "{}", line).map_err(|e| Self::io_to_persistence("write_event", e))?;
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
        let sessions = self.load_unlocked();
        self.save_unlocked(&sessions)?;
        Ok(())
    }

    pub fn load(&self) -> Vec<PersistedSession> {
        match self.acquire_lock() {
            Ok(_lock) => {
                let _ = self.migrate_legacy_if_needed_locked();
                self.load_unlocked()
            }
            Err(e) => {
                warn!(error = %e, "Failed to acquire lock for loading sessions");
                self.load_unlocked()
            }
        }
    }

    pub fn save(&self, sessions: &[PersistedSession]) -> Result<(), SessionError> {
        let _lock = self.acquire_lock()?;
        self.migrate_legacy_if_needed_locked()?;
        self.save_unlocked(sessions)
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
        let sessions = self.load_unlocked();
        let mut cleaned = 0;

        let mut active_sessions = Vec::new();
        for session in sessions {
            if session.pid == 0 {
                cleaned += 1;
                continue;
            }

            reap_child_if_any(session.pid);
            if !is_process_running(session.pid) {
                cleaned += 1;
                continue;
            }

            match verify_persisted_session_identity(&session) {
                ProcessIdentity::Match => {
                    let _ = terminate_process_group(session.pid);
                    reap_child_if_any(session.pid);
                    if !is_process_running(session.pid) {
                        cleaned += 1;
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

        self.save_unlocked(&active_sessions)?;
        Ok(cleaned)
    }
}

#[cfg(test)]
mod stream_tests {
    use super::StreamBuffer;
    use super::StreamCursor;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn stream_read_returns_data_and_advances_cursor() {
        let buffer = StreamBuffer::new(16);
        let mut cursor = StreamCursor::default();

        buffer.push(b"hello");
        let read = buffer.read(&mut cursor, 16, 0).unwrap();

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
        let read = buffer.read(&mut cursor, 10, 0).unwrap();

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
            thread::sleep(Duration::from_millis(50));
            buffer_clone.push(b"ok");
        });

        let read = buffer.read(&mut cursor, 16, 200).unwrap();
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

        let read_a = buffer.read(&mut cursor_a, 2, 0).unwrap();
        let read_b = buffer.read(&mut cursor_b, 16, 0).unwrap();

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
        buffer.close(None);
        assert!(subscription.wait(Some(Duration::from_millis(50))));
    }
}

#[cfg(test)]
mod pump_tests {
    use super::Session;
    use super::StreamCursor;
    use super::spawn_pump;
    use crate::infra::terminal::PtyHandle;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::time::Duration;
    use std::time::Instant;

    #[cfg(unix)]
    #[test]
    fn session_pump_streams_output_into_buffer() {
        let args = vec!["-c".to_string(), "printf 'hi'".to_string()];
        let pty = PtyHandle::spawn("sh", &args, None, None, 80, 24).unwrap();
        let session = Session::new("test-session".into(), "sh".to_string(), pty, 80, 24);
        let session = Arc::new(Mutex::new(session));

        let (tx, join) = spawn_pump(Arc::clone(&session), "test-pump".to_string());
        {
            let mut guard = session.lock().unwrap();
            guard.attach_pump(tx, join);
        }

        let reader = { session.lock().unwrap().stream_reader() };
        let mut cursor = StreamCursor::default();
        let deadline = Instant::now() + Duration::from_millis(500);
        let mut collected = Vec::new();

        while Instant::now() < deadline {
            let read = reader.read(&mut cursor, 64, 10).unwrap();
            if !read.data.is_empty() {
                collected.extend_from_slice(&read.data);
            }
            if String::from_utf8_lossy(&collected).contains("hi") {
                break;
            }
        }

        assert!(String::from_utf8_lossy(&collected).contains("hi"));

        let join = { session.lock().unwrap().shutdown_pump() };
        let _ = session.lock().unwrap().kill();
        if let Some(join) = join {
            let _ = join.join();
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

    if let (Some(cmdline), Some(expected)) =
        (info.cmdline.as_ref(), expected_command(&session.command))
        && !cmdline.contains(expected)
    {
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
        let cmdline = cmd
            .iter()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        Some(cmdline)
    } else {
        process.exe().map(|path| path.to_string_lossy().to_string())
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

fn parse_cmdline_bytes(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let mut cmdline = String::from_utf8_lossy(bytes).replace('\0', " ");
    cmdline = cmdline.trim().to_string();
    if cmdline.is_empty() {
        None
    } else {
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
        std::thread::sleep(STARTUP_KILL_POLL_INTERVAL);
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
            cols: info.size.cols(),
            rows: info.size.rows(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    struct HomeGuard(Option<String>);

    impl Drop for HomeGuard {
        fn drop(&mut self) {
            if let Some(home) = self.0.take() {
                // SAFETY: Test-only environment restoration after HOME override.
                unsafe {
                    std::env::set_var("HOME", home);
                }
            } else {
                // SAFETY: Test-only cleanup of HOME override.
                unsafe {
                    std::env::remove_var("HOME");
                }
            }
        }
    }

    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prev = std::env::var(key).ok();
            // SAFETY: Test-only environment override.
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, prev }
        }

        fn remove(key: &'static str) -> Self {
            let prev = std::env::var(key).ok();
            // SAFETY: Test-only environment override.
            unsafe {
                std::env::remove_var(key);
            }
            Self { key, prev }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(prev) = self.prev.take() {
                // SAFETY: Test-only environment restoration.
                unsafe {
                    std::env::set_var(self.key, prev);
                }
            } else {
                // SAFETY: Test-only environment cleanup.
                unsafe {
                    std::env::remove_var(self.key);
                }
            }
        }
    }

    #[test]
    fn test_persisted_session_serialization() {
        let session = PersistedSession {
            id: "test123".to_string(),
            command: "bash".to_string(),
            pid: 12345,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            cols: 80,
            rows: 24,
        };

        let json = serde_json::to_string(&session).unwrap();
        let parsed: PersistedSession = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, session.id);
        assert_eq!(parsed.command, session.command);
        assert_eq!(parsed.pid, session.pid);
    }

    #[test]
    fn test_is_process_running() {
        let current_pid = std::process::id();
        assert!(is_process_running(current_pid));

        assert!(!is_process_running(999999999));
    }

    #[test]
    fn test_process_info_current_pid() {
        let current_pid = std::process::id();
        let info = process_info(current_pid);
        assert!(info.is_some());
    }

    #[test]
    fn test_spawn_rejects_duplicate_session_id() {
        let temp_home = tempdir().unwrap();
        let _home_guard = HomeGuard(std::env::var("HOME").ok());
        // SAFETY: Test-only environment override for HOME directory.
        unsafe {
            std::env::set_var("HOME", temp_home.path());
        }

        let manager = SessionManager::with_max_sessions(2);
        let session_id = "dup-session".to_string();
        match manager.spawn("sh", &[], None, None, Some(session_id.clone()), 80, 24) {
            Ok(_) => {}
            Err(SessionError::Terminal(_)) => return, // PTY unavailable, skip
            Err(e) => panic!("unexpected error from first spawn: {e}"),
        }

        let result = manager.spawn("sh", &[], None, None, Some(session_id.clone()), 80, 24);

        assert!(matches!(
            result,
            Err(SessionError::AlreadyExists(id)) if id == session_id
        ));

        let _ = manager.kill(&session_id);
    }

    #[test]
    fn test_persistence_migration_from_json() {
        let temp_home = tempdir().unwrap();
        let _home_guard = HomeGuard(std::env::var("HOME").ok());
        // SAFETY: Test-only environment override for HOME directory.
        unsafe {
            std::env::set_var("HOME", temp_home.path());
        }
        let _store_guard = EnvGuard::remove("AGENT_TUI_SESSION_STORE");

        let legacy_dir = temp_home.path().join(".agent-tui");
        fs::create_dir_all(&legacy_dir).unwrap();
        let legacy_path = legacy_dir.join("sessions.json");
        let sessions = vec![PersistedSession {
            id: "legacy".to_string(),
            command: "sh".to_string(),
            pid: 1234,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            cols: 80,
            rows: 24,
        }];
        fs::write(&legacy_path, serde_json::to_string(&sessions).unwrap()).unwrap();

        let persistence = SessionPersistence::new();
        let loaded = persistence.load();
        assert_eq!(loaded.len(), 1);

        let jsonl_path = legacy_dir.join("sessions.jsonl");
        let backup_path = legacy_dir.join("sessions.json.bak");
        assert!(jsonl_path.exists());
        assert!(backup_path.exists());
    }

    #[test]
    fn test_jsonl_add_remove_roundtrip() {
        let temp_home = tempdir().unwrap();
        let _home_guard = HomeGuard(std::env::var("HOME").ok());
        // SAFETY: Test-only environment override for HOME directory.
        unsafe {
            std::env::set_var("HOME", temp_home.path());
        }
        let _store_guard = EnvGuard::remove("AGENT_TUI_SESSION_STORE");

        let persistence = SessionPersistence::new();
        let session = PersistedSession {
            id: "roundtrip".to_string(),
            command: "bash".to_string(),
            pid: 777,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            cols: 100,
            rows: 40,
        };
        persistence.add_session(session.clone()).unwrap();
        let loaded = persistence.load();
        assert!(loaded.iter().any(|s| s.id == session.id));

        persistence.remove_session(&session.id).unwrap();
        let loaded = persistence.load();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_session_store_env_override() {
        let temp_home = tempdir().unwrap();
        let _home_guard = HomeGuard(std::env::var("HOME").ok());
        // SAFETY: Test-only environment override for HOME directory.
        unsafe {
            std::env::set_var("HOME", temp_home.path());
        }
        let store_path = temp_home.path().join("custom-sessions.jsonl");
        let _store_guard = EnvGuard::set(
            "AGENT_TUI_SESSION_STORE",
            store_path.to_string_lossy().as_ref(),
        );

        let persistence = SessionPersistence::new();
        persistence
            .add_session(PersistedSession {
                id: "custom".to_string(),
                command: "sh".to_string(),
                pid: 456,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                cols: 80,
                rows: 24,
            })
            .unwrap();

        assert!(store_path.exists());
        let default_path = temp_home.path().join(".agent-tui").join("sessions.jsonl");
        assert!(!default_path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_startup_cleanup_kills_persisted_session_process_group() {
        use std::os::unix::process::CommandExt;
        use std::process::Command;

        let temp_home = tempdir().unwrap();
        let _home_guard = HomeGuard(std::env::var("HOME").ok());
        // SAFETY: Test-only environment override for HOME directory.
        unsafe {
            std::env::set_var("HOME", temp_home.path());
        }

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("sleep 10");
        // SAFETY: pre-exec runs in the child before exec.
        unsafe {
            cmd.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        let mut child = cmd.spawn().expect("failed to spawn test child");
        let pid = child.id();
        assert!(pid > 0);

        let persistence = SessionPersistence::new();
        persistence
            .add_session(PersistedSession {
                id: "orphan".to_string(),
                command: "sleep".to_string(),
                pid,
                created_at: Utc::now().to_rfc3339(),
                cols: 80,
                rows: 24,
            })
            .expect("failed to persist session");

        let _manager = SessionManager::with_max_sessions(1);

        let deadline = Instant::now() + Duration::from_secs(2);
        while is_process_running(pid) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(25));
        }

        if is_process_running(pid) {
            let pid_t: libc::pid_t = pid.try_into().unwrap_or(0);
            if pid_t > 0 {
                // SAFETY: negative pid targets the process group for cleanup.
                unsafe {
                    libc::kill(-pid_t, libc::SIGKILL);
                }
            }
        }

        let _ = child.wait();

        assert!(!is_process_running(pid));

        let sessions = persistence.load();
        assert!(sessions.is_empty());
    }
}
