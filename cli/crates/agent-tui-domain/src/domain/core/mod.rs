#![deny(clippy::all)]

//! Core domain model for screen and terminal semantics.

pub mod style;

pub use style::CellStyle;
pub use style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPosition {
    pub row: u16,
    pub col: u16,
    pub visible: bool,
}
