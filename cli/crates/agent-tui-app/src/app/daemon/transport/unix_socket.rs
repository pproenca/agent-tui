//! Unix socket transport for daemon RPC.

use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::RawFd;
use std::os::unix::net::UnixListener;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use crate::adapters::rpc::RpcRequest;
use crate::adapters::rpc::RpcResponse;

use super::TransportConnection;
use super::TransportError;
use super::TransportListener;

const DEFAULT_MAX_REQUEST_SIZE: usize = 1024 * 1024;

struct SizeLimitedReader<R> {
    inner: R,
    max_size: usize,
}

impl<R> SizeLimitedReader<R> {
    fn new(inner: R, max_size: usize) -> Self {
        Self { inner, max_size }
    }
}

impl<R: BufRead> SizeLimitedReader<R> {
    fn read_line(&mut self) -> Result<Option<String>, TransportError> {
        let mut line = String::new();
        match self.inner.read_line(&mut line) {
            Ok(0) => Ok(None),
            Ok(n) => {
                if n > self.max_size {
                    return Err(TransportError::SizeLimit {
                        max_bytes: self.max_size,
                    });
                }
                if line.ends_with('\n') {
                    line.pop();
                    if line.ends_with('\r') {
                        line.pop();
                    }
                }
                Ok(Some(line))
            }
            Err(e) => Err(TransportError::from(e)),
        }
    }
}

pub(crate) struct UnixSocketConnection {
    reader: SizeLimitedReader<BufReader<UnixStream>>,
    writer: UnixStream,
}

impl UnixSocketConnection {
    pub(crate) fn new(stream: UnixStream) -> Result<Self, TransportError> {
        Self::new_with_max(stream, DEFAULT_MAX_REQUEST_SIZE)
    }

    pub(crate) fn new_with_max(
        stream: UnixStream,
        max_request_bytes: usize,
    ) -> Result<Self, TransportError> {
        // Ensure accepted sockets are blocking so timeouts can be set reliably.
        let _ = stream.set_nonblocking(false);
        let reader_stream = stream.try_clone()?;
        Ok(Self {
            reader: SizeLimitedReader::new(BufReader::new(reader_stream), max_request_bytes),
            writer: stream,
        })
    }

    pub(crate) fn raw_fd(&self) -> RawFd {
        self.writer.as_raw_fd()
    }
}

impl TransportConnection for UnixSocketConnection {
    fn read_request(&mut self) -> Result<RpcRequest, TransportError> {
        loop {
            match self.reader.read_line()? {
                None => return Err(TransportError::ConnectionClosed),
                Some(line) if line.trim().is_empty() => continue,
                Some(line) => {
                    return serde_json::from_str(&line).map_err(TransportError::Parse);
                }
            }
        }
    }

    fn write_response(&mut self, response: &RpcResponse) -> Result<(), TransportError> {
        let json = serde_json::to_string(response).map_err(TransportError::Serialize)?;
        writeln!(self.writer, "{json}")?;
        Ok(())
    }

    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> Result<(), TransportError> {
        self.writer.set_read_timeout(timeout)?;
        Ok(())
    }

    fn set_write_timeout(&mut self, timeout: Option<Duration>) -> Result<(), TransportError> {
        self.writer.set_write_timeout(timeout)?;
        Ok(())
    }
}

pub(crate) struct UnixSocketListener {
    inner: UnixListener,
    max_request_bytes: usize,
}

impl UnixSocketListener {
    pub(crate) fn bind(path: &Path, max_request_bytes: usize) -> Result<Self, TransportError> {
        let listener = UnixListener::bind(path)?;
        Ok(Self {
            inner: listener,
            max_request_bytes,
        })
    }

    pub(crate) fn into_inner(self) -> UnixListener {
        self.inner
    }
}

impl AsRawFd for UnixSocketListener {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        self.inner.as_raw_fd()
    }
}

impl TransportListener for UnixSocketListener {
    type Connection = UnixSocketConnection;

    fn accept(&self) -> Result<Self::Connection, TransportError> {
        let (stream, _addr) = self.inner.accept()?;
        UnixSocketConnection::new_with_max(stream, self.max_request_bytes)
    }

    fn set_nonblocking(&self, nonblocking: bool) -> Result<(), TransportError> {
        self.inner.set_nonblocking(nonblocking)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "unix_socket_tests.rs"]
mod tests;
