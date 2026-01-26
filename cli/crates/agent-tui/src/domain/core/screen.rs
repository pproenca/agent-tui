use super::CursorPosition;
use super::style::CellStyle;

pub trait ScreenGrid {
    fn rows(&self) -> usize;
    fn cols(&self) -> usize;
    fn cell(&self, row: usize, col: usize) -> Option<(char, CellStyle)>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenCell {
    pub ch: char,
    pub style: CellStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenSnapshot {
    pub cols: u16,
    pub rows: u16,
    pub cells: Vec<Vec<ScreenCell>>,
    pub cursor: CursorPosition,
}
