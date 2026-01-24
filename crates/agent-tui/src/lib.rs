#![deny(clippy::all)]

pub mod common;
pub mod core;
pub mod daemon;
pub mod ipc;
pub mod terminal;

pub mod app;
pub mod attach;
pub mod commands;
pub mod error;
pub mod handlers;
pub mod presenter;

pub use app::Application;

pub use common::Colors;
pub use core::Element;
pub use core::ElementType;
pub use daemon::Session;
pub use daemon::SessionError;
pub use daemon::SessionId;
pub use daemon::SessionManager;
pub use ipc::ClientError;
pub use ipc::DaemonClient;

pub use error::AttachError;
pub use handlers::HandlerResult;
