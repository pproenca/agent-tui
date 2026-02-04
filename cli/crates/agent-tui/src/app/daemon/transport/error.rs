//! Transport error types for the daemon.

use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum TransportError {
    #[error("I/O error: {0}")]
    Io(#[source] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(#[source] serde_json::Error),
    #[error("Serialize error: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("Request size limit exceeded (max {max_bytes} bytes)")]
    SizeLimit { max_bytes: usize },
    #[error("Connection timeout")]
    Timeout,
    #[error("Connection closed")]
    ConnectionClosed,
}

impl From<std::io::Error> for TransportError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => Self::Timeout,
            std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::BrokenPipe => {
                Self::ConnectionClosed
            }
            _ => Self::Io(err),
        }
    }
}
