//! PTY session wrapper - intentional partial boundary.
//!
//! This module is an intentional partial boundary that wraps `PtyHandle` from
//! `agent_tui_terminal`. It exists at the root level because:
//!
//! 1. It's a thin wrapper with no business logic - just delegation and error mapping
//! 2. It adapts the terminal crate's PTY interface to the daemon's error types
//! 3. It provides a clean separation between PTY lifecycle and terminal emulation
//!
//! This follows Clean Architecture's guidance on partial boundaries: when the
//! cost of a full boundary isn't justified, a simpler separation can be used.

use agent_tui_terminal::PtyHandle;

use crate::error::SessionError;

/// Wraps PTY lifecycle operations.
///
/// Provides a clean interface for PTY I/O and lifecycle management,
/// separate from terminal emulation concerns.
pub struct PtySession {
    handle: PtyHandle,
}

impl PtySession {
    pub fn new(handle: PtyHandle) -> Self {
        Self { handle }
    }

    pub fn pid(&self) -> Option<u32> {
        self.handle.pid()
    }

    pub fn is_running(&mut self) -> bool {
        self.handle.is_running()
    }

    pub fn write(&self, data: &[u8]) -> Result<(), SessionError> {
        self.handle.write(data).map_err(SessionError::Pty)
    }

    pub fn write_str(&self, s: &str) -> Result<(), SessionError> {
        self.handle.write_str(s).map_err(SessionError::Pty)
    }

    pub fn try_read(&self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, SessionError> {
        self.handle
            .try_read(buf, timeout_ms)
            .map_err(SessionError::Pty)
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), SessionError> {
        self.handle.resize(cols, rows).map_err(SessionError::Pty)
    }

    pub fn kill(&mut self) -> Result<(), SessionError> {
        self.handle.kill().map_err(SessionError::Pty)
    }
}
