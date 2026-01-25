#![deny(clippy::all)]

pub mod adapters;
pub mod app;
pub mod common;
pub mod domain;
pub mod infra;
pub mod usecases;

pub use app::Application;

pub use common::Colors;
pub use domain::core::Element;
pub use domain::core::ElementType;
pub use infra::daemon::Session;
pub use infra::daemon::SessionError;
pub use infra::daemon::SessionId;
pub use infra::daemon::SessionManager;
pub use infra::ipc::ClientError;
pub use infra::ipc::DaemonClient;

pub use app::error::AttachError;
pub use app::handlers::HandlerResult;
