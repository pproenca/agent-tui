//! CLI binary and command handlers for agent-tui.
//!
//! This crate provides the command-line interface for interacting with TUI applications
//! through the agent-tui daemon.

#![deny(clippy::all)]

// Internal modules (previously separate crates)
pub mod common;
pub mod core;
pub mod daemon;
pub mod ipc;
pub mod terminal;

// CLI modules
pub mod app;
pub mod attach;
pub mod commands;
pub mod error;
pub mod handlers;
pub mod presenter;

pub use app::Application;

// Re-exports from internal modules
pub use common::Colors;
pub use core::Element;
pub use core::ElementType;
pub use daemon::Session;
pub use daemon::SessionError;
pub use daemon::SessionId;
pub use daemon::SessionManager;
pub use ipc::ClientError;
pub use ipc::DaemonClient;

// Re-exports from CLI modules
pub use error::AttachError;
pub use handlers::HandlerResult;
