//! Daemon server for agent-tui
//!
//! Provides a persistent server that manages PTY sessions and handles
//! JSON-RPC requests over Unix socket.

pub mod server;

pub use server::{socket_path, start_daemon};
