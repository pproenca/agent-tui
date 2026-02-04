pub mod unix_socket;

pub use unix_socket::{UnixSocketConnection, UnixSocketListener};

use crate::adapters::rpc::{RpcRequest, RpcResponse};
use std::time::Duration;

mod error;
pub(crate) use error::TransportError;

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
