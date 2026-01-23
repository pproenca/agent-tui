#![deny(clippy::all)]

mod client;
mod error;
pub mod error_codes;
mod socket;
mod types;

pub use client::DaemonClient;
pub use client::DaemonClientConfig;
pub use client::ensure_daemon;
pub use client::start_daemon_background;
pub use error::ClientError;
pub use socket::socket_path;
pub use types::ErrorData;
pub use types::RpcRequest;
pub use types::RpcResponse;
pub use types::RpcServerError;

pub type Result<T> = std::result::Result<T, ClientError>;
