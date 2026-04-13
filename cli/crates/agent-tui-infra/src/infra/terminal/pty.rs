//! PTY management.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::error::Error as StdError;
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::fd::FromRawFd;
use std::os::fd::RawFd;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use crossbeam_channel as channel;
use crossterm::event::KeyCode;
use libc::POLLERR;
use libc::POLLHUP;
use libc::POLLIN;
use libc::POLLNVAL;
use libc::POLLOUT;
use libc::poll;
use libc::pollfd;
use portable_pty::Child;
use portable_pty::CommandBuilder;
use portable_pty::MasterPty;
use portable_pty::PtySize;
use portable_pty::native_pty_system;
use tracing::debug;
use tracing::warn;

use crate::common::mutex_lock_or_recover;
use crate::domain::session_types::TerminalSize;
use crate::usecases::ports::SpawnErrorKind;

pub use crate::infra::terminal::error::PtyError;

pub struct PtyHandle {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    size: PtySize,
    read_rx: Option<channel::Receiver<ReadEvent>>,
    reader_worker: Option<ReaderWorker>,
    read_buffer: VecDeque<u8>,
    read_closed: bool,
    read_error: Option<String>,
}

const TERMINATE_TIMEOUT: Duration = Duration::from_millis(500);
const KILL_TIMEOUT: Duration = Duration::from_millis(500);
const KILL_POLL_INTERVAL: Duration = Duration::from_millis(25);
const READER_JOIN_TIMEOUT: Duration = Duration::from_millis(500);
const READER_JOIN_POLL_INTERVAL: Duration = Duration::from_millis(20);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReaderJoinOutcome {
    Joined,
    ReapingInBackground,
}

struct ReaderWorker {
    shutdown_writer: UnixStream,
    join: Option<thread::JoinHandle<()>>,
}

impl ReaderWorker {
    fn shutdown(&mut self) -> ReaderJoinOutcome {
        if let Err(err) = self.shutdown_writer.write_all(&[1]) {
            if err.kind() != io::ErrorKind::BrokenPipe {
                warn!(error = %err, "Failed to signal PTY reader shutdown");
            }
        }

        let Some(join) = self.join.take() else {
            return ReaderJoinOutcome::Joined;
        };

        join_thread_with_timeout_or_reap(join, READER_JOIN_TIMEOUT, "pty reader thread")
    }
}

impl Drop for PtyHandle {
    fn drop(&mut self) {
        let _ = self.kill_process();
        self.shutdown_reader();
    }
}

impl PtyHandle {
    pub fn spawn(
        command: &str,
        args: &[String],
        cwd: Option<&str>,
        env: Option<&HashMap<String, String>>,
        size: TerminalSize,
    ) -> Result<Self, PtyError> {
        let pty_system = native_pty_system();

        let size = PtySize {
            rows: size.rows(),
            cols: size.cols(),
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system.openpty(size).map_err(|e| PtyError::Open {
            reason: e.to_string(),
            source: None,
        })?;

        let mut cmd = CommandBuilder::new(command);
        cmd.args(args);

        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }

        if let Some(env_vars) = env {
            for (key, value) in env_vars {
                cmd.env(key, value);
            }
        }

        cmd.env("TERM", "xterm-256color");

        let mut child = pair.slave.spawn_command(cmd).map_err(|e| {
            let reason = e.to_string();
            let kind = classify_spawn_error(e.as_ref(), &reason);
            PtyError::Spawn { reason, kind }
        })?;

        let writer = pair.master.take_writer().map_err(|e| PtyError::Open {
            reason: e.to_string(),
            source: None,
        })?;

        let (read_rx, reader_worker) = match spawn_reader(pair.master.as_raw_fd()) {
            Ok(parts) => parts,
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(err);
            }
        };

