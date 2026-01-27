#![deny(clippy::all)]
mod config;
mod file_lock;
mod lock_helpers;
mod metrics;
mod pty_session;
mod repository;
mod session;
mod signal_handler;
mod terminal_state;
#[cfg(test)]
pub mod test_support;

pub use crate::usecases::ports::SessionError;
pub use config::DaemonConfig;
pub use file_lock::LockFile;
pub use file_lock::remove_lock_file;
pub use lock_helpers::LOCK_TIMEOUT;
pub use lock_helpers::MAX_BACKOFF;
pub use lock_helpers::acquire_session_lock;
pub use metrics::DaemonMetrics;
pub use pty_session::PtySession;
pub use repository::SessionSnapshot;
pub use session::DEFAULT_MAX_SESSIONS;
pub use session::PersistedSession;
pub use session::Session;
pub use session::SessionId;
pub use session::SessionInfo;
pub use session::SessionManager;
pub use session::SessionPersistence;
pub use signal_handler::SignalHandler;
pub use terminal_state::TerminalState;

pub type Result<T> = std::result::Result<T, SessionError>;
