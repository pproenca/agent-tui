#![deny(clippy::all)]

pub mod attach;
pub mod commands;
pub mod handlers;

pub use agent_tui_common::Colors;
pub use agent_tui_core::Element;
pub use agent_tui_core::ElementType;
pub use agent_tui_daemon::Session;
pub use agent_tui_daemon::SessionError;
pub use agent_tui_daemon::SessionId;
pub use agent_tui_daemon::SessionManager;
pub use agent_tui_ipc::ClientError;
pub use agent_tui_ipc::DaemonClient;
