#![deny(clippy::all)]

pub mod error;
mod pty;
mod terminal;

pub use error::PtyError;
pub use pty::PtyHandle;
pub use pty::key_to_escape_sequence;
pub use terminal::Cell;
pub use terminal::CursorPosition;
pub use terminal::ScreenBuffer;
pub use terminal::VirtualTerminal;

pub use agent_tui_core::CellStyle;
pub use agent_tui_core::Color;
pub use agent_tui_core::ScreenGrid;

pub type Result<T> = std::result::Result<T, PtyError>;
