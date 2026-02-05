//! Daemon application wiring and startup logic.

pub mod rpc_core;
pub mod server;
pub mod transport;
pub mod ws_server;
mod usecase_container;

pub use server::start_daemon;
