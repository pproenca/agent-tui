//! Vterm integration.

use std::io;
use std::sync::Arc;

use tattoy_wezterm_surface::CursorVisibility;
use tattoy_wezterm_term::Intensity;
use tattoy_wezterm_term::Terminal;
use tattoy_wezterm_term::TerminalConfiguration;
use tattoy_wezterm_term::TerminalSize as WeztermTerminalSize;
use tattoy_wezterm_term::Underline;
use tattoy_wezterm_term::color::ColorAttribute;
use tattoy_wezterm_term::color::ColorPalette;

use crate::domain::core::CellStyle;
use crate::domain::core::Color;
use crate::domain::session_types::TerminalSize;

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
pub(crate) struct TrimmedScreenBuffer<'a> {
    pub rows: Vec<&'a [Cell]>,
    pub total_trimmed_chars: usize,
}

impl ScreenBuffer {
    pub(crate) fn trimmed_rows(&self) -> Option<TrimmedScreenBuffer<'_>> {
        let mut rows = Vec::with_capacity(self.cells.len());
        let mut last_non_empty_row = None;
        let mut total_trimmed_chars = 0usize;

        for (row_idx, row) in self.cells.iter().enumerate() {
            let trimmed_len = row
                .iter()
                .rposition(|cell| !cell.char.is_whitespace())
                .map(|idx| idx + 1)
                .unwrap_or(0);
            if trimmed_len > 0 {
                last_non_empty_row = Some(row_idx);
                total_trimmed_chars += trimmed_len;
            }
            rows.push(&row[..trimmed_len]);
        }

        let rendered_rows = last_non_empty_row.map(|row_idx| row_idx + 1)?;
        rows.truncate(rendered_rows);

        Some(TrimmedScreenBuffer {
            rows,
            total_trimmed_chars,
        })
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
    size: TerminalSize,
}

impl VirtualTerminal {
    pub fn new(size: TerminalSize) -> Self {
        let config: Arc<dyn TerminalConfiguration + Send + Sync> =
            Arc::new(DefaultTerminalConfig::default());
        let writer: Box<dyn io::Write + Send> = Box::new(io::sink());
        let terminal = Terminal::new(
            wezterm_terminal_size(size),
            config,
            "agent-tui",
            env!("CARGO_PKG_VERSION"),
            writer,
        );
        Self { terminal, size }
    }

    pub fn process(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        self.terminal.advance_bytes(data);
    }

    pub fn screen_text(&self) -> String {
        let buffer = self.screen_buffer();
        let Some(trimmed) = buffer.trimmed_rows() else {
            return String::new();
        };

        let mut output = String::with_capacity(
            trimmed.total_trimmed_chars + trimmed.rows.len().saturating_sub(1),
        );

        for (row_idx, row) in trimmed.rows.iter().enumerate() {
            if row_idx > 0 {
                output.push('\n');
            }
            for cell in row.iter() {
                output.push(cell.char);
            }
        }

        output
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

    pub fn resize(&mut self, size: TerminalSize) {
        self.terminal.resize(wezterm_terminal_size(size));
        self.size = size;
    }

    pub fn size(&self) -> TerminalSize {
        self.size
    }
}

fn wezterm_terminal_size(size: TerminalSize) -> WeztermTerminalSize {
    WeztermTerminalSize {
        rows: size.rows() as usize,
        cols: size.cols() as usize,
        pixel_width: 0,
        pixel_height: 0,
        dpi: 0,
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
#[path = "vterm_tests.rs"]
mod tests;
