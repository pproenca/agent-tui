#![deny(clippy::all)]

pub mod adapters;
pub mod app;
pub mod common;
pub mod domain;
pub mod infra;
pub mod usecases;

pub use app::Application;

pub use adapters::ipc::ClientError;
pub use adapters::ipc::DaemonClient;
pub use common::Colors;
pub use infra::daemon::Session;
pub use infra::daemon::SessionError;
pub use infra::daemon::SessionId;
pub use infra::daemon::SessionManager;

pub use app::error::AttachError;
pub use app::handlers::HandlerResult;
