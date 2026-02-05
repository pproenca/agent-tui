#![deny(clippy::all)]

//! Core domain model for screen and terminal semantics.

pub mod screen;
pub mod style;

pub use screen::ScreenCell;
pub use screen::ScreenGrid;
pub use screen::ScreenSnapshot;
pub use style::CellStyle;
pub use style::Color;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorPosition {
    pub row: u16,
    pub col: u16,
    pub visible: bool,
}