        Ok(Self {
            master: pair.master,
            child,
            writer: Arc::new(Mutex::new(writer)),
            size,
            read_rx: Some(read_rx),
            reader_worker: Some(reader_worker),
            read_buffer: VecDeque::new(),
            read_closed: false,
            read_error: None,
        })
    }

    pub fn pid(&self) -> Option<u32> {
        self.child.process_id()
    }

    pub fn is_running(&mut self) -> bool {
        self.child
            .try_wait()
            .map(|status| status.is_none())
            .unwrap_or(false)
    }

    pub fn write(&self, data: &[u8]) -> Result<(), PtyError> {
        if data.is_empty() {
            return Ok(());
        }

        let mut writer = mutex_lock_or_recover(&self.writer);
        let mut offset = 0;
        while offset < data.len() {
            match writer.write(&data[offset..]) {
                Ok(0) => {
                    return Err(PtyError::Write {
                        reason: "write returned 0 bytes, PTY closed".to_string(),
                        source: None,
                    });
                }
                Ok(n) => offset += n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    self.wait_writable()?;
                }
                Err(e) => {
                    let reason = e.to_string();
                    return Err(PtyError::Write {
                        reason,
                        source: Some(e),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn write_str(&self, s: &str) -> Result<(), PtyError> {
        self.write(s.as_bytes())
    }

    fn wait_writable(&self) -> Result<(), PtyError> {
        #[cfg(unix)]
        {
            let Some(fd) = self.master.as_raw_fd() else {
                return Ok(());
            };
            let mut fds = [pollfd {
                fd,
                events: POLLOUT,
                revents: 0,
            }];
            loop {
                // SAFETY: `poll` is called with a valid pointer to `fds` and length 1.
                // The array lives for the duration of the call.
                let rc = unsafe { poll(fds.as_mut_ptr(), 1, -1) };
                if rc < 0 {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::Interrupted {
                        continue;
                    }
                    let reason = err.to_string();
                    return Err(PtyError::Write {
                        reason,
                        source: Some(err),
                    });
                }
                let events = fds[0].revents;
                if events & (POLLHUP | POLLERR) != 0 {
                    return Err(PtyError::Write {
                        reason: "PTY closed".to_string(),
                        source: None,
                    });
                }
                if events & POLLOUT != 0 {
                    return Ok(());
                }
            }
        }
        #[cfg(not(unix))]
        {
            Ok(())
        }
    }

    pub fn try_read(&mut self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, PtyError> {
        if buf.is_empty() {
            return Ok(0);
        }

        if self.read_closed && self.read_buffer.is_empty() {
            if let Some(error) = self.read_error.take() {
                return Err(PtyError::Read {
                    reason: error,
                    source: None,
                });
            }
            return Ok(0);
        }

        if self.read_buffer.is_empty() && !self.read_closed {
            let read_rx = match self.read_rx.as_ref() {
                Some(rx) => rx.clone(),
                None => {
                    return Err(PtyError::Read {
                        reason: "PTY read channel is not available".to_string(),
                        source: None,
                    });
                }
            };

            let first_event = if timeout_ms < 0 {
                match read_rx.recv() {
                    Ok(event) => Some(event),
                    Err(_) => {
                        self.read_closed = true;
                        None
                    }
                }
            } else {
                let timeout = Duration::from_millis(timeout_ms as u64);
                match read_rx.recv_timeout(timeout) {
                    Ok(event) => Some(event),
                    Err(channel::RecvTimeoutError::Timeout) => None,
                    Err(channel::RecvTimeoutError::Disconnected) => {
                        self.read_closed = true;
                        None
                    }
                }
            };

            if let Some(event) = first_event {
                self.handle_read_event(event);
            }

            while let Ok(event) = read_rx.try_recv() {
                self.handle_read_event(event);
            }
        }

        let mut total = 0;
        while total < buf.len() {
            match self.read_buffer.pop_front() {
                Some(byte) => {
                    buf[total] = byte;
                    total += 1;
                }
                None => break,
            }
        }

        let read_error = if total == 0 && self.read_closed {
            self.read_error.take()
        } else {
            None
        };
        if let Some(error) = read_error {
            return Err(PtyError::Read {
                reason: error,
                source: None,
            });
        }

        Ok(total)
    }

    pub fn resize(&mut self, size: TerminalSize) -> Result<(), PtyError> {
        self.size = PtySize {
            rows: size.rows(),
            cols: size.cols(),
            pixel_width: 0,
            pixel_height: 0,
        };
        self.master.resize(self.size).map_err(|e| PtyError::Resize {
            reason: e.to_string(),
            source: None,
        })
    }

    pub fn kill(&mut self) -> Result<(), PtyError> {
        let result = self.kill_process();
        if result.is_ok() {
            self.shutdown_reader();
        }
        result
    }

    pub(crate) fn take_read_rx(&mut self) -> Option<channel::Receiver<ReadEvent>> {
        self.read_rx.take()
    }
}

impl PtyHandle {
    fn kill_process(&mut self) -> Result<(), PtyError> {
        if !self.is_running() {
            return Ok(());
        }

        #[cfg(unix)]
        {
            if let Some(pid) = self.child.process_id() {
                match can_signal_process_group(pid) {
                    Ok(true) => {
                        if let Err(err) = signal_process_group(pid, libc::SIGTERM) {
                            if let Err(kill_err) = self.child.kill() {
                                return Err(PtyError::Spawn {
                                    reason: format!(
                                        "failed to signal process group ({err}) and kill child: {kill_err}"
                                    ),
                                    kind: SpawnErrorKind::Other,
                                });
                            }
                            let _ = self.wait_for_exit(KILL_TIMEOUT);
                            return Ok(());
                        } else if self.wait_for_exit(TERMINATE_TIMEOUT) {
                            return Ok(());
                        }

                        let _ = signal_process_group(pid, libc::SIGKILL);
                        let _ = self.wait_for_exit(KILL_TIMEOUT);
                        return Ok(());
                    }
                    Ok(false) => {
                        warn!(
                            pid,
                            "PTY child is not process-group leader; falling back to direct kill"
                        );
                    }
                    Err(err) => {
                        warn!(
                            pid,
                            error = %err,
                            "Failed to verify PTY process-group leadership; falling back to direct kill"
                        );
                    }
                }
                self.child.kill().map_err(|e| PtyError::Spawn {
                    reason: e.to_string(),
                    kind: SpawnErrorKind::Other,
                })?;
                let _ = self.wait_for_exit(KILL_TIMEOUT);
                return Ok(());
            }
        }

        self.child.kill().map_err(|e| PtyError::Spawn {
            reason: e.to_string(),
            kind: SpawnErrorKind::Other,
        })?;
        let _ = self.wait_for_exit(KILL_TIMEOUT);
        Ok(())
    }
    fn wait_for_exit(&mut self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => return true,
                Ok(None) => {}
                Err(_) => return false,
            }

            if Instant::now() >= deadline {
                return false;
            }
            std::thread::park_timeout(KILL_POLL_INTERVAL);
        }
    }

    fn shutdown_reader(&mut self) {
        let _ = self.read_rx.take();
        if let Some(mut worker) = self.reader_worker.take() {
            let _ = worker.shutdown();
        }
    }

    fn handle_read_event(&mut self, event: ReadEvent) {
        match event {
            ReadEvent::Data(data) => self.read_buffer.extend(data),
            ReadEvent::Eof => self.read_closed = true,
            ReadEvent::Error(error) => {
                self.read_closed = true;
                self.read_error = Some(error);
            }
        }
    }
}

#[cfg(unix)]
fn signal_process_group(pid: u32, signal: libc::c_int) -> io::Result<()> {
    let pid_t: libc::pid_t = pid
        .try_into()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid pid"))?;
    // SAFETY: negative pid sends the signal to the process group.
    let rc = unsafe { libc::kill(-pid_t, signal) };
    if rc == 0 {
        return Ok(());
    }
    let err = io::Error::last_os_error();
    match err.raw_os_error() {
        Some(code) if code == libc::ESRCH => Ok(()),
        _ => Err(err),
    }
}

#[cfg(unix)]
fn can_signal_process_group(pid: u32) -> io::Result<bool> {
    let pid_t: libc::pid_t = pid
        .try_into()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid pid"))?;
    // SAFETY: `getpgid` is safe with a valid pid_t.
    let pgid = unsafe { libc::getpgid(pid_t) };
    if pgid == -1 {
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ESRCH) {
            return Ok(false);
        }
        return Err(err);
    }
    Ok(pgid == pid_t)
}

pub(crate) enum ReadEvent {
    Data(Vec<u8>),
    Eof,
    Error(String),
}

fn join_thread_with_timeout_or_reap(
    handle: thread::JoinHandle<()>,
    timeout: Duration,
    thread_label: &'static str,
) -> ReaderJoinOutcome {
    let deadline = Instant::now() + timeout;
    while !handle.is_finished() {
        if Instant::now() >= deadline {
            warn!(
                thread = thread_label,
                timeout_ms = timeout.as_millis(),
                "Timed out joining thread; handing ownership to background reaper"
            );
            return spawn_join_reaper(handle, thread_label);
        }
        thread::park_timeout(READER_JOIN_POLL_INTERVAL);
    }
    let _ = handle.join();
    ReaderJoinOutcome::Joined
}

fn spawn_join_reaper(
    handle: thread::JoinHandle<()>,
    thread_label: &'static str,
) -> ReaderJoinOutcome {
    let handle_cell = Arc::new(Mutex::new(Some(handle)));
    let handle_for_thread = Arc::clone(&handle_cell);
    match thread::Builder::new()
        .name("pty-reader-reaper".to_string())
        .spawn(move || {
            let Some(handle) = handle_for_thread
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take()
            else {
                return;
            };

            if handle.join().is_err() {
                warn!(
                    thread = thread_label,
                    "Background reaper observed thread panic"
                );
            }
        }) {
        Ok(_) => ReaderJoinOutcome::ReapingInBackground,
        Err(err) => {
            warn!(
                thread = thread_label,
                error = %err,
                "Failed to spawn background join reaper; joining synchronously"
            );
            if let Some(handle) = handle_cell
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take()
            {
                let _ = handle.join();
            }
            ReaderJoinOutcome::Joined
        }
    }
}

fn spawn_reader(
    master_fd: Option<RawFd>,
) -> Result<(channel::Receiver<ReadEvent>, ReaderWorker), PtyError> {
    let Some(master_fd) = master_fd else {
        return Err(PtyError::Open {
            reason: "PTY master file descriptor is unavailable".to_string(),
            source: None,
        });
    };
    spawn_reader_from_fd(master_fd)
}

fn spawn_reader_from_fd(
    master_fd: RawFd,
) -> Result<(channel::Receiver<ReadEvent>, ReaderWorker), PtyError> {
    let reader_fd = duplicate_fd(master_fd).map_err(|err| PtyError::Open {
        reason: format!("failed to duplicate PTY reader fd: {err}"),
        source: None,
    })?;
    let reader = {
        // SAFETY: `duplicate_fd` returns a new owned file descriptor that belongs
        // exclusively to this `File`.
        unsafe { File::from_raw_fd(reader_fd) }
    };
    let (mut shutdown_reader, shutdown_writer) =
        UnixStream::pair().map_err(|err| PtyError::Open {
            reason: format!("failed to create PTY reader shutdown signal: {err}"),
            source: None,
        })?;
    shutdown_reader
        .set_nonblocking(true)
        .map_err(|err| PtyError::Open {
            reason: format!("failed to configure PTY reader shutdown signal: {err}"),
            source: None,
        })?;

    #[allow(clippy::disallowed_methods)]
    let (tx, rx) = channel::unbounded();
    let span = tracing::debug_span!("pty_reader");
    let join = thread::Builder::new()
        .name("pty-reader".to_string())
        .spawn(move || {
            let _guard = span.enter();
            reader_loop(reader, &mut shutdown_reader, tx);
        })
        .map_err(|err| PtyError::Open {
            reason: format!("failed to spawn PTY reader thread: {err}"),
            source: None,
        })?;

    Ok((
        rx,
        ReaderWorker {
            shutdown_writer,
            join: Some(join),
        },
    ))
}

fn reader_loop(mut reader: File, shutdown_reader: &mut UnixStream, tx: channel::Sender<ReadEvent>) {
    let mut buf = [0u8; 8192];
    loop {
        match wait_for_reader_or_shutdown(reader.as_raw_fd(), shutdown_reader.as_raw_fd()) {
            Ok(ReaderPoll::Shutdown) => {
                drain_shutdown_signal(shutdown_reader);
                debug!("PTY reader shutdown");
                break;
            }
            Ok(ReaderPoll::Readable) => {
                if !forward_read_event(&mut reader, &tx, &mut buf) {
                    break;
                }
            }
            Err(err) => {
                warn!(error = %err, "PTY reader poll error");
                let _ = tx.send(ReadEvent::Error(err.to_string()));
                break;
            }
        }
    }
}

fn forward_read_event<R: Read>(
    reader: &mut R,
    tx: &channel::Sender<ReadEvent>,
    buf: &mut [u8; 8192],
) -> bool {
    match reader.read(buf) {
        Ok(0) => {
            let _ = tx.send(ReadEvent::Eof);
            debug!("PTY reader EOF");
            false
        }
        Ok(n) => tx.send(ReadEvent::Data(buf[..n].to_vec())).is_ok(),
        Err(err) if err.kind() == io::ErrorKind::Interrupted => true,
        Err(err) => {
            warn!(error = %err, "PTY reader error");
            let _ = tx.send(ReadEvent::Error(err.to_string()));
            false
        }
    }
}

enum ReaderPoll {
    Readable,
    Shutdown,
}

fn wait_for_reader_or_shutdown(reader_fd: RawFd, shutdown_fd: RawFd) -> io::Result<ReaderPoll> {
    let mut fds = [
        pollfd {
            fd: shutdown_fd,
            events: POLLIN,
            revents: 0,
        },
        pollfd {
            fd: reader_fd,
            events: POLLIN | POLLHUP | POLLERR | POLLNVAL,
            revents: 0,
        },
    ];
    loop {
        // SAFETY: `poll` is called with a valid pointer to two live `pollfd`
        // entries whose storage outlives the call.
        let rc = unsafe { poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, -1) };
        if rc < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(err);
        }
        if fds[0].revents & (POLLIN | POLLHUP | POLLERR | POLLNVAL) != 0 {
            return Ok(ReaderPoll::Shutdown);
        }
        if fds[1].revents & (POLLIN | POLLHUP | POLLERR | POLLNVAL) != 0 {
            return Ok(ReaderPoll::Readable);
        }
    }
}

