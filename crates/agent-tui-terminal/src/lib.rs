#![deny(clippy::all)]

pub mod error;
mod pty;
mod terminal;

pub use error::PtyError;
pub use pty::PtyHandle;
pub use pty::key_to_escape_sequence;
pub use terminal::Cell;
pub use terminal::CellStyle;
pub use terminal::Color;
pub use terminal::CursorPosition;
pub use terminal::ScreenBuffer;
pub use terminal::VirtualTerminal;

pub type Result<T> = std::result::Result<T, PtyError>;
