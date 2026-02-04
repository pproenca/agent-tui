#![deny(clippy::all)]
//! Daemon infrastructure: session management, persistence, and metrics.

mod config;
mod file_lock;
mod lock_helpers;
mod metrics;
mod pty_session;
mod repository;
mod session;
mod signal_handler;
mod system_clock;
mod system_info;
mod terminal_state;

pub use crate::usecases::ports::SessionError;
pub use config::DaemonConfig;
pub use file_lock::LockFile;
pub use file_lock::remove_lock_file;
pub use metrics::DaemonMetrics;
pub use session::SessionManager;
pub use signal_handler::SignalHandler;
pub use system_clock::SystemClock;
pub use system_info::SystemInfo;
pub use terminal_state::TerminalState;

pub type Result<T> = std::result::Result<T, SessionError>;
