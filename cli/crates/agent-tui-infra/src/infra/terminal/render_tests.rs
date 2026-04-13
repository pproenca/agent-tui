use super::*;
use crate::common::strip_ansi_codes;
use insta::assert_snapshot;

fn plain_cell(ch: char) -> Cell {
    Cell {
        char: ch,
        style: CellStyle::default(),
    }
}

#[test]
fn render_screen_trimmed_drops_trailing_spaces_and_blank_rows() {
    let buffer = ScreenBuffer {
        cells: vec![
            vec![plain_cell('A'), plain_cell(' '), plain_cell(' ')],
            vec![plain_cell(' ')],
            vec![plain_cell('B'), plain_cell(' '), plain_cell(' ')],
            vec![plain_cell(' ')],
            vec![plain_cell(' '), plain_cell(' ')],
        ],
    };

    let rendered = render_screen_trimmed(&buffer);

    assert_eq!(strip_ansi_codes(&rendered).replace("\r\n", "\n"), "A\n\nB");
}

#[test]
fn render_screen_snapshot_captures_ansi_runs_and_line_breaks() {
    let accent = CellStyle {
        bold: true,
        underline: true,
        fg_color: Some(Color::Indexed(1)),
        bg_color: Some(Color::Rgb(16, 32, 48)),
        ..CellStyle::default()
    };
    let inverse = CellStyle {
        inverse: true,
        ..CellStyle::default()
    };
    let buffer = ScreenBuffer {
        cells: vec![
            vec![
                plain_cell('A'),
                Cell {
                    char: 'B',
                    style: accent,
                },
                Cell {
                    char: 'C',
                    style: accent,
                },
            ],
            vec![
                Cell {
                    char: 'D',
                    style: inverse,
                },
                plain_cell(' '),
                Cell {
                    char: 'E',
                    style: accent,
                },
            ],
        ],
    };

    let rendered = render_screen(&buffer);
    assert_snapshot!(
        "render_screen_mixed_styles",
        rendered.escape_debug().to_string()
    );
}

#[test]
fn render_screen_trimmed_snapshot_preserves_internal_blank_rows() {
    let buffer = ScreenBuffer {
        cells: vec![
            vec![plain_cell('A'), plain_cell(' '), plain_cell(' ')],
            vec![plain_cell(' ')],
            vec![plain_cell('B'), plain_cell(' '), plain_cell(' ')],
            vec![plain_cell(' ')],
            vec![plain_cell(' '), plain_cell(' ')],
        ],
    };

    let rendered = render_screen_trimmed(&buffer);
    assert_snapshot!(
        "render_screen_trimmed_internal_blank_rows",
        rendered.escape_debug().to_string()
    );
}

#[test]
fn render_screen_trimmed_resets_styles_at_end() {
    let buffer = ScreenBuffer {
        cells: vec![vec![Cell {
            char: 'X',
            style: CellStyle {
                bold: true,
                ..CellStyle::default()
            },
        }]],
    };

    let rendered = render_screen_trimmed(&buffer);

    assert!(
        rendered.ends_with("\u{1b}[0m"),
        "expected trailing style reset, got {rendered:?}"
    );
}
