use std::collections::HashMap;
use std::collections::VecDeque;
use std::io;
use std::io::Read;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use crossbeam_channel as channel;
use libc::{POLLERR, POLLHUP, POLLOUT, poll, pollfd};
use portable_pty::Child;
use portable_pty::CommandBuilder;
use portable_pty::MasterPty;
use portable_pty::PtySize;
use portable_pty::native_pty_system;
use tracing::{debug, warn};

use crate::common::mutex_lock_or_recover;
use crate::usecases::ports::SpawnErrorKind;

pub use crate::infra::terminal::error::PtyError;

pub struct PtyHandle {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    size: PtySize,
    read_rx: Option<channel::Receiver<ReadEvent>>,
    read_buffer: VecDeque<u8>,
    read_closed: bool,
    read_error: Option<String>,
}

impl Drop for PtyHandle {
    fn drop(&mut self) {
        if self.is_running() {
            let _ = self.kill();
        }
    }
}

impl PtyHandle {
    pub fn spawn(
        command: &str,
        args: &[String],
        cwd: Option<&str>,
        env: Option<&HashMap<String, String>>,
        cols: u16,
        rows: u16,
    ) -> Result<Self, PtyError> {
        let pty_system = native_pty_system();

        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system
            .openpty(size)
            .map_err(|e| PtyError::Open(e.to_string()))?;

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

        let child = pair.slave.spawn_command(cmd).map_err(|e| {
            let kind = if let Some(io_err) = e.downcast_ref::<io::Error>() {
                match io_err.kind() {
                    io::ErrorKind::NotFound => SpawnErrorKind::NotFound,
                    io::ErrorKind::PermissionDenied => SpawnErrorKind::PermissionDenied,
                    _ => SpawnErrorKind::Other,
                }
            } else {
                SpawnErrorKind::Other
            };
            PtyError::Spawn {
                reason: e.to_string(),
                kind,
            }
        })?;

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::Open(e.to_string()))?;
        let read_rx = spawn_reader(reader);

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::Open(e.to_string()))?;

        Ok(Self {
            master: pair.master,
            child,
            writer: Arc::new(Mutex::new(writer)),
            size,
            read_rx: Some(read_rx),
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
                    return Err(PtyError::Write(
                        "write returned 0 bytes, PTY closed".to_string(),
                    ));
                }
                Ok(n) => offset += n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    self.wait_writable()?;
                }
                Err(e) => return Err(PtyError::Write(e.to_string())),
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
                let rc = unsafe { poll(fds.as_mut_ptr(), 1, -1) };
                if rc < 0 {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(PtyError::Write(err.to_string()));
                }
                let events = fds[0].revents;
                if events & (POLLHUP | POLLERR) != 0 {
                    return Err(PtyError::Write("PTY closed".to_string()));
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
                return Err(PtyError::Read(error));
            }
            return Ok(0);
        }

        if self.read_buffer.is_empty() && !self.read_closed {
            let mut events = Vec::new();
            {
                let read_rx = match self.read_rx.as_ref() {
                    Some(rx) => rx,
                    None => {
                        return Err(PtyError::Read(
                            "PTY read channel is not available".to_string(),
                        ));
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
                    events.push(event);
                }

                while let Ok(event) = read_rx.try_recv() {
                    events.push(event);
                }
            }

            for event in events {
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

        if total == 0 && self.read_closed {
            if let Some(error) = self.read_error.take() {
                return Err(PtyError::Read(error));
            }
        }

        Ok(total)
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), PtyError> {
        self.size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        self.master
            .resize(self.size)
            .map_err(|e| PtyError::Resize(e.to_string()))
    }

    pub fn kill(&mut self) -> Result<(), PtyError> {
        if !self.is_running() {
            return Ok(());
        }

        self.child.kill().map_err(|e| PtyError::Spawn {
            reason: e.to_string(),
            kind: SpawnErrorKind::Other,
        })
    }

    pub(crate) fn take_read_rx(&mut self) -> Option<channel::Receiver<ReadEvent>> {
        self.read_rx.take()
    }
}

impl PtyHandle {
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

pub(crate) enum ReadEvent {
    Data(Vec<u8>),
    Eof,
    Error(String),
}

const PTY_READ_CHANNEL_CAPACITY: usize = 256;

fn spawn_reader(mut reader: Box<dyn Read + Send>) -> channel::Receiver<ReadEvent> {
    let (tx, rx) = channel::bounded(PTY_READ_CHANNEL_CAPACITY);
    let span = tracing::debug_span!("pty_reader");
    let builder = std::thread::Builder::new().name("pty-reader".to_string());
    let tx_thread = tx.clone();
    if let Err(err) = builder.spawn(move || {
        let _guard = span.enter();
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    let _ = tx_thread.send(ReadEvent::Eof);
                    debug!("PTY reader EOF");
                    break;
                }
                Ok(n) => {
                    if tx_thread.send(ReadEvent::Data(buf[..n].to_vec())).is_err() {
                        break;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    warn!(error = %e, "PTY reader error");
                    let _ = tx_thread.send(ReadEvent::Error(e.to_string()));
                    break;
                }
            }
        }
    }) {
        let _ = tx.send(ReadEvent::Error(err.to_string()));
    }
    rx
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_to_escape_sequence() {
        assert_eq!(key_to_escape_sequence("Enter"), Some(vec![b'\r']));
        assert_eq!(key_to_escape_sequence("Tab"), Some(vec![b'\t']));
        assert_eq!(key_to_escape_sequence("Escape"), Some(vec![0x1b]));
        assert_eq!(
            key_to_escape_sequence("ArrowUp"),
            Some(vec![0x1b, b'[', b'A'])
        );
        assert_eq!(key_to_escape_sequence("Ctrl+C"), Some(vec![3]));
        assert_eq!(key_to_escape_sequence("a"), Some(vec![b'a']));
    }
}
