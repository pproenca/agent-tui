//! Terminal rendering helpers.

use std::io::Write;

use crossterm::queue;
use crossterm::style;
use tracing::debug;

use super::CellStyle;
use super::Color;
use super::ScreenBuffer;

pub fn render_screen(buffer: &ScreenBuffer) -> String {
    if buffer.cells.is_empty() {
        return String::new();
    }

    let mut out = Vec::new();
    let mut current_style: Option<&CellStyle> = None;

    for (row_idx, row) in buffer.cells.iter().enumerate() {
        let mut col = 0;
        while col < row.len() {
            let style = &row[col].style;
            let mut run_end = col + 1;
            while run_end < row.len() && row[run_end].style == *style {
                run_end += 1;
            }

            if current_style != Some(style) {
                if let Err(err) = apply_style(&mut out, style) {
                    debug!(error = %err, "Failed to apply terminal style");
                }
                current_style = Some(style);
            }

            let mut text = String::with_capacity(run_end - col);
            for cell in &row[col..run_end] {
                text.push(cell.char);
            }
            if let Err(err) = queue!(out, style::Print(text)) {
                debug!(error = %err, "Failed to write terminal text");
            }
            col = run_end;
        }

        if row_idx + 1 < buffer.cells.len()
            && let Err(err) = queue!(out, style::Print("\r\n"))
        {
            debug!(error = %err, "Failed to write terminal newline");
        }
    }

    String::from_utf8(out).unwrap_or_else(|err| {
        debug!(error = %err, "Failed to decode terminal output as UTF-8");
        String::new()
    })
}

fn apply_style(out: &mut impl Write, style: &CellStyle) -> std::io::Result<()> {
    queue!(out, style::SetAttribute(style::Attribute::Reset))?;

    if style.bold {
        queue!(out, style::SetAttribute(style::Attribute::Bold))?;
    }
    if style.underline {
        queue!(out, style::SetAttribute(style::Attribute::Underlined))?;
    }
    if style.inverse {
        queue!(out, style::SetAttribute(style::Attribute::Reverse))?;
    }

    let fg = style.fg_color.unwrap_or(Color::Default);
    let bg = style.bg_color.unwrap_or(Color::Default);

    queue!(out, style::SetForegroundColor(to_crossterm_color(fg)))?;
    queue!(out, style::SetBackgroundColor(to_crossterm_color(bg)))?;

    Ok(())
}

fn to_crossterm_color(color: Color) -> style::Color {
    match color {
        Color::Default => style::Color::Reset,
        Color::Indexed(idx) => style::Color::AnsiValue(idx),
        Color::Rgb(r, g, b) => style::Color::Rgb { r, g, b },
    }
}
