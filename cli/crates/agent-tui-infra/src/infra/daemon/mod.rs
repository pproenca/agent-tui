#![deny(clippy::all)]
//! Daemon infrastructure: session management and persistence.

mod config;
mod file_lock;
mod lock_helpers;
mod pty_session;
mod repository;
mod session;
mod signal_handler;
mod system_clock;
mod terminal_state;

pub use crate::usecases::ports::SessionError;
pub use config::DaemonConfig;
pub use file_lock::LockFile;
pub use file_lock::remove_lock_file;
pub use session::SessionManager;
pub use signal_handler::SignalHandler;
pub use system_clock::SystemClock;
pub use terminal_state::TerminalState;
