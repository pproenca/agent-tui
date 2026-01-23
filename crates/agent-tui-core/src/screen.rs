use crate::style::CellStyle;

pub trait ScreenGrid {
    fn rows(&self) -> usize;
    fn cols(&self) -> usize;
    fn cell(&self, row: usize, col: usize) -> Option<(char, CellStyle)>;
}
