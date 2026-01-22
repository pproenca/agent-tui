use crate::terminal::ScreenBuffer;
use crate::vom::Cluster;

pub fn segment_buffer(buffer: &ScreenBuffer) -> Vec<Cluster> {
    let mut clusters = Vec::new();

    for (y, row) in buffer.cells.iter().enumerate() {
        let mut current: Option<Cluster> = None;

        for (x, cell) in row.iter().enumerate() {
            let style_match = current
                .as_ref()
                .map(|c| c.style == cell.style)
                .unwrap_or(false);

            if style_match {
                if let Some(c) = &mut current {
                    c.extend(cell.char);
                }
            } else {
                if let Some(mut c) = current.take() {
                    c.seal();
                    clusters.push(c);
                }

                current = Some(Cluster::new(
                    x as u16,
                    y as u16,
                    cell.char,
                    cell.style.clone(),
                ));
            }
        }

        if let Some(mut c) = current {
            c.seal();
            clusters.push(c);
        }
    }

    clusters.into_iter().filter(|c| !c.is_whitespace).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::{Cell, CellStyle, Color};

    fn make_buffer(cells: Vec<Vec<Cell>>) -> ScreenBuffer {
        ScreenBuffer { cells }
    }

    fn make_cell(char: char, bold: bool, bg: Option<Color>) -> Cell {
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

    #[test]
    fn test_single_style_row() {
        let cells = vec![vec![
            make_cell('H', false, None),
            make_cell('e', false, None),
            make_cell('l', false, None),
            make_cell('l', false, None),
            make_cell('o', false, None),
        ]];
        let buffer = make_buffer(cells);
        let clusters = segment_buffer(&buffer);

        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].text, "Hello");
        assert_eq!(clusters[0].rect.x, 0);
        assert_eq!(clusters[0].rect.width, 5);
    }

    #[test]
    fn test_style_transition() {
        let cells = vec![vec![
            make_cell('H', false, None),
            make_cell('i', false, None),
            make_cell('!', true, None),
        ]];
        let buffer = make_buffer(cells);
        let clusters = segment_buffer(&buffer);

        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].text, "Hi");
        assert_eq!(clusters[1].text, "!");
        assert!(clusters[1].style.bold);
    }

    #[test]
    fn test_whitespace_filtering() {
        let cells = vec![vec![
            make_cell('H', false, None),
            make_cell('i', false, None),
            make_cell(' ', true, None),
            make_cell(' ', true, None),
        ]];
        let buffer = make_buffer(cells);
        let clusters = segment_buffer(&buffer);

        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].text, "Hi");
    }

    #[test]
    fn test_multi_row() {
        let cells = vec![
            vec![make_cell('A', false, None), make_cell('B', false, None)],
            vec![make_cell('C', true, None), make_cell('D', true, None)],
        ];
        let buffer = make_buffer(cells);
        let clusters = segment_buffer(&buffer);

        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].text, "AB");
        assert_eq!(clusters[0].rect.y, 0);
        assert_eq!(clusters[1].text, "CD");
        assert_eq!(clusters[1].rect.y, 1);
    }

    #[test]
    fn test_color_transition() {
        let cells = vec![vec![
            make_cell('H', false, Some(Color::Indexed(1))),
            make_cell('i', false, Some(Color::Indexed(2))),
        ]];
        let buffer = make_buffer(cells);
        let clusters = segment_buffer(&buffer);

        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].text, "H");
        assert_eq!(clusters[1].text, "i");
    }

    #[test]
    fn test_button_like_pattern() {
        let bg = Some(Color::Indexed(4));
        let cells = vec![vec![
            make_cell('[', false, bg.clone()),
            make_cell(' ', false, bg.clone()),
            make_cell('O', false, bg.clone()),
            make_cell('K', false, bg.clone()),
            make_cell(' ', false, bg.clone()),
            make_cell(']', false, bg.clone()),
        ]];
        let buffer = make_buffer(cells);
        let clusters = segment_buffer(&buffer);

        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].text, "[ OK ]");
    }

    #[test]
    fn test_empty_buffer() {
        let buffer = make_buffer(vec![]);
        let clusters = segment_buffer(&buffer);
        assert_eq!(clusters.len(), 0);
    }

    #[test]
    fn test_empty_row() {
        let buffer = make_buffer(vec![vec![]]);
        let clusters = segment_buffer(&buffer);
        assert_eq!(clusters.len(), 0);
    }

    #[test]
    fn test_single_cell() {
        let cells = vec![vec![make_cell('X', false, None)]];
        let buffer = make_buffer(cells);
        let clusters = segment_buffer(&buffer);

        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].text, "X");
        assert_eq!(clusters[0].rect.width, 1);
    }
}
