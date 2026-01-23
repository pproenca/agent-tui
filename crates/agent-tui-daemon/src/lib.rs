#![deny(clippy::all)]

pub mod ansi_keys;
mod config;
mod error;
mod lock_helpers;
mod metrics;
mod select_helpers;
mod server;
mod session;
mod wait;

pub use config::DaemonConfig;
pub use error::DomainError;
pub use lock_helpers::LOCK_TIMEOUT;
pub use lock_helpers::MAX_BACKOFF;
pub use lock_helpers::acquire_session_lock;
pub use metrics::DaemonMetrics;
pub use select_helpers::navigate_to_option;
pub use select_helpers::parse_select_options;
pub use select_helpers::strip_ansi_codes;
pub use server::start_daemon;
pub use session::ErrorEntry;
pub use session::PersistedSession;
pub use session::RecordingFrame;
pub use session::RecordingStatus;
pub use session::Session;
pub use session::SessionError;
pub use session::SessionId;
pub use session::SessionInfo;
pub use session::SessionManager;
pub use session::SessionPersistence;
pub use session::TraceEntry;
pub use wait::StableTracker;
pub use wait::WaitCondition;
pub use wait::check_condition;

pub type Result<T> = std::result::Result<T, SessionError>;
