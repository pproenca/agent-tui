use crate::detection::{Element, ElementDetector};
use crate::pty::{key_to_escape_sequence, PtyError, PtyHandle};
use crate::sync_utils::{mutex_lock_or_recover, rwlock_read_or_recover, rwlock_write_or_recover};
use crate::terminal::{CursorPosition, VirtualTerminal};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;
use thiserror::Error;
use uuid::Uuid;

fn get_last_n<T: Clone>(queue: &VecDeque<T>, count: usize) -> Vec<T> {
    let start = queue.len().saturating_sub(count);
    queue.iter().skip(start).cloned().collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(String);

impl SessionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string()[..8].to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for SessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct RecordingFrame {
    pub timestamp_ms: u64,
    pub screen: String,
}

struct RecordingState {
    is_recording: bool,
    start_time: Instant,
    frames: Vec<RecordingFrame>,
}

impl RecordingState {
    fn new() -> Self {
        Self {
            is_recording: false,
            start_time: Instant::now(),
            frames: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TraceEntry {
    pub timestamp_ms: u64,
    pub action: String,
    pub details: Option<String>,
}

struct TraceState {
    is_tracing: bool,
    start_time: Instant,
    entries: VecDeque<TraceEntry>,
}

impl TraceState {
    fn new() -> Self {
        Self {
            is_tracing: false,
            start_time: Instant::now(),
            entries: VecDeque::new(),
        }
    }
}

pub struct RecordingStatus {
    pub is_recording: bool,
    pub frame_count: usize,
    pub duration_ms: u64,
}

#[derive(Clone, Debug)]
pub struct ErrorEntry {
    pub timestamp: String,
    pub message: String,
    pub source: String,
}

struct ErrorState {
    entries: VecDeque<ErrorEntry>,
}

impl ErrorState {
    fn new() -> Self {
        Self {
            entries: VecDeque::new(),
        }
    }
}

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("No active session")]
    NoActiveSession,
    #[error("PTY error: {0}")]
    Pty(#[from] PtyError),
    #[error("Element not found: {0}")]
    ElementNotFound(String),
    #[error("Invalid key: {0}")]
    InvalidKey(String),
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
    pty: PtyHandle,
    terminal: VirtualTerminal,
    detector: ElementDetector,
    cached_elements: Vec<Element>,
    recording: RecordingState,
    trace: TraceState,
    held_modifiers: ModifierState,
    errors: ErrorState,
}

impl Session {
    fn new(id: SessionId, command: String, pty: PtyHandle, cols: u16, rows: u16) -> Self {
        Self {
            id,
            command,
            created_at: Utc::now(),
            pty,
            terminal: VirtualTerminal::new(cols, rows),
            detector: ElementDetector::new(),
            cached_elements: Vec::new(),
            recording: RecordingState::new(),
            trace: TraceState::new(),
            held_modifiers: ModifierState::default(),
            errors: ErrorState::new(),
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
                }
                Err(_) => break,
            }
        }

        Ok(())
    }

    pub fn screen_text(&self) -> String {
        self.terminal.screen_text()
    }

    pub fn cursor(&self) -> CursorPosition {
        self.terminal.cursor()
    }

    pub fn detect_elements(&mut self) -> &[Element] {
        let screen_text = self.terminal.screen_text();
        let screen_buffer = self.terminal.screen_buffer();
        self.cached_elements = self.detector.detect(&screen_text, Some(&screen_buffer));
        &self.cached_elements
    }

    pub fn cached_elements(&self) -> &[Element] {
        &self.cached_elements
    }

    pub fn find_element(&self, element_ref: &str) -> Option<&Element> {
        self.detector
            .find_by_ref(&self.cached_elements, element_ref)
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

    pub fn fill(&mut self, element_ref: &str, value: &str) -> Result<(), SessionError> {
        self.update()?;
        self.detect_elements();

        let _element = self
            .find_element(element_ref)
            .ok_or_else(|| SessionError::ElementNotFound(element_ref.to_string()))?;

        self.pty.write_str(value)?;

        Ok(())
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), SessionError> {
        self.pty.resize(cols, rows)?;
        self.terminal.resize(cols, rows);
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

    pub fn pty_try_read(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, SessionError> {
        self.pty
            .try_read(buf, timeout_ms)
            .map_err(SessionError::Pty)
    }

    pub fn start_recording(&mut self) {
        self.recording.is_recording = true;
        self.recording.start_time = Instant::now();
        self.recording.frames.clear();

        let screen = self.terminal.screen_text();
        self.recording.frames.push(RecordingFrame {
            timestamp_ms: 0,
            screen,
        });
    }

    pub fn stop_recording(&mut self) -> Vec<RecordingFrame> {
        self.recording.is_recording = false;
        std::mem::take(&mut self.recording.frames)
    }

    pub fn recording_status(&self) -> RecordingStatus {
        RecordingStatus {
            is_recording: self.recording.is_recording,
            frame_count: self.recording.frames.len(),
            duration_ms: if self.recording.is_recording {
                self.recording.start_time.elapsed().as_millis() as u64
            } else {
                0
            },
        }
    }

    pub fn start_trace(&mut self) {
        self.trace.is_tracing = true;
        self.trace.start_time = Instant::now();
        self.trace.entries.clear();
    }

    pub fn stop_trace(&mut self) {
        self.trace.is_tracing = false;
    }

    pub fn is_tracing(&self) -> bool {
        self.trace.is_tracing
    }

    pub fn get_trace_entries(&self, count: usize) -> Vec<TraceEntry> {
        get_last_n(&self.trace.entries, count)
    }

    pub fn get_errors(&self, count: usize) -> Vec<ErrorEntry> {
        get_last_n(&self.errors.entries, count)
    }

    pub fn error_count(&self) -> usize {
        self.errors.entries.len()
    }

    pub fn clear_errors(&mut self) {
        self.errors.entries.clear();
    }

    pub fn clear_console(&mut self) {
        self.terminal.clear();
    }

    // ========== VOM Integration ==========
    // VOM is in shadow mode - allow unused during verification phase

    /// Analyze the current screen using the Visual Object Model.
    /// Returns semantic components (buttons, inputs, tabs, etc.)
    #[allow(dead_code)]
    pub fn analyze_screen(&self) -> Vec<crate::vom::Component> {
        let buffer = self.terminal.screen_buffer();
        let cursor = self.terminal.cursor();
        crate::vom::analyze(&buffer, cursor.row, cursor.col)
    }

    /// Find a VOM component by text content (partial match)
    #[allow(dead_code)]
    pub fn find_vom_component(&self, text: &str) -> Option<crate::vom::Component> {
        let components = self.analyze_screen();
        crate::vom::find_by_text(&components, text).cloned()
    }

    /// Find all VOM components with a specific role
    #[allow(dead_code)]
    pub fn find_vom_by_role(&self, role: crate::vom::Role) -> Vec<crate::vom::Component> {
        let components = self.analyze_screen();
        crate::vom::find_by_role(&components, role)
            .into_iter()
            .cloned()
            .collect()
    }

    /// Check if the terminal supports mouse reporting
    #[allow(dead_code)]
    pub fn mouse_reporting_enabled(&self) -> bool {
        self.terminal.mouse_reporting_enabled()
    }

    /// Inject a mouse click at the center of a component.
    /// Uses SGR 1006 format mouse sequences.
    #[allow(dead_code)]
    pub fn click_vom_component(
        &mut self,
        component: &crate::vom::Component,
    ) -> Result<(), SessionError> {
        use crate::vom::interaction::{click_component, MouseButton};

        let seq = click_component(component, MouseButton::Left);
        self.pty.write(&seq)?;
        Ok(())
    }

    /// Inject a mouse click at specific coordinates.
    #[allow(dead_code)]
    pub fn inject_mouse_click(&mut self, x: u16, y: u16) -> Result<(), SessionError> {
        use crate::vom::interaction::{click_at, MouseButton};

        let seq = click_at(x, y, MouseButton::Left);
        self.pty.write(&seq)?;
        Ok(())
    }

    /// Compute the current layout signature for change detection.
    #[allow(dead_code)]
    pub fn layout_signature(&self) -> u64 {
        let components = self.analyze_screen();
        crate::vom::feedback::compute_layout_signature(&components)
    }

    /// Wait for the layout to change from a previous signature.
    /// Returns true if layout changed, false if timeout.
    #[allow(dead_code)]
    pub fn wait_for_layout_change(&mut self, old_sig: u64, timeout: std::time::Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;

        while std::time::Instant::now() < deadline {
            // Process any pending output
            let _ = self.update();

            let new_sig = self.layout_signature();
            if new_sig != old_sig {
                return true;
            }

            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        false
    }

    /// Click a VOM component and wait for the layout to change.
    /// Returns true if the click had a visible effect.
    #[allow(dead_code)]
    pub fn robust_vom_click(
        &mut self,
        component: &crate::vom::Component,
        timeout: std::time::Duration,
    ) -> Result<bool, SessionError> {
        // Capture layout before click
        let before_sig = self.layout_signature();

        // Perform the click
        self.click_vom_component(component)?;

        // Wait for layout to change
        Ok(self.wait_for_layout_change(before_sig, timeout))
    }
}

pub struct SessionManager {
    sessions: RwLock<HashMap<SessionId, Arc<Mutex<Session>>>>,
    active_session: RwLock<Option<SessionId>>,
    persistence: SessionPersistence,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    pub fn new() -> Self {
        let persistence = SessionPersistence::new();
        let _ = persistence.cleanup_stale_sessions();

        Self {
            sessions: RwLock::new(HashMap::new()),
            active_session: RwLock::new(None),
            persistence,
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
        let id = session_id
            .map(SessionId::new)
            .unwrap_or_else(SessionId::generate);

        let pty = PtyHandle::spawn(command, args, cwd, env, cols, rows)?;
        let pid = pty.pid().unwrap_or(0);

        let session = Session::new(id.clone(), command.to_string(), pty, cols, rows);
        let session = Arc::new(Mutex::new(session));

        let created_at = Utc::now().to_rfc3339();
        let persisted = PersistedSession {
            id: id.clone(),
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
            eprintln!("Warning: Failed to persist session metadata: {}", e);
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
        let _ = self.get(session_id)?;

        let mut active = rwlock_write_or_recover(&self.active_session);
        *active = Some(SessionId::new(session_id));
        Ok(())
    }

    pub fn list(&self) -> Vec<SessionInfo> {
        let session_refs: Vec<(SessionId, Arc<Mutex<Session>>)> = {
            let sessions = rwlock_read_or_recover(&self.sessions);
            sessions
                .iter()
                .map(|(id, session)| (id.clone(), Arc::clone(session)))
                .collect()
        };

        session_refs
            .into_iter()
            .map(|(id, session)| match session.try_lock() {
                Ok(mut sess) => SessionInfo {
                    id: id.clone(),
                    command: sess.command.clone(),
                    pid: sess.pid().unwrap_or(0),
                    running: sess.is_running(),
                    created_at: sess.created_at.to_rfc3339(),
                    size: sess.size(),
                },
                Err(_) => SessionInfo {
                    id: id.clone(),
                    command: "(busy)".to_string(),
                    pid: 0,
                    running: true,
                    created_at: "".to_string(),
                    size: (80, 24),
                },
            })
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
            eprintln!("Warning: Failed to remove session from persistence: {}", e);
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

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: SessionId,
    pub command: String,
    pub pid: u32,
    pub running: bool,
    pub created_at: String,
    pub size: (u16, u16),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSession {
    pub id: SessionId,
    pub command: String,
    pub pid: u32,
    pub created_at: String,
    pub cols: u16,
    pub rows: u16,
}

pub struct SessionPersistence {
    path: PathBuf,
}

impl SessionPersistence {
    pub fn new() -> Self {
        let path = Self::sessions_file_path();
        Self { path }
    }

    fn sessions_file_path() -> PathBuf {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp"));
        let dir = home.join(".agent-tui");
        dir.join("sessions.json")
    }

    fn ensure_dir(&self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                std::io::Error::new(
                    e.kind(),
                    format!("Failed to create directory '{}': {}", parent.display(), e),
                )
            })?;
        }
        Ok(())
    }

    pub fn load(&self) -> Vec<PersistedSession> {
        if !self.path.exists() {
            return Vec::new();
        }

        match fs::File::open(&self.path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                serde_json::from_reader(reader).unwrap_or_default()
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to open sessions file '{}': {}",
                    self.path.display(),
                    e
                );
                Vec::new()
            }
        }
    }

    pub fn save(&self, sessions: &[PersistedSession]) -> std::io::Result<()> {
        self.ensure_dir()?;

        let file = fs::File::create(&self.path).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("Failed to create file '{}': {}", self.path.display(), e),
            )
        })?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, sessions).map_err(|e| {
            std::io::Error::other(format!(
                "Failed to write sessions to '{}': {}",
                self.path.display(),
                e
            ))
        })?;
        Ok(())
    }

    pub fn add_session(&self, session: PersistedSession) -> std::io::Result<()> {
        let mut sessions = self.load();

        sessions.retain(|s| s.id != session.id);
        sessions.push(session);

        self.save(&sessions)
    }

    pub fn remove_session(&self, session_id: &str) -> std::io::Result<()> {
        let mut sessions = self.load();
        sessions.retain(|s| s.id.as_str() != session_id);
        self.save(&sessions)
    }

    pub fn cleanup_stale_sessions(&self) -> std::io::Result<usize> {
        let sessions = self.load();
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

        self.save(&active_sessions)?;
        Ok(cleaned)
    }
}

impl Default for SessionPersistence {
    fn default() -> Self {
        Self::new()
    }
}

fn is_process_running(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

impl From<&SessionInfo> for PersistedSession {
    fn from(info: &SessionInfo) -> Self {
        PersistedSession {
            id: info.id.clone(),
            command: info.command.clone(),
            pid: info.pid,
            created_at: info.created_at.clone(),
            cols: info.size.0,
            rows: info.size.1,
        }
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;

    #[test]
    fn test_persisted_session_serialization() {
        let session = PersistedSession {
            id: SessionId::new("test123"),
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
}
