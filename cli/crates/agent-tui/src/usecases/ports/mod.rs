pub mod errors;
pub mod metrics;
pub mod session_repository;
pub mod shutdown_notifier;
pub mod terminal_engine;
#[cfg(test)]
pub(crate) mod test_support;

pub use errors::{LivePreviewError, PtyError, SessionError};
pub use metrics::MetricsProvider;
pub use session_repository::{
    LivePreviewSnapshot, SessionHandle, SessionOps, SessionRepository, StreamCursor, StreamRead,
    StreamSubscription,
};
pub use shutdown_notifier::{NoopShutdownNotifier, ShutdownNotifier, ShutdownNotifierHandle};
pub use terminal_engine::TerminalEngine;
