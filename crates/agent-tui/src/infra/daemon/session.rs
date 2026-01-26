use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::BufReader;
use std::io::BufWriter;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::time::Duration;
use std::time::Instant;

use avt::Vt;
use tracing::warn;

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::common::mutex_lock_or_recover;
use crate::common::rwlock_read_or_recover;
use crate::common::rwlock_write_or_recover;
use crate::domain::core::Element;
use crate::infra::terminal::CursorPosition;
use crate::infra::terminal::PtyHandle;
use crate::infra::terminal::key_to_escape_sequence;
use crate::infra::terminal::render_screen;
use crate::usecases::ports::{LivePreviewOutput, LivePreviewSnapshot};

use super::pty_session::PtySession;
use crate::infra::daemon::TerminalState;

pub use crate::domain::session_types::SessionId;
pub use crate::domain::session_types::SessionInfo;
pub use crate::infra::daemon::SessionError;

const LIVE_PREVIEW_MAX_BUFFER_BYTES: usize = 4 * 1024 * 1024;

pub fn generate_session_id() -> SessionId {
    SessionId::new(Uuid::new_v4().to_string()[..8].to_string())
}

struct LivePreviewBuffer {
    vt: Vt,
    cols: u16,
    rows: u16,
    pending: VecDeque<String>,
    pending_bytes: usize,
    dropped_bytes: u64,
}

impl LivePreviewBuffer {
    fn new(cols: u16, rows: u16) -> Self {
        Self {
            vt: build_preview_vt(cols, rows),
            cols,
            rows,
            pending: VecDeque::new(),
            pending_bytes: 0,
            dropped_bytes: 0,
        }
    }

    fn process(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        let text = String::from_utf8_lossy(data);
        if text.is_empty() {
            return;
        }
        self.vt.feed_str(&text);
        self.push_text(text.as_ref());
    }

    fn snapshot(&self) -> (u16, u16, String) {
        (self.cols, self.rows, self.vt.dump())
    }

    fn drain_output(&mut self) -> (String, u64) {
        let mut seq = String::new();
        if self.pending_bytes > 0 {
            seq.reserve(self.pending_bytes);
        }
        for chunk in self.pending.drain(..) {
            seq.push_str(&chunk);
        }
        self.pending_bytes = 0;
        let dropped_bytes = std::mem::take(&mut self.dropped_bytes);
        (seq, dropped_bytes)
    }

    fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
        self.vt.resize(cols as usize, rows as usize);
    }

    fn push_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.pending_bytes += text.len();
        self.pending.push_back(text.to_string());
        while self.pending_bytes > LIVE_PREVIEW_MAX_BUFFER_BYTES {
            if let Some(front) = self.pending.pop_front() {
                self.pending_bytes -= front.len();
                self.dropped_bytes += front.len() as u64;
            } else {
                self.pending_bytes = 0;
                break;
            }
        }
    }
}

fn build_preview_vt(cols: u16, rows: u16) -> Vt {
    Vt::builder().size(cols as usize, rows as usize).build()
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
    live_preview: LivePreviewBuffer,
}

