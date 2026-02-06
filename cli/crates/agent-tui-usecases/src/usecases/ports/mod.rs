//! Port interfaces owned by use cases.

pub mod clock;
pub mod errors;
pub mod session_repository;
pub mod shutdown_notifier;
pub mod terminal_engine;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use clock::Clock;
pub use errors::SessionError;
pub use errors::SpawnErrorKind;
pub use errors::TerminalError;
pub use session_repository::LivePreviewSnapshot;
pub use session_repository::SessionHandle;
pub use session_repository::SessionOps;
pub use session_repository::SessionRepository;
pub use session_repository::StreamCursor;
pub use session_repository::StreamRead;
pub use session_repository::StreamWaiter;
pub use session_repository::StreamWaiterHandle;
pub use shutdown_notifier::ShutdownNotifier;
pub use shutdown_notifier::ShutdownNotifierHandle;
pub use terminal_engine::TerminalEngine;
