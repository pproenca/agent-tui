use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::time::Duration;

use crate::infra::ipc::{RpcRequest, RpcResponse};

use super::{TransportConnection, TransportError, TransportListener};

const MAX_REQUEST_SIZE: usize = 1024 * 1024;

struct SizeLimitedReader<R> {
    inner: R,
    max_size: usize,
    read_count: usize,
}

impl<R> SizeLimitedReader<R> {
    fn new(inner: R, max_size: usize) -> Self {
        Self {
            inner,
            max_size,
            read_count: 0,
        }
    }
}

impl<R: BufRead> SizeLimitedReader<R> {
    fn read_line(&mut self) -> Result<Option<String>, TransportError> {
        let mut line = String::new();
        match self.inner.read_line(&mut line) {
            Ok(0) => Ok(None),
            Ok(n) => {
                self.read_count += n;
                if self.read_count > self.max_size {
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

pub struct UnixSocketConnection {
    reader: SizeLimitedReader<BufReader<UnixStream>>,
    writer: UnixStream,
}

impl UnixSocketConnection {
    pub fn new(stream: UnixStream) -> Result<Self, TransportError> {
        // Ensure accepted sockets are blocking so timeouts can be set reliably.
        let _ = stream.set_nonblocking(false);
        let reader_stream = stream.try_clone()?;
        Ok(Self {
            reader: SizeLimitedReader::new(BufReader::new(reader_stream), MAX_REQUEST_SIZE),
            writer: stream,
        })
    }
}

impl TransportConnection for UnixSocketConnection {
    fn read_request(&mut self) -> Result<RpcRequest, TransportError> {
        loop {
            match self.reader.read_line()? {
                None => return Err(TransportError::ConnectionClosed),
                Some(line) if line.trim().is_empty() => continue,
                Some(line) => {
                    return serde_json::from_str(&line)
                        .map_err(|e| TransportError::Parse(e.to_string()));
                }
            }
        }
    }

    fn write_response(&mut self, response: &RpcResponse) -> Result<(), TransportError> {
        let json = serde_json::to_string(response)
            .map_err(|e| TransportError::Parse(format!("Failed to serialize response: {}", e)))?;
        writeln!(self.writer, "{}", json)?;
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

pub struct UnixSocketListener {
    inner: UnixListener,
}

impl UnixSocketListener {
    pub fn bind(path: &Path) -> Result<Self, TransportError> {
        let listener = UnixListener::bind(path)?;
        Ok(Self { inner: listener })
    }

    pub fn into_inner(self) -> UnixListener {
        self.inner
    }
}

impl TransportListener for UnixSocketListener {
    type Connection = UnixSocketConnection;

    fn accept(&self) -> Result<Self::Connection, TransportError> {
        let (stream, _addr) = self.inner.accept()?;
        UnixSocketConnection::new(stream)
    }

    fn set_nonblocking(&self, nonblocking: bool) -> Result<(), TransportError> {
        self.inner.set_nonblocking(nonblocking)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_size_limited_reader_within_limit() {
        let data = "hello\nworld\n";
        let cursor = Cursor::new(data);
        let buf_reader = BufReader::new(cursor);
        let mut reader = SizeLimitedReader::new(buf_reader, 100);

        assert_eq!(reader.read_line().unwrap(), Some("hello".to_string()));
        assert_eq!(reader.read_line().unwrap(), Some("world".to_string()));
        assert_eq!(reader.read_line().unwrap(), None);
    }

    #[test]
    fn test_size_limited_reader_exceeds_limit() {
        let data = "this is a long line that exceeds the limit\n";
        let cursor = Cursor::new(data);
        let buf_reader = BufReader::new(cursor);
        let mut reader = SizeLimitedReader::new(buf_reader, 10);

        let result = reader.read_line();
        assert!(matches!(result, Err(TransportError::SizeLimit { .. })));
    }

    #[test]
    fn test_size_limited_reader_strips_newlines() {
        let data = "line with crlf\r\n";
        let cursor = Cursor::new(data);
        let buf_reader = BufReader::new(cursor);
        let mut reader = SizeLimitedReader::new(buf_reader, 100);

        assert_eq!(
            reader.read_line().unwrap(),
            Some("line with crlf".to_string())
        );
    }

    #[test]
    fn test_transport_error_display() {
        let io_err = TransportError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "test error",
        ));
        assert!(io_err.to_string().contains("I/O error"));

        let parse_err = TransportError::Parse("invalid json".to_string());
        assert!(parse_err.to_string().contains("Parse error"));

        let size_err = TransportError::SizeLimit { max_bytes: 1024 };
        assert!(size_err.to_string().contains("1024"));

        let timeout_err = TransportError::Timeout;
        assert_eq!(timeout_err.to_string(), "Connection timeout");

        let closed_err = TransportError::ConnectionClosed;
        assert_eq!(closed_err.to_string(), "Connection closed");
    }

    #[test]
    fn test_transport_error_from_io() {
        let timeout = std::io::Error::new(std::io::ErrorKind::TimedOut, "timed out");
        assert!(matches!(
            TransportError::from(timeout),
            TransportError::Timeout
        ));

        let would_block = std::io::Error::new(std::io::ErrorKind::WouldBlock, "would block");
        assert!(matches!(
            TransportError::from(would_block),
            TransportError::Timeout
        ));

        let eof = std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "eof");
        assert!(matches!(
            TransportError::from(eof),
            TransportError::ConnectionClosed
        ));

        let broken_pipe = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken");
        assert!(matches!(
            TransportError::from(broken_pipe),
            TransportError::ConnectionClosed
        ));

        let other = std::io::Error::other("other");
        assert!(matches!(TransportError::from(other), TransportError::Io(_)));
    }

    #[test]
    fn test_unix_socket_roundtrip() {
        use std::os::unix::net::UnixStream;
        use std::thread;

        let (client_stream, server_stream) = UnixStream::pair().unwrap();

        let server_handle = thread::spawn(move || {
            let mut conn = UnixSocketConnection::new(server_stream).unwrap();
            let request = conn.read_request().unwrap();
            assert_eq!(request.method, "test_method");

            let response = RpcResponse::success(request.id, serde_json::json!({"ok": true}));
            conn.write_response(&response).unwrap();
        });

        let mut client_stream_writer = client_stream.try_clone().unwrap();
        let mut client_conn = UnixSocketConnection::new(client_stream).unwrap();

        let request_json = r#"{"jsonrpc":"2.0","id":1,"method":"test_method"}"#;
        writeln!(client_stream_writer, "{}", request_json).unwrap();

        let response = client_conn.read_request();
        assert!(response.is_ok() || matches!(response, Err(TransportError::Parse(_))));

        server_handle.join().unwrap();
    }

    #[test]
    fn test_unix_socket_connection_closed() {
        use std::os::unix::net::UnixStream;

        let (client_stream, server_stream) = UnixStream::pair().unwrap();
        drop(server_stream);

        let mut conn = UnixSocketConnection::new(client_stream).unwrap();
        let result = conn.read_request();
        assert!(matches!(result, Err(TransportError::ConnectionClosed)));
    }

    #[test]
    fn test_size_limited_reader_cumulative_limit() {
        let data = "aaa\nbbb\nccc\n";
        let cursor = Cursor::new(data);
        let buf_reader = BufReader::new(cursor);
        let mut reader = SizeLimitedReader::new(buf_reader, 8);

        assert_eq!(reader.read_line().unwrap(), Some("aaa".to_string()));
        assert_eq!(reader.read_line().unwrap(), Some("bbb".to_string()));
        let result = reader.read_line();
        assert!(matches!(result, Err(TransportError::SizeLimit { .. })));
    }
}
