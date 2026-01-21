//! PTY management using portable-pty
//!
//! This module provides PTY creation and management for spawning
//! and controlling terminal applications.

use crate::sync_utils::mutex_lock_or_recover;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::os::fd::RawFd;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PtyError {
    #[error("Failed to open PTY: {0}")]
    OpenFailed(String),
    #[error("Failed to spawn process: {0}")]
    SpawnFailed(String),
    #[error("Failed to write to PTY: {0}")]
    WriteFailed(String),
    #[error("Failed to read from PTY: {0}")]
    ReadFailed(String),
    #[error("Failed to resize PTY: {0}")]
    ResizeFailed(String),
    #[error("Process not running")]
    NotRunning,
}

pub struct PtyHandle {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    reader: Arc<Mutex<Box<dyn Read + Send>>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    size: PtySize,
    reader_fd: RawFd,
}

impl PtyHandle {
    /// Spawn a new PTY with the given command
    pub fn spawn(
        command: &str,
        args: &[String],
        cwd: Option<&str>,
        env: Option<&std::collections::HashMap<String, String>>,
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
            .map_err(|e| PtyError::OpenFailed(e.to_string()))?;

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

        // Set TERM for proper terminal emulation
        cmd.env("TERM", "xterm-256color");

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::SpawnFailed(e.to_string()))?;

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::OpenFailed(e.to_string()))?;

        // Get raw fd for non-blocking poll from the master
        let reader_fd = pair
            .master
            .as_raw_fd()
            .ok_or_else(|| PtyError::OpenFailed("Failed to get master fd".to_string()))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::OpenFailed(e.to_string()))?;

        Ok(Self {
            master: pair.master,
            child,
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
            size,
            reader_fd,
        })
    }

    /// Get the process ID
    pub fn pid(&self) -> Option<u32> {
        self.child.process_id()
    }

    /// Check if the process is still running
    pub fn is_running(&mut self) -> bool {
        self.child
            .try_wait()
            .map(|status| status.is_none())
            .unwrap_or(false)
    }

    /// Write data to the PTY
    pub fn write(&self, data: &[u8]) -> Result<(), PtyError> {
        let mut writer = mutex_lock_or_recover(&self.writer);
        writer
            .write_all(data)
            .map_err(|e| PtyError::WriteFailed(e.to_string()))?;
        writer
            .flush()
            .map_err(|e| PtyError::WriteFailed(e.to_string()))?;
        Ok(())
    }

    /// Write a string to the PTY
    pub fn write_str(&self, s: &str) -> Result<(), PtyError> {
        self.write(s.as_bytes())
    }

    /// Read available data from the PTY (non-blocking)
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, PtyError> {
        let mut reader = mutex_lock_or_recover(&self.reader);
        reader
            .read(buf)
            .map_err(|e| PtyError::ReadFailed(e.to_string()))
    }

    /// Read available data without blocking
    /// Returns Ok(0) if no data available within timeout
    pub fn try_read(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, PtyError> {
        // Use poll to check if data is available
        let mut pollfd = libc::pollfd {
            fd: self.reader_fd,
            events: libc::POLLIN,
            revents: 0,
        };

        let result = unsafe { libc::poll(&mut pollfd, 1, timeout_ms) };

        if result < 0 {
            return Err(PtyError::ReadFailed("poll failed".to_string()));
        }

        if result == 0 {
            // Timeout, no data available
            return Ok(0);
        }

        // Data available, read it
        let mut reader = mutex_lock_or_recover(&self.reader);
        reader
            .read(buf)
            .map_err(|e| PtyError::ReadFailed(e.to_string()))
    }

    /// Resize the PTY
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), PtyError> {
        self.size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        self.master
            .resize(self.size)
            .map_err(|e| PtyError::ResizeFailed(e.to_string()))
    }

    /// Get the current size
    pub fn size(&self) -> (u16, u16) {
        (self.size.cols, self.size.rows)
    }

    /// Kill the process
    pub fn kill(&mut self) -> Result<(), PtyError> {
        self.child
            .kill()
            .map_err(|e| PtyError::SpawnFailed(e.to_string()))
    }

    /// Wait for the process to exit
    pub fn wait(&mut self) -> Result<Option<portable_pty::ExitStatus>, PtyError> {
        self.child
            .try_wait()
            .map_err(|e| PtyError::SpawnFailed(e.to_string()))
    }

    /// Get a clone of the reader for async operations
    pub fn reader(&self) -> Arc<Mutex<Box<dyn Read + Send>>> {
        Arc::clone(&self.reader)
    }

    /// Get the reader file descriptor for polling
    pub fn reader_fd(&self) -> RawFd {
        self.reader_fd
    }
}

