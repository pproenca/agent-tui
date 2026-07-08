//! Terminal rendering helpers.

use std::io::Write;

use tracing::debug;

use super::CellStyle;
use super::Color;
use super::ScreenBuffer;
use super::vterm::Cell;

pub fn render_screen(buffer: &ScreenBuffer) -> String {
    render_rows(buffer.cells.iter().map(Vec::as_slice))
}

pub fn render_screen_trimmed(buffer: &ScreenBuffer) -> String {
    let Some(trimmed) = buffer.trimmed_rows() else {
        return String::new();
    };

    render_rows(trimmed.rows)
}

fn render_rows<'a>(rows: impl IntoIterator<Item = &'a [Cell]>) -> String {
    let mut rows = rows.into_iter().peekable();
    if rows.peek().is_none() {
        return String::new();
    }

    let mut out = Vec::new();
    let mut current_style: Option<&CellStyle> = None;

    while let Some(row) = rows.next() {
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
            if let Err(err) = out.write_all(text.as_bytes()) {
                debug!(error = %err, "Failed to write terminal text");
            }
            col = run_end;
        }

        if rows.peek().is_some()
            && let Err(err) = out.write_all(b"\r\n")
        {
            debug!(error = %err, "Failed to write terminal newline");
        }
    }

    if current_style.is_some()
        && let Err(err) = write_sgr(&mut out, "0")
    {
        debug!(error = %err, "Failed to reset terminal style");
    }

    String::from_utf8(out).unwrap_or_else(|err| {
        debug!(error = %err, "Failed to decode terminal output as UTF-8");
        String::new()
    })
}

fn apply_style(out: &mut impl Write, style: &CellStyle) -> std::io::Result<()> {
    write_sgr(out, "0")?;

    if style.bold {
        write_sgr(out, "1")?;
    }
    if style.underline {
        write_sgr(out, "4")?;
    }
    if style.inverse {
        write_sgr(out, "7")?;
    }

    if let Some(fg) = style.fg_color.filter(|color| *color != Color::Default) {
        write_color_sgr(out, 38, fg)?;
    }
    if let Some(bg) = style.bg_color.filter(|color| *color != Color::Default) {
        write_color_sgr(out, 48, bg)?;
    }

    Ok(())
}

fn write_sgr(out: &mut impl Write, params: &str) -> std::io::Result<()> {
    write!(out, "\x1b[{params}m")
}

fn write_color_sgr(out: &mut impl Write, prefix: u8, color: Color) -> std::io::Result<()> {
    match color {
        Color::Default => Ok(()),
        Color::Indexed(idx) => write_sgr(out, &format!("{prefix};5;{idx}")),
        Color::Rgb(r, g, b) => write_sgr(out, &format!("{prefix};2;{r};{g};{b}")),
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