impl Session {
    fn new(id: SessionId, command: String, pty: PtyHandle, cols: u16, rows: u16) -> Self {
        Self {
            id,
            command,
            created_at: Utc::now(),
            pty: PtySession::new(pty),
            terminal: TerminalState::new(cols, rows),
            held_modifiers: ModifierState::default(),
            live_preview: LivePreviewBuffer::new(cols, rows),
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

    pub fn update(&mut self) -> Result<(), SessionError> {
        let mut buf = [0u8; 4096];

        loop {
            match self.pty.try_read(&mut buf, 10) {
                Ok(0) => break,
                Ok(n) => {
                    self.terminal.process(&buf[..n]);
                    self.live_preview.process(&buf[..n]);
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("Resource temporarily unavailable")
                        || err_str.contains("EAGAIN")
                        || err_str.contains("EWOULDBLOCK")
                    {
                        break;
                    }

                    return Err(e);
                }
            }
        }

        Ok(())
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

    pub fn detect_elements(&mut self) -> &[Element] {
        let cursor = self.terminal.cursor();
        self.terminal.detect_elements(&cursor)
    }

    pub fn cached_elements(&self) -> &[Element] {
        self.terminal.cached_elements()
    }

    pub fn find_element(&self, element_ref: &str) -> Option<&Element> {
        self.terminal.find_element(element_ref)
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

    pub fn click(&mut self, element_ref: &str) -> Result<(), SessionError> {
        self.update()?;
        self.detect_elements();

        let element = self
            .find_element(element_ref)
            .ok_or_else(|| SessionError::ElementNotFound(element_ref.to_string()))?;

        match element.element_type.as_str() {
            "checkbox" | "radio" => {
                self.pty.write(b" ")?;
            }
            _ => {
                self.pty.write(b"\r")?;
            }
        }

        Ok(())
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), SessionError> {
        self.pty.resize(cols, rows)?;
        self.terminal.resize(cols, rows);
        self.live_preview.resize(cols, rows);
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

    pub fn pty_try_read(&mut self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, SessionError> {
        let bytes_read = self.pty.try_read(buf, timeout_ms)?;
        if bytes_read > 0 {
            let data = &buf[..bytes_read];
            self.terminal.process(data);
            self.live_preview.process(data);
        }
        Ok(bytes_read)
    }

    pub fn live_preview_snapshot(&self) -> LivePreviewSnapshot {
        let (cols, rows, seq) = self.live_preview.snapshot();
        LivePreviewSnapshot { cols, rows, seq }
    }

    pub fn live_preview_drain_output(&mut self) -> LivePreviewOutput {
        let (seq, dropped_bytes) = self.live_preview.drain_output();
        LivePreviewOutput { seq, dropped_bytes }
    }

    pub fn analyze_screen(&self) -> Vec<crate::domain::core::Component> {
        let cursor = self.terminal.cursor();
        self.terminal.analyze_screen(&cursor)
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

        let pty = PtyHandle::spawn(command, args, cwd, env, cols, rows)?;
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
            sessions.insert(id.clone(), session);
        }

        {
            let mut active = rwlock_write_or_recover(&self.active_session);
            *active = Some(id.clone());
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
                    Some(mut sess) => SessionInfo {
                        id: id.clone(),
                        command: sess.command.clone(),
                        pid: sess.pid().unwrap_or(0),
                        running: sess.is_running(),
                        created_at: sess.created_at.to_rfc3339(),
                        size: sess.size(),
                    },
                    None => SessionInfo {
                        id: id.clone(),
                        command: "(locked)".to_string(),
                        pid: 0,
                        running: true,
                        created_at: "".to_string(),
                        size: (80, 24),
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
            sess.kill()?;
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

pub struct SessionPersistence {
    path: PathBuf,
    lock_path: PathBuf,
}

impl SessionPersistence {
    pub fn new() -> Self {
        let path = Self::sessions_file_path();
        let lock_path = path.with_extension("json.lock");
        Self { path, lock_path }
    }

    fn sessions_file_path() -> PathBuf {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp"));
        let dir = home.join(".agent-tui");
        dir.join("sessions.json")
    }

    fn io_to_persistence(operation: &str, e: std::io::Error) -> SessionError {
        SessionError::Persistence {
            operation: operation.to_string(),
            reason: e.to_string(),
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
                });
            }

            std::thread::sleep(backoff);
            backoff = (backoff * 2).min(Duration::from_millis(100));
        }
    }

    fn load_unlocked(&self) -> Vec<PersistedSession> {
        if !self.path.exists() {
            return Vec::new();
        }

        match File::open(&self.path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                match serde_json::from_reader(reader) {
                    Ok(sessions) => sessions,
                    Err(e) => {
                        warn!(
                            path = %self.path.display(),
                            error = %e,
                            "Sessions file corrupted, starting with empty session list"
                        );
                        Vec::new()
                    }
                }
            }
            Err(e) => {
                warn!(
                    path = %self.path.display(),
                    error = %e,
                    "Failed to open sessions file"
                );
                Vec::new()
            }
        }
    }

    fn save_unlocked(&self, sessions: &[PersistedSession]) -> Result<(), SessionError> {
        let temp_path = self.path.with_extension("json.tmp");

        let file = File::create(&temp_path).map_err(|e| SessionError::Persistence {
            operation: "create_temp".to_string(),
            reason: format!(
                "Failed to create temp file '{}': {}",
                temp_path.display(),
                e
            ),
        })?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, sessions).map_err(|e| SessionError::Persistence {
            operation: "write_json".to_string(),
            reason: format!(
                "Failed to write sessions to '{}': {}",
                temp_path.display(),
                e
            ),
        })?;

        fs::rename(&temp_path, &self.path).map_err(|e| SessionError::Persistence {
            operation: "rename".to_string(),
            reason: format!(
                "Failed to rename '{}' to '{}': {}",
                temp_path.display(),
                self.path.display(),
                e
            ),
        })?;

        Ok(())
    }

    pub fn load(&self) -> Vec<PersistedSession> {
        match self.acquire_lock() {
            Ok(_lock) => self.load_unlocked(),
            Err(e) => {
                warn!(error = %e, "Failed to acquire lock for loading sessions");
                self.load_unlocked()
            }
        }
    }

    pub fn save(&self, sessions: &[PersistedSession]) -> Result<(), SessionError> {
        let _lock = self.acquire_lock()?;
        self.save_unlocked(sessions)
    }

    pub fn add_session(&self, session: PersistedSession) -> Result<(), SessionError> {
        let _lock = self.acquire_lock()?;
        let mut sessions = self.load_unlocked();

        sessions.retain(|s| s.id != session.id);
        sessions.push(session);

        self.save_unlocked(&sessions)
    }

    pub fn remove_session(&self, session_id: &str) -> Result<(), SessionError> {
        let _lock = self.acquire_lock()?;
        let mut sessions = self.load_unlocked();
        sessions.retain(|s| s.id.as_str() != session_id);
        self.save_unlocked(&sessions)
    }

    pub fn cleanup_stale_sessions(&self) -> Result<usize, SessionError> {
        let _lock = self.acquire_lock()?;
        let sessions = self.load_unlocked();
        let mut cleaned = 0;

        let active_sessions: Vec<PersistedSession> = sessions
            .into_iter()
            .filter(|s| {
                let running = is_process_running(s.pid);
                if !running {
                    cleaned += 1;
                }
                running
            })
            .collect();

        self.save_unlocked(&active_sessions)?;
        Ok(cleaned)
    }
}

impl Default for SessionPersistence {
    fn default() -> Self {
        Self::new()
    }
}

fn is_process_running(pid: u32) -> bool {
    // SAFETY: `kill` with signal 0 performs a permission check without sending any signal.
    // This is a standard POSIX idiom to check if a process exists. The pid is a u32 cast
    // to i32, which is safe because valid PIDs are always positive and fit in i32.
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

impl From<&SessionInfo> for PersistedSession {
    fn from(info: &SessionInfo) -> Self {
        PersistedSession {
            id: info.id.to_string(),
            command: info.command.clone(),
            pid: info.pid,
            created_at: info.created_at.clone(),
            cols: info.size.0,
            rows: info.size.1,
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
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
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
    fn test_spawn_rejects_duplicate_session_id() {
        let temp_home = tempdir().unwrap();
        let _home_guard = HomeGuard(std::env::var("HOME").ok());
        std::env::set_var("HOME", temp_home.path());

        let manager = SessionManager::with_max_sessions(2);
        let session_id = "dup-session".to_string();
        let _ = manager
            .spawn("sh", &[], None, None, Some(session_id.clone()), 80, 24)
            .unwrap();

        let result = manager.spawn("sh", &[], None, None, Some(session_id.clone()), 80, 24);

        assert!(matches!(
            result,
            Err(SessionError::AlreadyExists(id)) if id == session_id
        ));

        let _ = manager.kill(&session_id);
    }
}
