#![deny(clippy::all)]

pub mod error;
mod pty;
mod render;
mod vterm;

pub use pty::PtyHandle;
pub(crate) use pty::ReadEvent;
pub use pty::key_to_escape_sequence;
pub(crate) use pty::keycode_to_escape_sequence;
pub use render::render_screen;
pub use vterm::CursorPosition;
pub use vterm::ScreenBuffer;
pub use vterm::VirtualTerminal;

pub use crate::domain::core::CellStyle;
pub use crate::domain::core::Color;
