//! Segmentation Pass ("Lexer")
//!
//! Algorithm: Raster Scan with Run-Length Encoding
//! Goal: Compress 1920 cells (80x24) into ~50-100 Clusters
//!
//! A Cluster is a contiguous sequence of cells on a single row
//! that share IDENTICAL styling (same fg, bg, bold, underline, inverse).

use crate::terminal::ScreenBuffer;
use crate::vom::Cluster;

/// Segment a screen buffer into clusters based on style transitions.
///
/// This is the "lexer" phase of the VOM pipeline. It performs a single
/// raster scan (row by row, left to right) and groups adjacent cells
/// with identical styles into Clusters.
///
/// # Algorithm
/// - For each row, iterate through cells
/// - If current cell's style matches previous cell's style, extend the current cluster
/// - If styles differ, seal the current cluster and start a new one
/// - After each row, seal the final cluster
/// - Filter out whitespace-only clusters (noise reduction)
///
/// # Performance
/// - O(N) where N = total cells (typically 80*24 = 1920)
/// - Single pass, no backtracking
/// - Typically produces 50-100 clusters from a full screen
pub fn segment_buffer(buffer: &ScreenBuffer) -> Vec<Cluster> {
    let mut clusters = Vec::new();

    for (y, row) in buffer.cells.iter().enumerate() {
        let mut current: Option<Cluster> = None;

        for (x, cell) in row.iter().enumerate() {
            // Compare current cell's style vs previous cell's style
            let style_match = current
                .as_ref()
                .map(|c| c.style == cell.style)
                .unwrap_or(false);

            if style_match {
                // Match: Extend current Cluster
                if let Some(c) = &mut current {
                    c.extend(cell.char);
                }
            } else {
                // Mismatch: Seal current Cluster, start new one
                if let Some(mut c) = current.take() {
                    c.seal();
                    clusters.push(c);
                }

                // Start new cluster
                current = Some(Cluster::new(
                    x as u16,
                    y as u16,
                    cell.char,
                    cell.style.clone(),
                ));
            }
        }

        // End of row: seal final cluster
        if let Some(mut c) = current {
            c.seal();
            clusters.push(c);
        }
    }

    // Filter out pure whitespace clusters (noise reduction)
    // This dramatically reduces the number of clusters while preserving
    // all semantically meaningful content
    clusters.into_iter().filter(|c| !c.is_whitespace).collect()
}

/// Segment without filtering whitespace (for debugging/testing)
pub fn segment_buffer_with_whitespace(buffer: &ScreenBuffer) -> Vec<Cluster> {
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

    clusters
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
        // "Hello" with default style
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
        // "Hi" normal + "!" bold
        let cells = vec![vec![
            make_cell('H', false, None),
            make_cell('i', false, None),
            make_cell('!', true, None), // bold
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
        // "Hi" followed by spaces with DIFFERENT style (bold spaces)
        // This creates two clusters: "Hi" (normal) and "  " (bold whitespace)
        // The whitespace cluster should be filtered out
        let cells = vec![vec![
            make_cell('H', false, None),
            make_cell('i', false, None),
            make_cell(' ', true, None), // bold - different style
            make_cell(' ', true, None), // bold - different style
        ]];
        let buffer = make_buffer(cells);
        let clusters = segment_buffer(&buffer);

        // Whitespace cluster should be filtered out, leaving just "Hi"
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
        // "Hi" with different background colors
        let cells = vec![vec![
            make_cell('H', false, Some(Color::Indexed(1))), // Red bg
            make_cell('i', false, Some(Color::Indexed(2))), // Green bg
        ]];
        let buffer = make_buffer(cells);
        let clusters = segment_buffer(&buffer);

        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].text, "H");
        assert_eq!(clusters[1].text, "i");
    }

    #[test]
    fn test_button_like_pattern() {
        // [ OK ] pattern - spaces before/after OK should be same cluster if same style
        let bg = Some(Color::Indexed(4)); // Blue
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

        // All same style, so should be one cluster
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].text, "[ OK ]");
    }
}
