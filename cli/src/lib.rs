//! agent-tui - Library for AI agents to interact with TUI applications
//!
//! This library provides the core functionality for managing PTY sessions,
//! detecting UI elements, and interacting with terminal applications.
//!
//! # Modules
//!
//! - [`session`] - Session management and lifecycle
//! - [`detection`] - UI element detection (buttons, inputs, checkboxes, etc.)
//! - [`terminal`] - Virtual terminal emulation
//! - [`pty`] - PTY (pseudo-terminal) handling
//! - [`wait`] - Wait conditions for synchronization

pub mod detection;
pub mod pty;
pub mod session;
pub mod sync_utils;
pub mod terminal;
pub mod wait;

pub use detection::{Element, ElementDetector, Framework};
pub use pty::{PtyError, PtyHandle};
pub use session::{Session, SessionError, SessionId, SessionInfo, SessionManager};
pub use terminal::{CursorPosition, ScreenBuffer, VirtualTerminal};
pub use wait::WaitCondition;
