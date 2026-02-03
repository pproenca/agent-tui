use crate::infra::terminal::{PtyHandle, ReadEvent};
use crossbeam_channel::Receiver;

use crate::infra::daemon::SessionError;

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
        self.handle
            .write(data)
            .map_err(|err| SessionError::Terminal(err.to_port_error()))
    }

    pub fn write_str(&self, s: &str) -> Result<(), SessionError> {
        self.handle
            .write_str(s)
            .map_err(|err| SessionError::Terminal(err.to_port_error()))
    }

    pub fn try_read(&mut self, buf: &mut [u8], timeout_ms: i32) -> Result<usize, SessionError> {
        self.handle
            .try_read(buf, timeout_ms)
            .map_err(|err| SessionError::Terminal(err.to_port_error()))
    }

    pub(crate) fn take_read_rx(&mut self) -> Option<Receiver<ReadEvent>> {
        self.handle.take_read_rx()
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), SessionError> {
        self.handle
            .resize(cols, rows)
            .map_err(|err| SessionError::Terminal(err.to_port_error()))
    }

    pub fn kill(&mut self) -> Result<(), SessionError> {
        self.handle
            .kill()
            .map_err(|err| SessionError::Terminal(err.to_port_error()))
    }
}
