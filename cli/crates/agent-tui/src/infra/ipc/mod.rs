#![deny(clippy::all)]

pub mod client;
pub mod daemon_lifecycle;
pub mod error;
mod mock_client;
pub mod polling;
pub mod process;
pub mod socket;
pub mod transport;
pub mod version;

pub use client::DaemonClient;
pub use client::DaemonClientConfig;
pub use client::PidLookupResult;
pub use client::UnixSocketClient;
pub use client::ensure_daemon;
pub use client::get_daemon_pid;
pub use daemon_lifecycle::StopResult;
pub use error::ClientError;
pub use mock_client::MockClient;
pub use process::{ProcessController, ProcessStatus, Signal, UnixProcessController};
pub use socket::socket_path;
pub use transport::InMemoryTransport;
pub use transport::IpcTransport;
pub use transport::TcpSocketTransport;
pub use transport::UnixSocketTransport;
pub use transport::default_transport;
pub use transport::start_daemon_background;
pub use version::VersionCheckResult;
pub use version::VersionMismatch;

pub type Result<T> = std::result::Result<T, ClientError>;
