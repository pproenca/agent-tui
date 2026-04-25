//! Application use cases orchestrating domain and ports.

mod diagnostics;
mod input;
mod session;
mod shutdown;
mod snapshot;
mod spawn_error;
mod wait;
mod wait_condition;

pub use diagnostics::TerminalWriteUseCase;
pub use diagnostics::TerminalWriteUseCaseImpl;
pub use input::KeydownUseCase;
pub use input::KeydownUseCaseImpl;
pub use input::KeystrokeUseCase;
pub use input::KeystrokeUseCaseImpl;
pub use input::KeyupUseCase;
pub use input::KeyupUseCaseImpl;
pub use input::TypeUseCase;
pub use input::TypeUseCaseImpl;
pub use input::MouseClickUseCase;
pub use input::MouseClickUseCaseImpl;
pub use input::MouseMoveUseCase;
pub use input::MouseMoveUseCaseImpl;
pub use input::MouseDownUseCase;
pub use input::MouseDownUseCaseImpl;
pub use input::MouseUpUseCase;
pub use input::MouseUpUseCaseImpl;
pub use session::AssertUseCase;
pub use session::AssertUseCaseImpl;
pub use session::AttachUseCase;
pub use session::AttachUseCaseImpl;
pub use session::CleanupUseCase;
pub use session::CleanupUseCaseImpl;
pub use session::KillUseCase;
pub use session::KillUseCaseImpl;
pub use session::ResizeUseCase;
pub use session::ResizeUseCaseImpl;
pub use session::RestartUseCase;
pub use session::RestartUseCaseImpl;
pub use session::SessionsUseCase;
pub use session::SessionsUseCaseImpl;
pub use session::SpawnUseCase;
pub use session::SpawnUseCaseImpl;
pub use shutdown::ShutdownUseCase;
pub use shutdown::ShutdownUseCaseImpl;
pub use snapshot::SnapshotUseCase;
pub use snapshot::SnapshotUseCaseImpl;
pub use spawn_error::SpawnError;
pub use wait::WaitUseCase;
pub use wait::WaitUseCaseImpl;
pub mod ports;

#[cfg(test)]
mod input_mouse_tests;