/// Parse a key string into escape sequences
pub fn key_to_escape_sequence(key: &str) -> Option<Vec<u8>> {
    // Handle modifier combinations
    if key.contains('+') {
        let parts: Vec<&str> = key.split('+').collect();
        if parts.len() == 2 {
            let modifier = parts[0];
            let base_key = parts[1];

            return match modifier.to_lowercase().as_str() {
                "ctrl" | "control" => {
                    // Ctrl+letter produces ASCII 1-26
                    if base_key.len() == 1 {
                        let c = base_key.chars().next()?.to_ascii_uppercase();
                        if c.is_ascii_alphabetic() {
                            return Some(vec![(c as u8) - b'A' + 1]);
                        }
                    }
                    // Special ctrl combinations
                    match base_key.to_lowercase().as_str() {
                        "c" => Some(vec![3]),   // Ctrl+C (SIGINT)
                        "d" => Some(vec![4]),   // Ctrl+D (EOF)
                        "z" => Some(vec![26]),  // Ctrl+Z (SIGTSTP)
                        "\\" => Some(vec![28]), // Ctrl+\ (SIGQUIT)
                        "[" => Some(vec![27]),  // Ctrl+[ (ESC)
                        _ => None,
                    }
                }
                "alt" | "meta" => {
                    // Alt+key sends ESC followed by the key
                    let base = key_to_escape_sequence(base_key)?;
                    let mut result = vec![0x1b]; // ESC
                    result.extend(base);
                    Some(result)
                }
                "shift" => {
                    // For function keys, shift modifies the escape sequence
                    match base_key.to_lowercase().as_str() {
                        "tab" => Some(vec![0x1b, b'[', b'Z']), // Shift+Tab
                        _ => {
                            // For letters, just uppercase
                            if base_key.len() == 1 {
                                Some(base_key.to_uppercase().as_bytes().to_vec())
                            } else {
                                None
                            }
                        }
                    }
                }
                _ => None,
            };
        }
    }

    // Handle single keys
    match key {
        // Standard keys
        "Enter" | "Return" => Some(vec![b'\r']),
        "Tab" => Some(vec![b'\t']),
        "Escape" | "Esc" => Some(vec![0x1b]),
        "Backspace" => Some(vec![0x7f]),
        "Delete" => Some(vec![0x1b, b'[', b'3', b'~']),
        "Space" => Some(vec![b' ']),

        // Arrow keys
        "ArrowUp" | "Up" => Some(vec![0x1b, b'[', b'A']),
        "ArrowDown" | "Down" => Some(vec![0x1b, b'[', b'B']),
        "ArrowRight" | "Right" => Some(vec![0x1b, b'[', b'C']),
        "ArrowLeft" | "Left" => Some(vec![0x1b, b'[', b'D']),

        // Navigation keys
        "Home" => Some(vec![0x1b, b'[', b'H']),
        "End" => Some(vec![0x1b, b'[', b'F']),
        "PageUp" => Some(vec![0x1b, b'[', b'5', b'~']),
        "PageDown" => Some(vec![0x1b, b'[', b'6', b'~']),
        "Insert" => Some(vec![0x1b, b'[', b'2', b'~']),

        // Function keys
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

        // Single character
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
