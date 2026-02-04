pub mod unix_socket;

pub use unix_socket::{UnixSocketConnection, UnixSocketListener};

use crate::adapters::rpc::{RpcRequest, RpcResponse};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("I/O error: {0}")]
    Io(#[source] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
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
