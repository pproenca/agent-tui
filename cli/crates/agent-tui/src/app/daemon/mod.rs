//! Daemon application wiring and startup logic.

pub mod rpc_core;
pub mod server;
pub mod transport;
mod usecase_container;
pub mod ws_server;

pub use server::start_daemon;
