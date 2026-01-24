pub mod unix_socket;
pub mod websocket;

pub use unix_socket::{UnixSocketConnection, UnixSocketListener};

use crate::ipc::{RpcRequest, RpcResponse};
use std::time::Duration;

#[derive(Debug)]
pub enum TransportError {
    Io(std::io::Error),
    Parse(String),
    SizeLimit { max_bytes: usize },
    Timeout,
    ConnectionClosed,
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {}", e),
            Self::Parse(msg) => write!(f, "Parse error: {}", msg),
            Self::SizeLimit { max_bytes } => {
                write!(f, "Request size limit exceeded (max {} bytes)", max_bytes)
            }
            Self::Timeout => write!(f, "Connection timeout"),
            Self::ConnectionClosed => write!(f, "Connection closed"),
        }
    }
}

impl std::error::Error for TransportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
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

pub trait TransportConnection: Send {
    fn read_request(&mut self) -> Result<RpcRequest, TransportError>;
    fn write_response(&mut self, response: &RpcResponse) -> Result<(), TransportError>;
    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> Result<(), TransportError>;
    fn set_write_timeout(&mut self, timeout: Option<Duration>) -> Result<(), TransportError>;
}

pub trait TransportListener {
    type Connection: TransportConnection;
    fn accept(&self) -> Result<Self::Connection, TransportError>;
    fn set_nonblocking(&self, nonblocking: bool) -> Result<(), TransportError>;
}
