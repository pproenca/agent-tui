//! Test support utilities for use case unit testing.
//!
//! This module provides mock implementations and builders for testing
//! use cases in isolation without requiring real PTY sessions.

mod element_builder;
mod mock_repository;
mod mock_session;

pub use element_builder::ElementBuilder;
pub use mock_repository::MockError;
pub use mock_repository::MockSessionRepository;
pub use mock_repository::MockSessionRepositoryBuilder;
pub use mock_repository::SpawnParams;
pub use mock_session::MockSession;
pub use mock_session::MockSessionBuilder;
