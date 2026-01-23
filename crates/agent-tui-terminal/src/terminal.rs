use std::sync::Arc;
use std::sync::Mutex;

use vt100::Parser;

use agent_tui_common::mutex_lock_or_recover;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct CellStyle {
    pub bold: bool,
    pub underline: bool,
    pub inverse: bool,
    pub fg_color: Option<Color>,
    pub bg_color: Option<Color>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Color {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone)]
pub struct Cell {
    pub char: char,
    pub style: CellStyle,
}

#[derive(Debug, Clone)]
pub struct ScreenBuffer {
    pub cells: Vec<Vec<Cell>>,
}

#[derive(Debug, Clone)]
pub struct CursorPosition {
    pub row: u16,
    pub col: u16,
    pub visible: bool,
}

pub struct VirtualTerminal {
    parser: Arc<Mutex<Parser>>,
    cols: u16,
    rows: u16,
}

const MAX_SCROLLBACK: usize = 1000;

impl VirtualTerminal {
    pub fn new(cols: u16, rows: u16) -> Self {
        let parser = Parser::new(rows, cols, MAX_SCROLLBACK);
        Self {
            parser: Arc::new(Mutex::new(parser)),
            cols,
            rows,
        }
    }

    pub fn process(&self, data: &[u8]) {
        let mut parser = mutex_lock_or_recover(&self.parser);
        parser.process(data);
    }

    pub fn screen_text(&self) -> String {
        let parser = mutex_lock_or_recover(&self.parser);
        let screen = parser.screen();

        let mut lines = Vec::new();
        for row in 0..screen.size().0 {
            let mut line = String::new();
            for col in 0..screen.size().1 {
                let cell = screen.cell(row, col);
                if let Some(cell) = cell {
                    line.push(cell.contents().chars().next().unwrap_or(' '));
                } else {
                    line.push(' ');
                }
            }

            let trimmed = line.trim_end();
            lines.push(trimmed.to_string());
        }

        while lines.last().map(|l| l.is_empty()).unwrap_or(false) {
            lines.pop();
        }

        lines.join("\n")
    }

    pub fn screen_buffer(&self) -> ScreenBuffer {
        let parser = mutex_lock_or_recover(&self.parser);
        let screen = parser.screen();

        let mut cells = Vec::new();
        for row in 0..screen.size().0 {
            let mut row_cells = Vec::new();
            for col in 0..screen.size().1 {
                let cell = screen.cell(row, col);
                let (char, style) = if let Some(cell) = cell {
                    let c = cell.contents().chars().next().unwrap_or(' ');
                    let s = CellStyle {
                        bold: cell.bold(),
                        underline: cell.underline(),
                        inverse: cell.inverse(),
                        fg_color: convert_color(cell.fgcolor()),
                        bg_color: convert_color(cell.bgcolor()),
                    };
                    (c, s)
                } else {
                    (' ', CellStyle::default())
                };
                row_cells.push(Cell { char, style });
            }
            cells.push(row_cells);
        }

        ScreenBuffer { cells }
    }

    pub fn cursor(&self) -> CursorPosition {
        let parser = mutex_lock_or_recover(&self.parser);
        let screen = parser.screen();
        let (row, col) = screen.cursor_position();

        CursorPosition {
            row,
            col,
            visible: !screen.hide_cursor(),
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        let mut parser = mutex_lock_or_recover(&self.parser);
        parser.set_size(rows, cols);
        self.cols = cols;
        self.rows = rows;
    }

    pub fn size(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }

    pub fn clear(&mut self) {
        let rows = self.rows;
        let cols = self.cols;
        let mut parser = mutex_lock_or_recover(&self.parser);
        parser.set_size(rows, cols);
    }
}

fn convert_color(color: vt100::Color) -> Option<Color> {
    match color {
        vt100::Color::Default => Some(Color::Default),
        vt100::Color::Idx(idx) => Some(Color::Indexed(idx)),
        vt100::Color::Rgb(r, g, b) => Some(Color::Rgb(r, g, b)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_terminal() {
        let term = VirtualTerminal::new(80, 24);
        term.process(b"Hello, World!");
        let text = term.screen_text();
        assert!(text.contains("Hello, World!"));
    }

    #[test]
    fn test_cursor_position() {
        let term = VirtualTerminal::new(80, 24);
        term.process(b"ABC");
        let cursor = term.cursor();
        assert_eq!(cursor.col, 3);
        assert_eq!(cursor.row, 0);
    }

    #[test]
    fn test_screen_buffer() {
        let term = VirtualTerminal::new(80, 24);
        term.process(b"\x1b[1mBold\x1b[0m Normal");
        let buffer = term.screen_buffer();

        assert!(buffer.cells[0][0].style.bold);
        assert_eq!(buffer.cells[0][0].char, 'B');
    }
}
