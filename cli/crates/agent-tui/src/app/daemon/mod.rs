//! Daemon application wiring and startup logic.

pub mod http_api;
pub mod server;
pub mod transport;
mod usecase_container;

pub use server::start_daemon;
