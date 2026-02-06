//! Session recording lifecycle and VHS orchestration.

use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::anyhow;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;
use tracing::warn;

use crate::infra::ipc::ProcessController;
use crate::infra::ipc::ProcessStatus;
use crate::infra::ipc::Signal;
use crate::infra::ipc::UnixProcessController;

const RECORDING_STATE_VERSION: u32 = 1;
const STOP_TERM_TIMEOUT: Duration = Duration::from_secs(2);
const STOP_KILL_TIMEOUT: Duration = Duration::from_secs(2);
const STOP_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RecordingMode {
    Background,
    Foreground,
}

impl RecordingMode {
    fn from_foreground(foreground: bool) -> Self {
        if foreground {
            Self::Foreground
        } else {
            Self::Background
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Foreground => "foreground",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RecordingEntry {
    pub session_id: String,
    pub pid: u32,
    pub gif_path: PathBuf,
    pub tape_path: PathBuf,
    pub started_at: String,
    pub mode: RecordingMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RecordingState {
    pub version: u32,
    pub recordings: Vec<RecordingEntry>,
}

impl Default for RecordingState {
    fn default() -> Self {
        Self {
            version: RECORDING_STATE_VERSION,
            recordings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StartRecordingRequest {
    pub session_id: String,
    pub output_file: Option<PathBuf>,
    pub foreground: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct StartRecordingResult {
    pub entry: RecordingEntry,
}

#[derive(Debug, Clone)]
pub(crate) struct StopRecordingResult {
    pub entry: RecordingEntry,
}

#[derive(Debug, Error)]
pub(crate) enum RecordingError {
    #[error("A recording is already active for session {session_id}")]
    AlreadyRecording { session_id: String },

    #[error("No active recording for session {session_id}")]
    NotRecording { session_id: String },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Debug, Clone)]
struct RecordingPaths {
    gif_path: PathBuf,
    tape_path: PathBuf,
}

struct RecordingStore {
    state_path: PathBuf,
    lock_path: PathBuf,
}

impl RecordingStore {
    fn new() -> Self {
        let state_path = recording_state_path();
        let lock_path = state_path.with_extension("lock");
        Self {
            state_path,
            lock_path,
        }
    }

    fn load(&self) -> RecordingState {
        let contents = match fs::read_to_string(&self.state_path) {
            Ok(contents) => contents,
            Err(err) => {
                if err.kind() != std::io::ErrorKind::NotFound {
                    warn!(
                        path = %self.state_path.display(),
                        error = %err,
                        "Failed to read recording state file"
                    );
                }
                return RecordingState::default();
            }
        };

        match serde_json::from_str::<RecordingState>(&contents) {
            Ok(state) => state,
            Err(err) => {
                warn!(
                    path = %self.state_path.display(),
                    error = %err,
                    "Failed to parse recording state file; falling back to empty state"
                );
                RecordingState::default()
            }
        }
    }

    fn save(&self, state: &RecordingState) -> anyhow::Result<()> {
        if let Some(parent) = self.state_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let tmp_path = self.state_path.with_extension("json.tmp");
        let encoded =
            serde_json::to_vec_pretty(state).context("failed to serialize recording state")?;
        fs::write(&tmp_path, encoded)
            .with_context(|| format!("failed to write {}", tmp_path.display()))?;
        fs::rename(&tmp_path, &self.state_path).with_context(|| {
            format!(
                "failed to move {} to {}",
                tmp_path.display(),
                self.state_path.display()
            )
        })?;
        Ok(())
    }
}

struct RecordingFileLock {
    file: File,
}

impl RecordingFileLock {
    fn acquire(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)
            .with_context(|| format!("failed to open recording lock file {}", path.display()))?;

        let fd = file.as_raw_fd();
        // SAFETY: `flock` is safe with a valid file descriptor and we retain the file
        // handle for the lock lifetime.
        let result = unsafe { libc::flock(fd, libc::LOCK_EX) };
        if result != 0 {
            let err = std::io::Error::last_os_error();
            return Err(err)
                .with_context(|| format!("failed to acquire recording lock {}", path.display()));
        }

        Ok(Self { file })
    }
}

impl Drop for RecordingFileLock {
    fn drop(&mut self) {
        let fd = self.file.as_raw_fd();
        // SAFETY: `fd` belongs to `file`, still alive here; unlocking is idempotent.
        let _ = unsafe { libc::flock(fd, libc::LOCK_UN) };
    }
}

pub(crate) fn start_recording(
    request: StartRecordingRequest,
) -> Result<StartRecordingResult, RecordingError> {
    let controller = UnixProcessController;
    start_recording_with_controller(request, &controller)
}

pub(crate) fn stop_recording(session_id: &str) -> Result<StopRecordingResult, RecordingError> {
    let controller = UnixProcessController;
    stop_recording_with_controller(session_id, &controller)
}

pub(crate) fn start_recording_with_controller<C: ProcessController>(
    request: StartRecordingRequest,
    controller: &C,
) -> Result<StartRecordingResult, RecordingError> {
    let mode = RecordingMode::from_foreground(request.foreground);
    let started_at = Utc::now();
    let store = RecordingStore::new();
    let (entry, mut child) = {
        let _guard = RecordingFileLock::acquire(&store.lock_path).map_err(RecordingError::Other)?;

        let mut state = store.load();
        let _ = prune_stale_recordings(&mut state, controller);
        ensure_recording_slot_available(&state, &request.session_id)?;

        let paths = resolve_output_paths_with_now(
            &request.session_id,
            request.output_file.as_deref(),
            started_at,
        )
        .map_err(RecordingError::Other)?;

        write_tape_file(&paths, &request.session_id).map_err(RecordingError::Other)?;
        let child = spawn_vhs(&paths.tape_path, mode).map_err(RecordingError::Other)?;

        let entry = RecordingEntry {
            session_id: request.session_id.clone(),
            pid: child.id(),
            gif_path: paths.gif_path.clone(),
            tape_path: paths.tape_path.clone(),
            started_at: started_at.to_rfc3339(),
            mode,
        };

        state.recordings.push(entry.clone());
        if let Err(err) = store.save(&state) {
            let _ = stop_recording_process_with_controller_and_timeouts(
                &entry,
                controller,
                Duration::from_millis(500),
                Duration::from_millis(500),
            );
            return Err(RecordingError::Other(err));
        }
        (entry, child)
    };

    if mode == RecordingMode::Foreground {
        let status = child
            .wait()
            .context("failed waiting for VHS process")
            .map_err(RecordingError::Other)?;
        remove_recording_entry(&store, &request.session_id, controller)
            .map_err(RecordingError::Other)?;
        if !status.success() {
            return Err(RecordingError::Other(anyhow!(
                "VHS recording exited with status {}",
                status
            )));
        }
    }

    Ok(StartRecordingResult { entry })
}

pub(crate) fn stop_recording_with_controller<C: ProcessController>(
    session_id: &str,
    controller: &C,
) -> Result<StopRecordingResult, RecordingError> {
    let store = RecordingStore::new();
    let _guard = RecordingFileLock::acquire(&store.lock_path).map_err(RecordingError::Other)?;
    let mut state = store.load();
    if prune_stale_recordings(&mut state, controller) > 0 {
        store.save(&state).map_err(RecordingError::Other)?;
    }

    let Some(entry) = recording_for_session(&state, session_id).cloned() else {
        return Err(RecordingError::NotRecording {
            session_id: session_id.to_string(),
        });
    };

    if !is_recording_process_alive(controller, &entry) {
        if let Some(index) = state
            .recordings
            .iter()
            .position(|recording| recording.session_id == session_id)
        {
            state.recordings.remove(index);
            store.save(&state).map_err(RecordingError::Other)?;
        }
        return Err(RecordingError::NotRecording {
            session_id: session_id.to_string(),
        });
    }

    stop_recording_process_with_controller(&entry, controller)?;

    if let Some(index) = state
        .recordings
        .iter()
        .position(|recording| recording.session_id == session_id)
    {
        state.recordings.remove(index);
    }

    store.save(&state).map_err(RecordingError::Other)?;

    Ok(StopRecordingResult { entry })
}

fn remove_recording_entry<C: ProcessController>(
    store: &RecordingStore,
    session_id: &str,
    controller: &C,
) -> anyhow::Result<()> {
    let _guard = RecordingFileLock::acquire(&store.lock_path)?;
    let mut state = store.load();
    let mut changed = prune_stale_recordings(&mut state, controller) > 0;
    if let Some(index) = state
        .recordings
        .iter()
        .position(|recording| recording.session_id == session_id)
    {
        state.recordings.remove(index);
        changed = true;
    }
    if changed {
        store.save(&state)?;
    }
    Ok(())
}

pub(crate) fn ensure_recording_slot_available(
    state: &RecordingState,
    session_id: &str,
) -> Result<(), RecordingError> {
    if recording_for_session(state, session_id).is_some() {
        return Err(RecordingError::AlreadyRecording {
            session_id: session_id.to_string(),
        });
    }
    Ok(())
}

pub(crate) fn prune_stale_recordings<C: ProcessController>(
    state: &mut RecordingState,
    controller: &C,
) -> usize {
    let before = state.recordings.len();
    state
        .recordings
        .retain(|entry| is_recording_process_alive(controller, entry));
    before.saturating_sub(state.recordings.len())
}

fn recording_for_session<'a>(
    state: &'a RecordingState,
    session_id: &str,
) -> Option<&'a RecordingEntry> {
    state
        .recordings
        .iter()
        .find(|recording| recording.session_id == session_id)
}

pub(crate) fn stop_recording_process_with_controller<C: ProcessController>(
    entry: &RecordingEntry,
    controller: &C,
) -> Result<(), RecordingError> {
    stop_recording_process_with_controller_and_timeouts(
        entry,
        controller,
        STOP_TERM_TIMEOUT,
        STOP_KILL_TIMEOUT,
    )
}

fn stop_recording_process_with_controller_and_timeouts<C: ProcessController>(
    entry: &RecordingEntry,
    controller: &C,
    term_timeout: Duration,
    kill_timeout: Duration,
) -> Result<(), RecordingError> {
    let pid = entry.pid;
    if !is_recording_process_alive(controller, entry) {
        return Ok(());
    }

    controller
        .send_signal(pid, Signal::Term)
        .with_context(|| format!("failed to send SIGTERM to recording pid {pid}"))
        .map_err(RecordingError::Other)?;

    if wait_for_process_exit(controller, entry, term_timeout) {
        return Ok(());
    }

    if !is_recording_process_alive(controller, entry) {
        return Ok(());
    }

    controller
        .send_signal(pid, Signal::Kill)
        .with_context(|| format!("failed to send SIGKILL to recording pid {pid}"))
        .map_err(RecordingError::Other)?;

    if wait_for_process_exit(controller, entry, kill_timeout) {
        return Ok(());
    }

    Err(RecordingError::Other(anyhow!(
        "recording process {pid} did not exit after SIGTERM/SIGKILL"
    )))
}

fn wait_for_process_exit<C: ProcessController>(
    controller: &C,
    entry: &RecordingEntry,
    timeout: Duration,
) -> bool {
    let pid = entry.pid;
    let deadline = Instant::now() + timeout;
    loop {
        match controller.check_process(pid) {
            Ok(ProcessStatus::NotFound) => return true,
            Ok(ProcessStatus::Running) | Ok(ProcessStatus::NoPermission) => {
                if !recording_process_identity_matches(entry) {
                    return true;
                }
            }
            Err(_) => {}
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(STOP_POLL_INTERVAL);
    }
}

fn is_recording_process_alive<C: ProcessController>(
    controller: &C,
    entry: &RecordingEntry,
) -> bool {
    let pid = entry.pid;
    match controller.check_process(pid) {
        Ok(ProcessStatus::Running) => recording_process_identity_matches(entry),
        Ok(ProcessStatus::NoPermission) => false,
        Ok(ProcessStatus::NotFound) => false,
        Err(err) => {
            warn!(pid, error = %err, "Failed to query recording process status; keeping entry");
            true
        }
    }
}

fn recording_process_identity_matches(entry: &RecordingEntry) -> bool {
    let pid = entry.pid;
    let command_line = match process_command_line_for_pid(pid) {
        Ok(Some(command_line)) => command_line,
        Ok(None) => return false,
        Err(err) => {
            warn!(
                pid,
                error = %err,
                "Failed to inspect process command line for recording identity check"
            );
            return false;
        }
    };

    let tape_path = entry.tape_path.to_string_lossy();
    let matches_identity =
        command_looks_like_vhs(&command_line) && command_line.contains(tape_path.as_ref());
    if !matches_identity {
        warn!(
            pid,
            command_line = %command_line,
            expected_tape = %entry.tape_path.display(),
            "Recording PID no longer matches expected VHS process; treating entry as stale"
        );
    }
    matches_identity
}

fn process_command_line_for_pid(pid: u32) -> anyhow::Result<Option<String>> {
    let output = Command::new("ps")
        .arg("-o")
        .arg("command=")
        .arg("-p")
        .arg(pid.to_string())
        .output()
        .with_context(|| format!("failed to run ps for pid {pid}"))?;
    if !output.status.success() {
        return Ok(None);
    }
    let command_line = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if command_line.is_empty() {
        return Ok(None);
    }
    Ok(Some(command_line))
}

fn command_looks_like_vhs(command_line: &str) -> bool {
    command_line
        .split_whitespace()
        .take(3)
        .any(|token| token == "vhs" || token.ends_with("/vhs"))
}

fn spawn_vhs(tape_path: &Path, mode: RecordingMode) -> anyhow::Result<Child> {
    let mut command = Command::new("vhs");
    command.arg(tape_path);
    if mode == RecordingMode::Background {
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
    }
    command
        .spawn()
        .with_context(|| format!("failed to start vhs with tape {}", tape_path.display()))
}

fn write_tape_file(paths: &RecordingPaths, session_id: &str) -> anyhow::Result<()> {
    let exe = std::env::current_exe().context("failed to resolve current executable path")?;
    let attach_cmd = build_attach_command(&exe, session_id);
    let tape = format!(
        "Output \"{}\"\n\nSet Shell \"bash\"\nSet FontSize 20\nSet Width 1200\nSet Height 700\nSet TypingSpeed 40ms\nSet Padding 20\n\nType \"{}\" Enter\n",
        escape_tape_string(&paths.gif_path.to_string_lossy()),
        escape_tape_string(&attach_cmd),
    );

    if let Some(parent) = paths.tape_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let mut file = File::create(&paths.tape_path)
        .with_context(|| format!("failed to create {}", paths.tape_path.display()))?;
    file.write_all(tape.as_bytes())
        .with_context(|| format!("failed to write {}", paths.tape_path.display()))?;
    Ok(())
}

fn build_attach_command(exe: &Path, session_id: &str) -> String {
    let mut command = String::new();
    for env_var in [
        "AGENT_TUI_SOCKET",
        "AGENT_TUI_TRANSPORT",
        "AGENT_TUI_WS_ADDR",
    ] {
        if let Ok(value) = std::env::var(env_var)
            && !value.trim().is_empty()
        {
            if !command.is_empty() {
                command.push(' ');
            }
            command.push_str(env_var);
            command.push('=');
            command.push_str(&shell_quote(&value));
        }
    }

    if !command.is_empty() {
        command.push(' ');
    }
    command.push_str(&shell_quote(&exe.to_string_lossy()));
    command.push_str(" --session ");
    command.push_str(&shell_quote(session_id));
    command.push_str(" sessions attach --no-tty");
    command
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    let escaped = value.replace('\'', "'\"'\"'");
    format!("'{escaped}'")
}

fn escape_tape_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn resolve_output_paths_with_now(
    session_id: &str,
    output_file: Option<&Path>,
    started_at: DateTime<Utc>,
) -> anyhow::Result<RecordingPaths> {
    if let Some(path) = output_file {
        return resolve_explicit_output_paths(session_id, path, started_at);
    }

    let output_dir = default_recordings_dir()?;
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;
    let filename_base = format!("{}-{}", session_id, started_at.format("%Y%m%d-%H%M%S"));
    Ok(RecordingPaths {
        gif_path: output_dir.join(format!("{filename_base}.gif")),
        tape_path: output_dir.join(format!("{filename_base}.tape")),
    })
}

fn resolve_explicit_output_paths(
    session_id: &str,
    output_file: &Path,
    started_at: DateTime<Utc>,
) -> anyhow::Result<RecordingPaths> {
    let absolute_path = absolutize(output_file)?;
    let directory_mode = if absolute_path.exists() {
        absolute_path.is_dir()
    } else {
        absolute_path.extension().is_none()
    };

    if directory_mode {
        fs::create_dir_all(&absolute_path)
            .with_context(|| format!("failed to create {}", absolute_path.display()))?;
        let filename_base = format!("{}-{}", session_id, started_at.format("%Y%m%d-%H%M%S"));
        return Ok(RecordingPaths {
            gif_path: absolute_path.join(format!("{filename_base}.gif")),
            tape_path: absolute_path.join(format!("{filename_base}.tape")),
        });
    }

    let parent = absolute_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&parent)
        .with_context(|| format!("failed to create {}", parent.display()))?;

    let ext = absolute_path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    let (gif_path, tape_path) = match ext.as_deref() {
        Some("gif") => (absolute_path.clone(), absolute_path.with_extension("tape")),
        Some("tape") => (absolute_path.with_extension("gif"), absolute_path.clone()),
        _ => (
            absolute_path.with_extension("gif"),
            absolute_path.with_extension("tape"),
        ),
    };

    Ok(RecordingPaths {
        gif_path,
        tape_path,
    })
}

fn default_recordings_dir() -> anyhow::Result<PathBuf> {
    if let Ok(path) = std::env::var("AGENT_TUI_RECORDINGS_DIR") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return absolutize(Path::new(trimmed));
        }
    }
    std::env::current_dir().context("failed to resolve current working directory")
}

fn recording_state_path() -> PathBuf {
    if let Ok(path) = std::env::var("AGENT_TUI_RECORD_STATE") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    home.join(".agent-tui").join("recordings.json")
}

fn absolutize(path: &Path) -> anyhow::Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Ok(std::env::current_dir()
        .context("failed to resolve current working directory")?
        .join(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::ipc::ProcessStatus;
    use std::collections::HashMap;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;
    use std::process::Stdio;
    use std::sync::Mutex;
    use std::sync::MutexGuard;
    use std::sync::OnceLock;
    use tempfile::TempDir;

    struct EnvGuard {
        key: &'static str,
        value: Option<String>,
        _lock: MutexGuard<'static, ()>,
    }

    fn env_lock() -> &'static Mutex<()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    impl EnvGuard {
        fn set(key: &'static str, value: impl Into<String>) -> Self {
            let lock = env_lock().lock().unwrap_or_else(|e| e.into_inner());
            let previous = std::env::var(key).ok();
            // SAFETY: test-only environment mutation for isolated scenarios.
            unsafe {
                std::env::set_var(key, value.into());
            }
            Self {
                key,
                value: previous,
                _lock: lock,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: test-only environment restoration.
            unsafe {
                if let Some(value) = self.value.take() {
                    std::env::set_var(self.key, value);
                } else {
                    std::env::remove_var(self.key);
                }
            }
        }
    }

    #[derive(Default)]
    struct TestProcessController {
        statuses: Mutex<HashMap<u32, ProcessStatus>>,
        signals: Mutex<Vec<(u32, Signal)>>,
        kill_on_term: Mutex<bool>,
        kill_on_kill: Mutex<bool>,
    }

    impl TestProcessController {
        fn with_process(self, pid: u32, status: ProcessStatus) -> Self {
            self.statuses
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(pid, status);
            self
        }

        fn kill_on_signal(self, signal: Signal, enabled: bool) -> Self {
            match signal {
                Signal::Term => {
                    *self.kill_on_term.lock().unwrap_or_else(|e| e.into_inner()) = enabled;
                }
                Signal::Kill => {
                    *self.kill_on_kill.lock().unwrap_or_else(|e| e.into_inner()) = enabled;
                }
            }
            self
        }

        fn signals(&self) -> Vec<(u32, Signal)> {
            self.signals
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        }
    }

    impl ProcessController for TestProcessController {
        fn check_process(&self, pid: u32) -> std::io::Result<ProcessStatus> {
            Ok(*self
                .statuses
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&pid)
                .unwrap_or(&ProcessStatus::NotFound))
        }

        fn send_signal(&self, pid: u32, signal: Signal) -> std::io::Result<()> {
            self.signals
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push((pid, signal));
            let should_kill = match signal {
                Signal::Term => *self.kill_on_term.lock().unwrap_or_else(|e| e.into_inner()),
                Signal::Kill => *self.kill_on_kill.lock().unwrap_or_else(|e| e.into_inner()),
            };
            if should_kill {
                self.statuses
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(pid, ProcessStatus::NotFound);
            }
            Ok(())
        }
    }

    fn fixed_time() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-01-01T12:34:56Z")
            .expect("valid timestamp")
            .with_timezone(&Utc)
    }

    fn sample_entry(session_id: &str, pid: u32, tape_path: PathBuf) -> RecordingEntry {
        RecordingEntry {
            session_id: session_id.to_string(),
            pid,
            gif_path: tape_path.with_extension("gif"),
            tape_path,
            started_at: "2026-01-01T12:34:56Z".to_string(),
            mode: RecordingMode::Background,
        }
    }

    fn spawn_fake_vhs_process(temp: &TempDir, tape_path: &Path) -> std::process::Child {
        let fake_vhs = temp.path().join("vhs");
        fs::write(
            &fake_vhs,
            r#"#!/bin/sh
sleep 30
"#,
        )
        .expect("write fake vhs");
        let mut perms = fs::metadata(&fake_vhs)
            .expect("fake vhs metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_vhs, perms).expect("set fake vhs perms");
        Command::new(fake_vhs)
            .arg(tape_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn fake vhs")
    }

    #[test]
    fn resolve_output_paths_matrix() {
        let temp = TempDir::new_in("/tmp").expect("tempdir");
        let existing_dir = temp.path().join("dir-existing");
        let existing_file = temp.path().join("file-existing.gif");
        fs::create_dir_all(&existing_dir).expect("create dir");
        fs::write(&existing_file, b"x").expect("create file");

        let from_existing_dir =
            resolve_output_paths_with_now("sess1", Some(&existing_dir), fixed_time()).expect("dir");
        assert!(from_existing_dir.gif_path.starts_with(&existing_dir));
        assert!(from_existing_dir.tape_path.starts_with(&existing_dir));
        assert!(
            from_existing_dir
                .gif_path
                .ends_with("sess1-20260101-123456.gif")
        );
        assert!(
            from_existing_dir
                .tape_path
                .ends_with("sess1-20260101-123456.tape")
        );

        let from_existing_file =
            resolve_output_paths_with_now("sess1", Some(&existing_file), fixed_time())
                .expect("file");
        assert_eq!(from_existing_file.gif_path, existing_file);
        assert_eq!(
            from_existing_file.tape_path,
            temp.path().join("file-existing.tape")
        );

        let non_existing_with_ext = temp.path().join("out.gif");
        let with_ext =
            resolve_output_paths_with_now("sess1", Some(&non_existing_with_ext), fixed_time())
                .expect("ext");
        assert_eq!(with_ext.gif_path, non_existing_with_ext);
        assert_eq!(with_ext.tape_path, temp.path().join("out.tape"));
    }

    #[test]
    fn non_existing_no_extension_is_directory_mode() {
        let temp = TempDir::new_in("/tmp").expect("tempdir");
        let output = temp.path().join("recordings");
        let resolved = resolve_output_paths_with_now("sess1", Some(&output), fixed_time())
            .expect("resolve output");
        assert!(resolved.gif_path.starts_with(&output));
        assert!(resolved.tape_path.starts_with(&output));
        assert!(resolved.gif_path.ends_with("sess1-20260101-123456.gif"));
        assert!(resolved.tape_path.ends_with("sess1-20260101-123456.tape"));
        assert!(output.exists());
        assert!(output.is_dir());
    }

    #[test]
    fn prune_stale_recordings_removes_dead_processes() {
        let temp = TempDir::new_in("/tmp").expect("tempdir");
        let running_tape = temp.path().join("running.tape");
        let dead_tape = temp.path().join("dead.tape");
        let mut running_child = spawn_fake_vhs_process(&temp, &running_tape);
        let mut dead_child = spawn_fake_vhs_process(&temp, &dead_tape);
        let dead_pid = dead_child.id();
        let _ = dead_child.kill();
        let _ = dead_child.wait();

        let mut state = RecordingState {
            version: RECORDING_STATE_VERSION,
            recordings: vec![
                sample_entry("sess1", running_child.id(), running_tape.clone()),
                sample_entry("sess2", dead_pid, dead_tape),
            ],
        };
        let controller = UnixProcessController;

        let pruned = prune_stale_recordings(&mut state, &controller);
        assert_eq!(pruned, 1);
        assert_eq!(state.recordings.len(), 1);
        assert_eq!(state.recordings[0].session_id, "sess1");
        assert_eq!(state.recordings[0].pid, running_child.id());

        let _ = running_child.kill();
        let _ = running_child.wait();
    }

    #[test]
    fn prune_stale_recordings_removes_pid_reuse_candidates() {
        let mut non_vhs_process = Command::new("sh")
            .arg("-c")
            .arg("sleep 30")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn non-vhs process");
        let mut state = RecordingState {
            version: RECORDING_STATE_VERSION,
            recordings: vec![sample_entry(
                "sess1",
                non_vhs_process.id(),
                PathBuf::from("/tmp/expected-recording.tape"),
            )],
        };
        let controller = UnixProcessController;

        let pruned = prune_stale_recordings(&mut state, &controller);
        assert_eq!(pruned, 1);
        assert!(state.recordings.is_empty());

        let _ = non_vhs_process.kill();
        let _ = non_vhs_process.wait();
    }

    #[test]
    fn one_per_session_conflict_behavior() {
        let state = RecordingState {
            version: RECORDING_STATE_VERSION,
            recordings: vec![sample_entry("sess1", 10, PathBuf::from("/tmp/demo.tape"))],
        };
        let result = ensure_recording_slot_available(&state, "sess1");
        assert!(matches!(
            result,
            Err(RecordingError::AlreadyRecording { session_id }) if session_id == "sess1"
        ));
    }

    #[test]
    fn stop_errors_when_recording_absent() {
        let temp = TempDir::new_in("/tmp").expect("tempdir");
        let state_path = temp.path().join("recordings.json");
        let _state_guard =
            EnvGuard::set("AGENT_TUI_RECORD_STATE", state_path.display().to_string());

        let controller = TestProcessController::default();
        let result = stop_recording_with_controller("missing", &controller);
        assert!(matches!(
            result,
            Err(RecordingError::NotRecording { session_id }) if session_id == "missing"
        ));
    }

    #[test]
    fn stop_recording_treats_mismatched_process_as_stale() {
        let temp = TempDir::new_in("/tmp").expect("tempdir");
        let state_path = temp.path().join("recordings.json");
        let _state_guard =
            EnvGuard::set("AGENT_TUI_RECORD_STATE", state_path.display().to_string());
        let mut non_vhs_process = Command::new("sh")
            .arg("-c")
            .arg("sleep 30")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn non-vhs process");
        let store = RecordingStore::new();
        let state = RecordingState {
            version: RECORDING_STATE_VERSION,
            recordings: vec![sample_entry(
                "sess1",
                non_vhs_process.id(),
                temp.path().join("expected.tape"),
            )],
        };
        store.save(&state).expect("save state");

        let controller = TestProcessController::default()
            .with_process(non_vhs_process.id(), ProcessStatus::Running);
        let result = stop_recording_with_controller("sess1", &controller);
        assert!(matches!(
            result,
            Err(RecordingError::NotRecording { session_id }) if session_id == "sess1"
        ));
        assert!(controller.signals().is_empty());
        assert!(store.load().recordings.is_empty());

        let _ = non_vhs_process.kill();
        let _ = non_vhs_process.wait();
    }

    #[test]
    fn start_recording_fails_when_lock_path_is_unavailable() {
        let temp = TempDir::new_in("/tmp").expect("tempdir");
        let not_a_dir = temp.path().join("state-parent");
        fs::write(&not_a_dir, b"x").expect("create blocking file");
        let state_path = not_a_dir.join("recordings.json");
        let _state_guard =
            EnvGuard::set("AGENT_TUI_RECORD_STATE", state_path.display().to_string());

        let request = StartRecordingRequest {
            session_id: "sess1".to_string(),
            output_file: Some(temp.path().join("out.gif")),
            foreground: false,
        };
        let controller = TestProcessController::default();
        let result = start_recording_with_controller(request, &controller);
        assert!(matches!(result, Err(RecordingError::Other(_))));
    }

    #[test]
    fn stop_signal_escalation_behavior() {
        let temp = TempDir::new_in("/tmp").expect("tempdir");
        let tape_path = temp.path().join("escalation.tape");
        let mut fake_vhs = spawn_fake_vhs_process(&temp, &tape_path);
        let pid = fake_vhs.id();
        let entry = sample_entry("sess1", pid, tape_path);
        let controller = TestProcessController::default()
            .with_process(pid, ProcessStatus::Running)
            .kill_on_signal(Signal::Term, false)
            .kill_on_signal(Signal::Kill, true);

        stop_recording_process_with_controller_and_timeouts(
            &entry,
            &controller,
            Duration::from_millis(20),
            Duration::from_millis(20),
        )
        .expect("stop should escalate to kill");

        assert_eq!(
            controller.signals(),
            vec![(pid, Signal::Term), (pid, Signal::Kill)]
        );

        let _ = fake_vhs.kill();
        let _ = fake_vhs.wait();
    }
}
