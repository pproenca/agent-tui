//! Vterm integration.

use std::io;
use std::sync::Arc;

use tattoy_wezterm_surface::CursorVisibility;
use tattoy_wezterm_term::Intensity;
use tattoy_wezterm_term::Terminal;
use tattoy_wezterm_term::TerminalConfiguration;
use tattoy_wezterm_term::TerminalSize;
use tattoy_wezterm_term::Underline;
use tattoy_wezterm_term::color::ColorAttribute;
use tattoy_wezterm_term::color::ColorPalette;

use crate::domain::core::CellStyle;
use crate::domain::core::Color;
use crate::domain::core::ScreenGrid;
use crate::domain::core::ScreenSnapshot;
use crate::usecases::ports::TerminalEngine;

#[derive(Debug, Clone)]
pub struct Cell {
    pub char: char,
    pub style: CellStyle,
}

#[derive(Debug, Clone)]
pub struct ScreenBuffer {
    pub cells: Vec<Vec<Cell>>,
}

impl ScreenGrid for ScreenBuffer {
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

pub use crate::domain::core::CursorPosition;

const DEFAULT_SCROLLBACK: usize = 1000;

#[derive(Debug, Default)]
struct DefaultTerminalConfig {
    palette: ColorPalette,
}

impl TerminalConfiguration for DefaultTerminalConfig {
    fn scrollback_size(&self) -> usize {
        DEFAULT_SCROLLBACK
    }

    fn color_palette(&self) -> ColorPalette {
        self.palette.clone()
    }
}

pub struct VirtualTerminal {
    terminal: Terminal,
    cols: u16,
    rows: u16,
}

impl VirtualTerminal {
    pub fn new(cols: u16, rows: u16) -> Self {
        let size = TerminalSize {
            rows: rows as usize,
            cols: cols as usize,
            pixel_width: 0,
            pixel_height: 0,
            dpi: 0,
        };
        let config: Arc<dyn TerminalConfiguration + Send + Sync> =
            Arc::new(DefaultTerminalConfig::default());
        let writer: Box<dyn io::Write + Send> = Box::new(io::sink());
        let terminal = Terminal::new(size, config, "agent-tui", env!("CARGO_PKG_VERSION"), writer);
        Self {
            terminal,
            cols,
            rows,
        }
    }

    pub fn process(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        self.terminal.advance_bytes(data);
    }

    pub fn screen_text(&self) -> String {
        let buffer = self.screen_buffer();
        let mut lines = Vec::new();
        for row in &buffer.cells {
            let mut line = String::new();
            for cell in row {
                line.push(cell.char);
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
        let screen = self.terminal.screen();
        let rows = screen.physical_rows;
        let cols = screen.physical_cols;
        let total_lines = screen.scrollback_rows();
        let start = total_lines.saturating_sub(rows);
        let end = start + rows;
        let lines = screen.lines_in_phys_range(start..end);

        let mut cells = Vec::with_capacity(rows);
        for line in lines {
            let mut row_cells = Vec::with_capacity(cols);
            row_cells.resize_with(cols, || Cell {
                char: ' ',
                style: CellStyle::default(),
            });

            for cell in line.visible_cells() {
                let idx = cell.cell_index();
                if idx >= cols {
                    continue;
                }
                let ch = cell.str().chars().next().unwrap_or(' ');
                let style = style_from_attrs(cell.attrs());
                row_cells[idx] = Cell { char: ch, style };
            }

            cells.push(row_cells);
        }

        ScreenBuffer { cells }
    }

    pub fn cursor(&self) -> CursorPosition {
        let cursor = self.terminal.cursor_pos();
        let row = if cursor.y < 0 {
            0
        } else {
            (cursor.y as u64).min(u16::MAX as u64) as u16
        };
        let col = cursor.x.min(u16::MAX as usize) as u16;
        CursorPosition {
            row,
            col,
            visible: matches!(cursor.visibility, CursorVisibility::Visible),
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        let size = TerminalSize {
            rows: rows as usize,
            cols: cols as usize,
            pixel_width: 0,
            pixel_height: 0,
            dpi: 0,
        };
        self.terminal.resize(size);
        self.cols = cols;
        self.rows = rows;
    }

    pub fn size(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }

    pub fn clear(&mut self) {
        self.terminal.erase_scrollback_and_viewport();
    }
}

impl TerminalEngine for VirtualTerminal {
    fn process_bytes(&mut self, bytes: &[u8]) {
        self.process(bytes);
    }

    fn resize(&mut self, cols: u16, rows: u16) {
        self.resize(cols, rows);
    }

    fn snapshot(&self) -> ScreenSnapshot {
        let buffer = self.screen_buffer();
        ScreenSnapshot {
            cols: self.cols,
            rows: self.rows,
            cells: buffer
                .cells
                .into_iter()
                .map(|row| {
                    row.into_iter()
                        .map(|cell| crate::domain::core::ScreenCell {
                            ch: cell.char,
                            style: cell.style,
                        })
                        .collect()
                })
                .collect(),
            cursor: self.cursor(),
        }
    }

    fn plain_text(&self) -> String {
        self.screen_text()
    }
}

fn style_from_attrs(attrs: &tattoy_wezterm_term::CellAttributes) -> CellStyle {
    let bold = matches!(attrs.intensity(), Intensity::Bold);
    let underline = !matches!(attrs.underline(), Underline::None);
    let inverse = attrs.reverse();

    let fg = convert_color(attrs.foreground());
    let bg = convert_color(attrs.background());

    CellStyle {
        bold,
        underline,
        inverse,
        fg_color: fg,
        bg_color: bg,
    }
}

fn convert_color(color: ColorAttribute) -> Option<Color> {
    match color {
        ColorAttribute::Default => Some(Color::Default),
        ColorAttribute::PaletteIndex(idx) => Some(Color::Indexed(idx)),
        ColorAttribute::TrueColorWithPaletteFallback(color, _)
        | ColorAttribute::TrueColorWithDefaultFallback(color) => {
            let (r, g, b, _) = color.as_rgba_u8();
            Some(Color::Rgb(r, g, b))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_terminal() {
        let mut term = VirtualTerminal::new(80, 24);
        term.process(b"Hello, World!");
        let text = term.screen_text();
        assert!(text.contains("Hello, World!"));
    }

    #[test]
    fn test_cursor_position() {
        let mut term = VirtualTerminal::new(80, 24);
        term.process(b"ABC");
        let cursor = term.cursor();
        assert_eq!(cursor.col, 3);
        assert_eq!(cursor.row, 0);
    }

    #[test]
    fn test_screen_buffer() {
        let mut term = VirtualTerminal::new(80, 24);
        term.process(b"\x1b[1mBold\x1b[0m Normal");
        let buffer = term.screen_buffer();

        assert!(buffer.cells[0][0].style.bold);
        assert_eq!(buffer.cells[0][0].char, 'B');
    }
}
