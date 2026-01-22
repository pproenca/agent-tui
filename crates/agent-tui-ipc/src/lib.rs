#![deny(clippy::all)]

mod client;
mod error;
mod error_messages;
mod socket;
mod types;

pub use client::DaemonClient;
pub use client::DaemonClientConfig;
pub use client::ensure_daemon;
pub use client::start_daemon_background;
pub use error::ClientError;
pub use error_messages::ai_friendly_error;
pub use error_messages::lock_timeout_response;
pub use socket::socket_path;
pub use types::RpcRequest;
pub use types::RpcResponse;
pub use types::RpcServerError;

pub type Result<T> = std::result::Result<T, ClientError>;
