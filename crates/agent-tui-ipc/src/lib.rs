#![deny(clippy::all)]

mod client;
mod error;
mod mock_client;
pub mod params;
mod snapshot_dto;
mod socket;
mod types;

// Re-export error_codes from common for backwards compatibility
pub use agent_tui_common::error_codes;

pub use client::DaemonClient;
pub use client::DaemonClientConfig;
pub use client::UnixSocketClient;
pub use client::ensure_daemon;
pub use client::start_daemon_background;
pub use error::ClientError;
pub use mock_client::MockClient;
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

pub type Result<T> = std::result::Result<T, ClientError>;
