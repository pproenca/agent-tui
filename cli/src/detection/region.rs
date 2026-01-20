//! Region and modal detection for TUI applications
//!
//! Detects box-drawing character boundaries to identify:
//! - Modal dialogs
//! - Panels
//! - Windows
//! - Bordered regions

/// A detected region/box in the terminal
#[derive(Debug, Clone)]
pub struct Region {
    /// Top-left row (0-indexed)
    pub top: u16,
    /// Top-left column (0-indexed)
    pub left: u16,
    /// Bottom-right row (0-indexed)
    pub bottom: u16,
    /// Bottom-right column (0-indexed)
    pub right: u16,
    /// The border style used
    pub border_style: BorderStyle,
    /// Optional title extracted from the border
    pub title: Option<String>,
}

impl Region {
    /// Get the width of the region
    pub fn width(&self) -> u16 {
        self.right.saturating_sub(self.left) + 1
    }

    /// Get the height of the region
    pub fn height(&self) -> u16 {
        self.bottom.saturating_sub(self.top) + 1
    }

    /// Check if this region is likely a modal (centered, not full-width)
    pub fn is_modal(&self, screen_cols: u16, _screen_rows: u16) -> bool {
        let width = self.width();
        let height = self.height();

        // Modal criteria:
        // - Not full width
        // - Not at edges
        // - Reasonable size
        let not_full_width = width < screen_cols - 4;
        let not_at_left_edge = self.left > 2;
        let not_at_top_edge = self.top > 0;
        let reasonable_size = width > 10 && height > 3;

        not_full_width && not_at_left_edge && not_at_top_edge && reasonable_size
    }

    /// Check if a position is inside this region
    pub fn contains(&self, row: u16, col: u16) -> bool {
        row >= self.top && row <= self.bottom && col >= self.left && col <= self.right
    }

    /// Check if a position is inside the content area (excluding border)
    pub fn contains_content(&self, row: u16, col: u16) -> bool {
        row > self.top && row < self.bottom && col > self.left && col < self.right
    }
}

/// Border styles for box drawing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    /// Single line: ┌ ─ ┐ │ └ ─ ┘
    Single,
    /// Rounded corners: ╭ ─ ╮ │ ╰ ─ ╯
    Rounded,
    /// Double line: ╔ ═ ╗ ║ ╚ ═ ╝
    Double,
    /// Heavy/thick line: ┏ ━ ┓ ┃ ┗ ━ ┛
    Heavy,
    /// ASCII: + - + | + - +
    Ascii,
    /// Unknown/mixed style
    Unknown,
}

impl BorderStyle {
    fn top_left(&self) -> &[char] {
        match self {
            BorderStyle::Single => &['┌'],
            BorderStyle::Rounded => &['╭'],
            BorderStyle::Double => &['╔'],
            BorderStyle::Heavy => &['┏'],
            BorderStyle::Ascii => &['+'],
            BorderStyle::Unknown => &['┌', '╭', '╔', '┏', '+'],
        }
    }

    fn top_right(&self) -> &[char] {
        match self {
            BorderStyle::Single => &['┐'],
            BorderStyle::Rounded => &['╮'],
            BorderStyle::Double => &['╗'],
            BorderStyle::Heavy => &['┓'],
            BorderStyle::Ascii => &['+'],
            BorderStyle::Unknown => &['┐', '╮', '╗', '┓', '+'],
        }
    }

    fn bottom_left(&self) -> &[char] {
        match self {
            BorderStyle::Single => &['└'],
            BorderStyle::Rounded => &['╰'],
            BorderStyle::Double => &['╚'],
            BorderStyle::Heavy => &['┗'],
            BorderStyle::Ascii => &['+'],
            BorderStyle::Unknown => &['└', '╰', '╚', '┗', '+'],
        }
    }

    fn bottom_right(&self) -> &[char] {
        match self {
            BorderStyle::Single => &['┘'],
            BorderStyle::Rounded => &['╯'],
            BorderStyle::Double => &['╝'],
            BorderStyle::Heavy => &['┛'],
            BorderStyle::Ascii => &['+'],
            BorderStyle::Unknown => &['┘', '╯', '╝', '┛', '+'],
        }
    }

