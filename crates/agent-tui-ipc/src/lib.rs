#![deny(clippy::all)]

mod client;
pub mod daemon_lifecycle;
mod error;
mod mock_client;
pub mod params;
pub mod process;
mod snapshot_dto;
mod socket;
mod types;
pub mod version;

// Re-export error_codes from common for backwards compatibility
pub use agent_tui_common::error_codes;

// Re-export polling constants for external use
pub use client::polling;

pub use client::DaemonClient;
pub use client::DaemonClientConfig;
pub use client::PidLookupResult;
pub use client::StopDaemonResult;
pub use client::UnixSocketClient;
pub use client::ensure_daemon;
pub use client::get_daemon_pid;
pub use client::start_daemon_background;
pub use client::stop_daemon;
pub use daemon_lifecycle::StopResult;
pub use error::ClientError;
pub use mock_client::MockClient;
pub use process::{ProcessController, ProcessStatus, Signal, UnixProcessController};
pub use snapshot_dto::AccessibilitySnapshotDto;
pub use snapshot_dto::BoundsDto;
pub use snapshot_dto::ElementRefDto;
pub use snapshot_dto::RefMapDto;
pub use snapshot_dto::SnapshotStatsDto;
pub use socket::socket_path;
pub use types::ErrorData;
pub use types::RpcRequest;
pub use types::RpcResponse;
pub use types::RpcServerError;
pub use version::VersionCheckResult;
pub use version::VersionMismatch;

pub type Result<T> = std::result::Result<T, ClientError>;
