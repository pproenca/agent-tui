//! CLI binary and command handlers for agent-tui.
//!
//! This crate provides the command-line interface for interacting with TUI applications
//! through the agent-tui daemon.

#![deny(clippy::all)]

pub mod attach;
pub mod commands;
pub mod error;
pub mod handlers;
pub mod presenter;

pub use agent_tui_common::Colors;
pub use agent_tui_core::Element;
pub use agent_tui_core::ElementType;
pub use agent_tui_daemon::Session;
pub use agent_tui_daemon::SessionError;
pub use agent_tui_daemon::SessionId;
pub use agent_tui_daemon::SessionManager;
pub use agent_tui_ipc::ClientError;
pub use agent_tui_ipc::DaemonClient;
pub use error::AttachError;
pub use handlers::HandlerResult;