    fn horizontal(&self) -> &[char] {
        match self {
            BorderStyle::Single => &['─'],
            BorderStyle::Rounded => &['─'],
            BorderStyle::Double => &['═'],
            BorderStyle::Heavy => &['━'],
            BorderStyle::Ascii => &['-'],
            BorderStyle::Unknown => &['─', '═', '━', '-'],
        }
    }

    fn vertical(&self) -> &[char] {
        match self {
            BorderStyle::Single => &['│'],
            BorderStyle::Rounded => &['│'],
            BorderStyle::Double => &['║'],
            BorderStyle::Heavy => &['┃'],
            BorderStyle::Ascii => &['|'],
            BorderStyle::Unknown => &['│', '║', '┃', '|'],
        }
    }
}

/// All top-left corner characters
const TOP_LEFT_CORNERS: [char; 5] = ['┌', '╭', '╔', '┏', '+'];

/// All horizontal line characters
const HORIZONTAL_CHARS: [char; 4] = ['─', '═', '━', '-'];

/// All vertical line characters
const VERTICAL_CHARS: [char; 4] = ['│', '║', '┃', '|'];

/// Detect regions (boxes) in the screen
pub fn detect_regions(screen: &str) -> Vec<Region> {
    let lines: Vec<Vec<char>> = screen.lines().map(|l| l.chars().collect()).collect();
    let mut regions = Vec::new();

    if lines.is_empty() {
        return regions;
    }

    // Scan for top-left corners
    for (row_idx, row) in lines.iter().enumerate() {
        for (col_idx, &ch) in row.iter().enumerate() {
            if TOP_LEFT_CORNERS.contains(&ch) {
                // Found a potential top-left corner, try to find the complete box
                if let Some(region) = trace_box(&lines, row_idx, col_idx) {
                    // Check if this region is not a duplicate or nested inside another
                    let dominated = regions.iter().any(|r: &Region| {
                        r.top <= region.top
                            && r.left <= region.left
                            && r.bottom >= region.bottom
                            && r.right >= region.right
                            && !(r.top == region.top
                                && r.left == region.left
                                && r.bottom == region.bottom
                                && r.right == region.right)
                    });

                    if !dominated {
                        // Remove any existing regions that are dominated by this one
                        regions.retain(|r: &Region| {
                            !(region.top <= r.top
                                && region.left <= r.left
                                && region.bottom >= r.bottom
                                && region.right >= r.right)
                        });
                        regions.push(region);
                    }
                }
            }
        }
    }

    // Sort by size (larger first, likely to be more important)
    regions.sort_by(|a, b| {
        let area_a = a.width() as u32 * a.height() as u32;
        let area_b = b.width() as u32 * b.height() as u32;
        area_b.cmp(&area_a)
    });

    regions
}

/// Trace a complete box starting from a top-left corner
fn trace_box(lines: &[Vec<char>], start_row: usize, start_col: usize) -> Option<Region> {
    let top_left = lines[start_row][start_col];
    let border_style = detect_border_style(top_left);

    // Find the top-right corner by following horizontal lines
    let mut top_right_col = None;
    let start_row_chars = &lines[start_row];
    for (col, &ch) in start_row_chars.iter().enumerate().skip(start_col + 1) {
        if border_style.top_right().contains(&ch) {
            top_right_col = Some(col);
            break;
        } else if !border_style.horizontal().contains(&ch) && ch != ' ' {
            // Allow spaces in titles
            if !ch.is_alphanumeric() && ch != ' ' && ch != ':' && ch != '-' {
                break;
            }
        }
    }

    let right_col = top_right_col?;

    // Find the bottom-left corner by following vertical lines
    let mut bottom_row = None;
    for (row, row_chars) in lines.iter().enumerate().skip(start_row + 1) {
        if start_col >= row_chars.len() {
            break;
        }
        let ch = row_chars[start_col];
        if border_style.bottom_left().contains(&ch) {
            // Verify bottom-right corner exists
            if right_col < row_chars.len() {
                let br = row_chars[right_col];
                if border_style.bottom_right().contains(&br) {
                    bottom_row = Some(row);
                    break;
                }
            }
        } else if !border_style.vertical().contains(&ch) {
            break;
        }
    }

    let bottom = bottom_row?;

    // Verify the box has complete sides
    // Check right side has vertical lines
    for row_chars in lines.iter().take(bottom).skip(start_row + 1) {
        if right_col >= row_chars.len() {
            return None;
        }
        let ch = row_chars[right_col];
        if !border_style.vertical().contains(&ch) {
            return None;
        }
    }

    // Check bottom side has horizontal lines
    for col in (start_col + 1)..right_col {
        if col >= lines[bottom].len() {
            return None;
        }
        let ch = lines[bottom][col];
        if !border_style.horizontal().contains(&ch) && ch != ' ' {
            return None;
        }
    }

    // Extract title from top border if present
    let title = extract_title(&lines[start_row], start_col, right_col);

    Some(Region {
        top: start_row as u16,
        left: start_col as u16,
        bottom: bottom as u16,
        right: right_col as u16,
        border_style,
        title,
    })
}

