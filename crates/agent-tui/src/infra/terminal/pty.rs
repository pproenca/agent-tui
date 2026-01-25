use std::collections::HashMap;
use std::io;
use std::io::Read;
use std::io::Write;
use std::os::fd::RawFd;
use std::sync::Arc;
use std::sync::Mutex;

use portable_pty::Child;
use portable_pty::CommandBuilder;
use portable_pty::MasterPty;
use portable_pty::PtySize;
use portable_pty::native_pty_system;

use crate::common::mutex_lock_or_recover;

pub use crate::infra::terminal::error::PtyError;

pub struct PtyHandle {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    reader: Arc<Mutex<Box<dyn Read + Send>>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    size: PtySize,
    reader_fd: RawFd,
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

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::Spawn(e.to_string()))?;

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::Open(e.to_string()))?;

        let reader_fd = pair
            .master
            .as_raw_fd()
            .ok_or_else(|| PtyError::Open("Failed to get master fd".to_string()))?;

        set_non_blocking(reader_fd)?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::Open(e.to_string()))?;

        Ok(Self {
            master: pair.master,
            child,
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
            size,
            reader_fd,
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
                    wait_writable(self.reader_fd)?;
                }
                Err(e) => return Err(PtyError::Write(e.to_string())),
            }
        }
        Ok(())
    }

    pub fn write_str(&self, s: &str) -> Result<(), PtyError> {
        self.write(s.as_bytes())
    }

    pub fn try_read(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, PtyError> {
        if buf.is_empty() {
            return Ok(0);
        }

        let ready = wait_readable(self.reader_fd, timeout_ms)?;
        if !ready {
            return Ok(0);
        }

        let mut reader = mutex_lock_or_recover(&self.reader);
        let mut total = 0;
        loop {
            match reader.read(&mut buf[total..]) {
                Ok(0) => break,
                Ok(n) => {
                    total += n;
                    if total == buf.len() {
                        break;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(PtyError::Read(e.to_string())),
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

        self.child
            .kill()
            .map_err(|e| PtyError::Spawn(e.to_string()))
    }
}

fn set_non_blocking(fd: RawFd) -> Result<(), PtyError> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(PtyError::Open(io::Error::last_os_error().to_string()));
    }

    if flags & libc::O_NONBLOCK != 0 {
        return Ok(());
    }

    let result = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
    if result < 0 {
        return Err(PtyError::Open(io::Error::last_os_error().to_string()));
    }

    Ok(())
}

fn wait_readable(fd: RawFd, timeout_ms: i32) -> Result<bool, PtyError> {
    let mut pollfd = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };

    let result = unsafe { libc::poll(&mut pollfd, 1, timeout_ms) };
    if result < 0 {
        return Err(PtyError::Read(io::Error::last_os_error().to_string()));
    }
    if result == 0 {
        return Ok(false);
    }

    if pollfd.revents & (libc::POLLERR | libc::POLLNVAL) != 0 {
        return Err(PtyError::Read("poll error on PTY".to_string()));
    }

    if pollfd.revents & libc::POLLHUP != 0 && pollfd.revents & libc::POLLIN == 0 {
        return Ok(false);
    }

    Ok(pollfd.revents & libc::POLLIN != 0)
}

fn wait_writable(fd: RawFd) -> Result<(), PtyError> {
    let mut pollfd = libc::pollfd {
        fd,
        events: libc::POLLOUT,
        revents: 0,
    };

    let result = unsafe { libc::poll(&mut pollfd, 1, -1) };
    if result < 0 {
        return Err(PtyError::Write(io::Error::last_os_error().to_string()));
    }

    if pollfd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
        return Err(PtyError::Write("poll error on PTY".to_string()));
    }

    Ok(())
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
