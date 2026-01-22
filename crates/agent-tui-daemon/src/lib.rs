#![deny(clippy::all)]

pub mod ansi_keys;
mod lock_helpers;
mod select_helpers;
mod session;
mod wait;

pub use lock_helpers::acquire_session_lock;
pub use lock_helpers::LOCK_TIMEOUT;
pub use select_helpers::navigate_to_option;
pub use select_helpers::parse_select_options;
pub use select_helpers::strip_ansi_codes;
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
pub use wait::check_condition;
pub use wait::StableTracker;
pub use wait::WaitCondition;

pub type Result<T> = std::result::Result<T, SessionError>;
