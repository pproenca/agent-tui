#![deny(clippy::all)]

//! IPC layer for CLI/daemon coordination and lifecycle management.

pub mod client;
pub mod daemon_lifecycle;
pub mod error;
mod mock_client;
pub mod polling;
pub mod process;
pub mod socket;
pub mod transport;

pub use client::DaemonClient;
pub use client::DaemonClientConfig;
pub use client::DaemonProcessLookupResult;
pub use client::PidLookupResult;
pub use client::UnixSocketClient;
pub use client::ensure_daemon;
pub use client::get_daemon_pid;
pub use client::get_daemon_process_identity;
pub use error::ClientError;
pub use mock_client::MockClient;
pub use process::ProcessController;
pub use process::ProcessIdentity;
pub use process::ProcessStatus;
pub use process::Signal;
pub use process::UnixProcessController;
pub use process::check_expected_process;
pub use process::current_process_identity;
pub use socket::socket_path;
pub use transport::daemon_uses_client_working_directory;
pub use transport::start_daemon_background;
