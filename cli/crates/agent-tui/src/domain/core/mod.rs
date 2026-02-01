#![deny(clippy::all)]

pub mod screen;
pub mod style;
pub mod vom;

#[cfg(test)]
pub mod test_fixtures;

pub use screen::ScreenCell;
pub use screen::ScreenGrid;
pub use screen::ScreenSnapshot;
pub use style::CellStyle;
pub use style::Color;
pub use vom::Cluster;
pub use vom::Component;
pub use vom::Rect;
pub use vom::Role;
pub use vom::analyze;
pub use vom::classify;
pub use vom::hash_cluster;
pub use vom::segment_buffer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorPosition {
    pub row: u16,
    pub col: u16,
    pub visible: bool,
}
