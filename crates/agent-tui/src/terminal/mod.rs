#![deny(clippy::all)]

pub mod error;
mod pty;
mod vterm;

pub use error::PtyError;
pub use pty::PtyHandle;
pub use pty::key_to_escape_sequence;
pub use vterm::Cell;
pub use vterm::CursorPosition;
pub use vterm::ScreenBuffer;
pub use vterm::VirtualTerminal;

pub use crate::core::CellStyle;
pub use crate::core::Color;
pub use crate::core::ScreenGrid;

pub type Result<T> = std::result::Result<T, PtyError>;
