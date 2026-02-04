use super::screen::ScreenGrid;
use super::style::CellStyle;
use super::style::Color;
use super::vom::Cluster;
use super::vom::Component;
use super::vom::Rect;
use super::vom::Role;

#[derive(Debug, Clone)]
pub struct Cell {
    pub char: char,
    pub style: CellStyle,
}

#[derive(Debug)]
pub struct MockScreenBuffer {
    pub cells: Vec<Vec<Cell>>,
}

impl MockScreenBuffer {
    pub fn new(cols: usize, rows: usize) -> Self {
        let cells = (0..rows)
            .map(|_| {
                (0..cols)
                    .map(|_| Cell {
                        char: ' ',
                        style: CellStyle::default(),
                    })
                    .collect()
            })
            .collect();
        Self { cells }
    }

    pub fn set_line(&mut self, row: usize, content: &[(char, CellStyle)]) {
        if row >= self.cells.len() {
            return;
        }
        for (col, (ch, style)) in content.iter().enumerate() {
            if col < self.cells[row].len() {
                self.cells[row][col] = Cell {
                    char: *ch,
                    style: style.clone(),
                };
            }
        }
    }
}

impl ScreenGrid for MockScreenBuffer {
    fn rows(&self) -> usize {
        self.cells.len()
    }

    fn cols(&self) -> usize {
        self.cells.first().map(|r| r.len()).unwrap_or(0)
    }

    fn cell(&self, row: usize, col: usize) -> Option<(char, CellStyle)> {
        self.cells
            .get(row)
            .and_then(|r| r.get(col))
            .map(|c| (c.char, c.style.clone()))
    }
}

pub fn make_cell(char: char, bold: bool, bg: Option<Color>) -> Cell {
    Cell {
        char,
        style: CellStyle {
            bold,
            underline: false,
            inverse: false,
            fg_color: None,
            bg_color: bg,
        },
    }
}

pub fn make_buffer(cells: Vec<Vec<Cell>>) -> MockScreenBuffer {
    MockScreenBuffer { cells }
}

pub fn make_cluster(text: &str, style: CellStyle, x: u16, y: u16) -> Cluster {
    Cluster {
        rect: Rect::new(x, y, text.len() as u16, 1),
        text: text.to_string(),
        style,
        is_whitespace: false,
    }
}

pub fn make_component(role: Role, text: &str, x: u16, y: u16, width: u16) -> Component {
    Component {
        role,
        bounds: Rect::new(x, y, width, 1),
        text_content: text.to_string(),
        visual_hash: 0,
        selected: false,
    }
}