/// Detect the border style from a top-left corner character
fn detect_border_style(corner: char) -> BorderStyle {
    match corner {
        '┌' => BorderStyle::Single,
        '╭' => BorderStyle::Rounded,
        '╔' => BorderStyle::Double,
        '┏' => BorderStyle::Heavy,
        '+' => BorderStyle::Ascii,
        _ => BorderStyle::Unknown,
    }
}

/// Extract a title from the top border
fn extract_title(line: &[char], left: usize, right: usize) -> Option<String> {
    if right <= left + 2 {
        return None;
    }

    // Look for text between the corners
    let content: String = line[(left + 1)..right].iter().collect();

    // Remove border characters and trim
    let title: String = content
        .chars()
        .filter(|c| !HORIZONTAL_CHARS.contains(c))
        .collect();

    let trimmed = title.trim();
    if trimmed.is_empty() || trimmed.len() < 2 {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Find the innermost region containing a point
pub fn find_region_at(regions: &[Region], row: u16, col: u16) -> Option<&Region> {
    regions
        .iter()
        .filter(|r| r.contains_content(row, col))
        .min_by_key(|r| r.width() as u32 * r.height() as u32)
}

/// Find modal dialogs (regions that appear to be overlay dialogs)
pub fn find_modals(regions: &[Region], screen_cols: u16, screen_rows: u16) -> Vec<&Region> {
    regions
        .iter()
        .filter(|r| r.is_modal(screen_cols, screen_rows))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_single_box() {
        let screen = "┌────────┐\n│  Test  │\n└────────┘";
        let regions = detect_regions(screen);

        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].top, 0);
        assert_eq!(regions[0].left, 0);
        assert_eq!(regions[0].bottom, 2);
        assert_eq!(regions[0].right, 9);
        assert_eq!(regions[0].border_style, BorderStyle::Single);
    }

    #[test]
    fn test_detect_rounded_box() {
        let screen = "╭──────╮\n│ Modal│\n╰──────╯";
        let regions = detect_regions(screen);

        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].border_style, BorderStyle::Rounded);
    }

    #[test]
    fn test_detect_double_box() {
        let screen = "╔════════╗\n║ Dialog ║\n╚════════╝";
        let regions = detect_regions(screen);

        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].border_style, BorderStyle::Double);
    }

    #[test]
    fn test_detect_ascii_box() {
        let screen = "+--------+\n| Text   |\n+--------+";
        let regions = detect_regions(screen);

        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].border_style, BorderStyle::Ascii);
    }

    #[test]
    fn test_extract_title() {
        let screen = "┌─ Title ─┐\n│ Content │\n└─────────┘";
        let regions = detect_regions(screen);

        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].title, Some("Title".to_string()));
    }

    #[test]
    fn test_region_contains() {
        let region = Region {
            top: 5,
            left: 10,
            bottom: 15,
            right: 50,
            border_style: BorderStyle::Single,
            title: None,
        };

        assert!(region.contains(5, 10)); // top-left corner
        assert!(region.contains(15, 50)); // bottom-right corner
        assert!(region.contains(10, 30)); // middle
        assert!(!region.contains(4, 10)); // above
        assert!(!region.contains(10, 9)); // left of
    }

    #[test]
    fn test_is_modal() {
        let modal = Region {
            top: 5,
            left: 20,
            bottom: 15,
            right: 60,
            border_style: BorderStyle::Rounded,
            title: Some("Confirm".to_string()),
        };

        assert!(modal.is_modal(80, 24));

        let fullwidth = Region {
            top: 0,
            left: 0,
            bottom: 23,
            right: 79,
            border_style: BorderStyle::Single,
            title: None,
        };

        assert!(!fullwidth.is_modal(80, 24));
    }
}
