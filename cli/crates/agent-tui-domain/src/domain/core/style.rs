//! Screen styling types.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct CellStyle {
    pub bold: bool,
    pub underline: bool,
    pub inverse: bool,
    pub fg_color: Option<Color>,
    pub bg_color: Option<Color>,
}