fn drain_shutdown_signal(reader: &mut UnixStream) {
    let mut buf = [0u8; 64];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(_) => continue,
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
            Err(_) => break,
        }
    }
}

fn duplicate_fd(fd: RawFd) -> io::Result<RawFd> {
    // SAFETY: `fd` is borrowed for the duration of this syscall and the returned
    // descriptor is independent from the original on success.
    let duplicated = unsafe { libc::dup(fd) };
    if duplicated < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(duplicated)
}

pub fn key_to_escape_sequence(key: &str) -> Option<Vec<u8>> {
    if key.contains('+') {
        let parts: Vec<&str> = key.split('+').collect();
        if parts.len() == 2 {
            let modifier = parts[0];
            let base_key = parts[1];

            return match modifier.to_lowercase().as_str() {
                "ctrl" | "control" => {
                    if base_key.len() == 1 {
                        let c = base_key.chars().next()?.to_ascii_uppercase();
                        if c.is_ascii_alphabetic() {
                            return Some(vec![(c as u8) - b'A' + 1]);
                        }
                    }

                    match base_key.to_lowercase().as_str() {
                        "c" => Some(vec![3]),
                        "d" => Some(vec![4]),
                        "z" => Some(vec![26]),
                        "\\" => Some(vec![28]),
                        "[" => Some(vec![27]),
                        _ => None,
                    }
                }
                "alt" | "meta" => {
                    let base = key_to_escape_sequence(base_key)?;
                    let mut result = vec![0x1b];
                    result.extend(base);
                    Some(result)
                }
                "shift" => match base_key.to_lowercase().as_str() {
                    "tab" => Some(vec![0x1b, b'[', b'Z']),
                    _ => {
                        if base_key.len() == 1 {
                            Some(base_key.to_uppercase().as_bytes().to_vec())
                        } else {
                            None
                        }
                    }
                },
                _ => None,
            };
        }
    }

    match key {
        "Enter" | "Return" => Some(vec![b'\r']),
        "Tab" => Some(vec![b'\t']),
        "Escape" | "Esc" => Some(vec![0x1b]),
        "Backspace" => Some(vec![0x7f]),
        "Delete" => Some(vec![0x1b, b'[', b'3', b'~']),
        "Space" => Some(vec![b' ']),

        "ArrowUp" | "Up" => Some(vec![0x1b, b'[', b'A']),
        "ArrowDown" | "Down" => Some(vec![0x1b, b'[', b'B']),
        "ArrowRight" | "Right" => Some(vec![0x1b, b'[', b'C']),
        "ArrowLeft" | "Left" => Some(vec![0x1b, b'[', b'D']),

        "Home" => Some(vec![0x1b, b'[', b'H']),
        "End" => Some(vec![0x1b, b'[', b'F']),
        "PageUp" => Some(vec![0x1b, b'[', b'5', b'~']),
        "PageDown" => Some(vec![0x1b, b'[', b'6', b'~']),
        "Insert" => Some(vec![0x1b, b'[', b'2', b'~']),

        "F1" => Some(vec![0x1b, b'O', b'P']),
        "F2" => Some(vec![0x1b, b'O', b'Q']),
        "F3" => Some(vec![0x1b, b'O', b'R']),
        "F4" => Some(vec![0x1b, b'O', b'S']),
        "F5" => Some(vec![0x1b, b'[', b'1', b'5', b'~']),
        "F6" => Some(vec![0x1b, b'[', b'1', b'7', b'~']),
        "F7" => Some(vec![0x1b, b'[', b'1', b'8', b'~']),
        "F8" => Some(vec![0x1b, b'[', b'1', b'9', b'~']),
        "F9" => Some(vec![0x1b, b'[', b'2', b'0', b'~']),
        "F10" => Some(vec![0x1b, b'[', b'2', b'1', b'~']),
        "F11" => Some(vec![0x1b, b'[', b'2', b'3', b'~']),
        "F12" => Some(vec![0x1b, b'[', b'2', b'4', b'~']),

        _ if key.len() == 1 => Some(key.as_bytes().to_vec()),

        _ => None,
    }
}

