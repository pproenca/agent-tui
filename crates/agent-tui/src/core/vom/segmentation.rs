use super::Cluster;
use crate::core::screen::ScreenGrid;

pub fn segment_buffer(buffer: &impl ScreenGrid) -> Vec<Cluster> {
    let mut clusters = Vec::new();

    for y in 0..buffer.rows() {
        let mut current: Option<Cluster> = None;

        for x in 0..buffer.cols() {
            if let Some((char, style)) = buffer.cell(y, x) {
                let style_match = current.as_ref().map(|c| c.style == style).unwrap_or(false);

                if style_match {
                    if let Some(c) = &mut current {
                        c.extend(char);
                    }
                } else {
                    if let Some(mut c) = current.take() {
                        c.seal();
                        clusters.push(c);
                    }

                    current = Some(Cluster::new(x as u16, y as u16, char, style));
                }
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
    use crate::core::style::CellStyle;
    use crate::core::style::Color;

    #[derive(Debug, Clone)]
    struct Cell {
        char: char,
        style: CellStyle,
    }

    #[derive(Debug)]
    struct MockScreenBuffer {
        cells: Vec<Vec<Cell>>,
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

    fn make_buffer(cells: Vec<Vec<Cell>>) -> MockScreenBuffer {
        MockScreenBuffer { cells }
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
            make_cell('[', false, bg),
            make_cell(' ', false, bg),
            make_cell('O', false, bg),
            make_cell('K', false, bg),
            make_cell(' ', false, bg),
            make_cell(']', false, bg),
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

    mod prop_tests {
        use super::*;
        use proptest::prelude::*;

        fn arb_color() -> impl Strategy<Value = Option<Color>> {
            prop_oneof![Just(None), (0u8..16).prop_map(|i| Some(Color::Indexed(i))),]
        }

        fn arb_cell() -> impl Strategy<Value = Cell> {
            (any::<char>(), any::<bool>(), arb_color()).prop_map(|(char, bold, bg)| Cell {
                char: if char.is_control() { ' ' } else { char },
                style: CellStyle {
                    bold,
                    underline: false,
                    inverse: false,
                    fg_color: None,
                    bg_color: bg,
                },
            })
        }

        fn arb_buffer(max_rows: usize, max_cols: usize) -> impl Strategy<Value = MockScreenBuffer> {
            prop::collection::vec(
                prop::collection::vec(arb_cell(), 1..=max_cols),
                1..=max_rows,
            )
            .prop_map(|rows| {
                let max_width = rows.iter().map(|r| r.len()).max().unwrap_or(0);
                let normalized: Vec<Vec<Cell>> = rows
                    .into_iter()
                    .map(|mut row| {
                        row.resize(
                            max_width,
                            Cell {
                                char: ' ',
                                style: CellStyle::default(),
                            },
                        );
                        row
                    })
                    .collect();
                MockScreenBuffer { cells: normalized }
            })
        }

        proptest! {
            #[test]
            fn clusters_never_overlap(buffer in arb_buffer(10, 40)) {
                let clusters = segment_buffer(&buffer);

                for (i, a) in clusters.iter().enumerate() {
                    for b in clusters.iter().skip(i + 1) {
                        if a.rect.y == b.rect.y {
                            let a_end = a.rect.x + a.rect.width;
                            let b_end = b.rect.x + b.rect.width;
                            let no_overlap = a_end <= b.rect.x || b_end <= a.rect.x;
                            prop_assert!(
                                no_overlap,
                                "Clusters overlap on row {}: [{}, {}) vs [{}, {})",
                                a.rect.y, a.rect.x, a_end, b.rect.x, b_end
                            );
                        }
                    }
                }
            }

            #[test]
            fn cluster_bounds_within_buffer(buffer in arb_buffer(10, 40)) {
                let clusters = segment_buffer(&buffer);
                let rows = buffer.rows() as u16;
                let cols = buffer.cols() as u16;

                for cluster in &clusters {
                    prop_assert!(
                        cluster.rect.y < rows,
                        "Cluster y {} >= rows {}",
                        cluster.rect.y, rows
                    );
                    prop_assert!(
                        cluster.rect.x + cluster.rect.width <= cols,
                        "Cluster extends past buffer: x={} width={} cols={}",
                        cluster.rect.x, cluster.rect.width, cols
                    );
                }
            }

            #[test]
            fn segmentation_is_deterministic(buffer in arb_buffer(5, 20)) {
                let clusters1 = segment_buffer(&buffer);
                let clusters2 = segment_buffer(&buffer);

                prop_assert_eq!(clusters1.len(), clusters2.len());
                for (a, b) in clusters1.iter().zip(clusters2.iter()) {
                    prop_assert_eq!(a.rect, b.rect);
                    prop_assert_eq!(&a.text, &b.text);
                    prop_assert_eq!(&a.style, &b.style);
                }
            }
        }
    }
}