pub(crate) fn keycode_to_escape_sequence(code: KeyCode) -> Option<Vec<u8>> {
    let key = match code {
        KeyCode::Up => "Up",
        KeyCode::Down => "Down",
        KeyCode::Left => "Left",
        KeyCode::Right => "Right",
        _ => return None,
    };
    key_to_escape_sequence(key)
}

fn classify_spawn_error(error: &(dyn StdError + 'static), reason: &str) -> SpawnErrorKind {
    if let Some(kind) = find_spawn_error_kind_in_chain(error) {
        return kind;
    }

    let normalized_reason = reason.to_ascii_lowercase();
    if normalized_reason.contains("no viable candidates found in path")
        || normalized_reason.contains("command not found")
        || normalized_reason.contains("no such file or directory")
    {
        return SpawnErrorKind::NotFound;
    }

    if normalized_reason.contains("permission denied")
        || normalized_reason.contains("operation not permitted")
    {
        return SpawnErrorKind::PermissionDenied;
    }

    SpawnErrorKind::Other
}

fn find_spawn_error_kind_in_chain(error: &(dyn StdError + 'static)) -> Option<SpawnErrorKind> {
    let mut current = Some(error);
    while let Some(err) = current {
        if let Some(io_err) = err.downcast_ref::<io::Error>() {
            return match io_err.kind() {
                io::ErrorKind::NotFound => Some(SpawnErrorKind::NotFound),
                io::ErrorKind::PermissionDenied => Some(SpawnErrorKind::PermissionDenied),
                _ => None,
            };
        }
        current = err.source();
    }
    None
}

#[cfg(test)]
#[path = "pty_tests.rs"]
mod tests;
