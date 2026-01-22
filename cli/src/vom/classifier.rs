//! Classification Pass ("Parser")
//!
//! Algorithm: Geometric & Attribute Heuristics
//! Goal: Promote Clusters into Components with assigned Roles
//!
//! The classifier uses deterministic rules based on:
//! - Cursor position (focus detection)
//! - Text patterns (brackets, markers)
//! - Style attributes (inverse, colors)
//! - Geometric properties (position, size)

use crate::terminal::Color;
use crate::vom::{hash_cluster, Cluster, Component, Role};

/// Classify clusters into semantic components.
///
/// Takes the output of the segmentation phase and promotes each cluster
/// into a Component with an assigned Role based on heuristics.
///
/// # Arguments
/// - `clusters`: The clusters from segmentation
/// - `cursor_row`: Current cursor row position
/// - `cursor_col`: Current cursor column position
pub fn classify(clusters: Vec<Cluster>, cursor_row: u16, cursor_col: u16) -> Vec<Component> {
    clusters
        .into_iter()
        .map(|cluster| {
            let role = infer_role(&cluster, cursor_row, cursor_col);
            let visual_hash = hash_cluster(&cluster);

            Component::new(role, cluster.rect, cluster.text.clone(), visual_hash)
        })
        .collect()
}

/// Infer the semantic role of a cluster.
///
/// Rules are evaluated in priority order (first match wins):
/// 1. Focus Rule: Cursor intersects cluster → Input
/// 2. Button Rule: Bracketed text [Label] → Button
/// 3. Tab Rule: Inverse video or Blue/Cyan background → Tab
/// 4. Input Rule: Contains underscores → Input
/// 5. Checkbox Rule: Checkbox patterns → Checkbox
/// 6. MenuItem Rule: Menu markers → MenuItem
/// 7. Panel Rule: Box-drawing characters → Panel
///
/// If no rule matches, defaults to StaticText.
fn infer_role(cluster: &Cluster, cursor_row: u16, cursor_col: u16) -> Role {
    let text = cluster.text.trim();

    // RULE 1: Focus Rule
    // Cluster intersects with cursor position = focused/input
    if cluster.rect.y == cursor_row
        && cursor_col >= cluster.rect.x
        && cursor_col < cluster.rect.x + cluster.rect.width
    {
        return Role::Input;
    }

    // RULE 2: Button Rule
    // Bracketed text [Label] with meaningful content
    if is_button_text(text) {
        return Role::Button;
    }

    // RULE 3: Tab Rule - Multiple checks
    // 3a: Inverse video = selected/active (common for tabs and menu items)
    if cluster.style.inverse {
        // If it's on the top few rows, likely a tab
        if cluster.rect.y <= 2 {
            return Role::Tab;
        }
        // Otherwise could be selected menu item
        return Role::MenuItem;
    }

    // 3b: Blue or Cyan background (common for tabs)
    if let Some(Color::Indexed(idx)) = &cluster.style.bg_color {
        // 4 = Blue, 6 = Cyan in standard 16-color palette
        if *idx == 4 || *idx == 6 {
            return Role::Tab;
        }
    }

    // RULE 4: Input Rule
    // Contains underscores (placeholder for input) or is clearly an input field
    if is_input_field(text) {
        return Role::Input;
    }

    // RULE 5: Checkbox Rule
    if is_checkbox(text) {
        return Role::Checkbox;
    }

    // RULE 6: MenuItem Rule
    // Menu markers (selection indicators)
    if is_menu_item(text) {
        return Role::MenuItem;
    }

    // RULE 7: Panel detection (box-drawing characters)
    if is_panel_border(text) {
        return Role::Panel;
    }

    // Default: Static text
    Role::StaticText
}

/// Check if text represents a button
fn is_button_text(text: &str) -> bool {
    // [Label] pattern with meaningful content
    if text.starts_with('[') && text.ends_with(']') && text.len() > 2 {
        let inner = &text[1..text.len() - 1].trim();
        // Exclude checkbox-like patterns
        if !matches!(*inner, "x" | "X" | " " | "" | "✓" | "✔") {
            return true;
        }
    }

    // (Label) pattern
    if text.starts_with('(') && text.ends_with(')') && text.len() > 2 {
        let inner = &text[1..text.len() - 1].trim();
        // Exclude radio button patterns
        if !matches!(*inner, "" | " " | "o" | "O" | "●" | "◉") {
            return true;
        }
    }

    // <Label> pattern (HTML-like buttons)
    if text.starts_with('<') && text.ends_with('>') && text.len() > 2 {
        return true;
    }

    false
}

/// Check if text represents an input field
fn is_input_field(text: &str) -> bool {
    // Multiple underscores = placeholder for input
    if text.contains("___") {
        return true;
    }

    // All underscores
    if !text.is_empty() && text.chars().all(|ch| ch == '_') {
        return true;
    }

    // Common input indicators
    if text.ends_with(": _") || text.ends_with(":_") {
        return true;
    }

    false
}

/// Check if text represents a checkbox
fn is_checkbox(text: &str) -> bool {
    matches!(
        text,
        "[x]"
            | "[X]"
            | "[ ]"
            | "[✓]"
            | "[✔]"
            | "◉"
            | "◯"
            | "●"
            | "○"
            | "◼"
            | "◻"
            | "☐"
            | "☑"
            | "☒"
    )
}

/// Check if text represents a menu item
fn is_menu_item(text: &str) -> bool {
    // Common selection indicators at start
    text.starts_with('>')
        || text.starts_with('❯')
        || text.starts_with('›')
        || text.starts_with('→')
        || text.starts_with('▶')
        || text.starts_with("• ")
        || text.starts_with("* ")
        || text.starts_with("- ")
}

/// Check if text is a panel border (box-drawing characters)
fn is_panel_border(text: &str) -> bool {
    // Common box-drawing characters
    let box_chars = [
        '─', '│', '┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼', '═', '║', '╔', '╗', '╚', '╝', '╠',
        '╣', '╦', '╩', '╬',
    ];

    // If the majority of non-whitespace chars are box-drawing, it's a border
    let total = text.chars().filter(|c| !c.is_whitespace()).count();
    if total == 0 {
        return false;
    }

    let box_count = text.chars().filter(|c| box_chars.contains(c)).count();
    box_count > total / 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::CellStyle;
    use crate::vom::Rect;

    fn make_cluster(text: &str, style: CellStyle, x: u16, y: u16) -> Cluster {
        Cluster {
            rect: Rect::new(x, y, text.len() as u16, 1),
            text: text.to_string(),
            style,
            is_whitespace: false,
        }
    }

    #[test]
    fn test_button_detection() {
        let cluster = make_cluster("[Submit]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99); // cursor far away
        assert_eq!(role, Role::Button);
    }

    #[test]
    fn test_checkbox_not_button() {
        let cluster = make_cluster("[x]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::Checkbox);
    }

    #[test]
    fn test_input_from_cursor() {
        let cluster = make_cluster("Hello", CellStyle::default(), 0, 0);
        // Cursor is within the cluster
        let role = infer_role(&cluster, 0, 2);
        assert_eq!(role, Role::Input);
    }

    #[test]
    fn test_input_from_underscores() {
        let cluster = make_cluster("Name: ___", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::Input);
    }

    #[test]
    fn test_tab_from_inverse() {
        let cluster = make_cluster(
            "Tab1",
            CellStyle {
                inverse: true,
                ..Default::default()
            },
            0,
            0, // top row
        );
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::Tab);
    }

    #[test]
    fn test_tab_from_blue_bg() {
        let cluster = make_cluster(
            "Tab2",
            CellStyle {
                bg_color: Some(Color::Indexed(4)), // Blue
                ..Default::default()
            },
            0,
            0,
        );
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::Tab);
    }

    #[test]
    fn test_menu_item() {
        let cluster = make_cluster("> Option 1", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::MenuItem);
    }

    #[test]
    fn test_static_text_default() {
        let cluster = make_cluster("Hello World", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::StaticText);
    }

    #[test]
    fn test_classify_multiple() {
        let clusters = vec![
            make_cluster("[OK]", CellStyle::default(), 0, 0),
            make_cluster("Cancel", CellStyle::default(), 10, 0),
            make_cluster("[ ]", CellStyle::default(), 20, 0),
        ];

        let components = classify(clusters, 99, 99);

        assert_eq!(components.len(), 3);
        assert_eq!(components[0].role, Role::Button);
        assert_eq!(components[1].role, Role::StaticText);
        assert_eq!(components[2].role, Role::Checkbox);
    }

    #[test]
    fn test_cursor_at_cluster_start_boundary() {
        // Cluster at x=10, width=5 ("Hello")
        let cluster = make_cluster("Hello", CellStyle::default(), 10, 5);
        // Cursor at start boundary (x=10, y=5) - should be Input
        let role = infer_role(&cluster, 5, 10);
        assert_eq!(role, Role::Input);
    }

    #[test]
    fn test_cursor_at_cluster_end_boundary() {
        // Cluster at x=10, width=5 ("Hello") - valid range is 10-14
        let cluster = make_cluster("Hello", CellStyle::default(), 10, 5);
        // Cursor at end boundary (x=14, y=5) - should be Input
        let role = infer_role(&cluster, 5, 14);
        assert_eq!(role, Role::Input);
    }

    #[test]
    fn test_cursor_past_cluster_end() {
        // Cluster at x=10, width=5 - valid range is 10-14, so 15 is outside
        let cluster = make_cluster("Hello", CellStyle::default(), 10, 5);
        // Cursor one past end (x=15) - should NOT be Input
        let role = infer_role(&cluster, 5, 15);
        assert_eq!(role, Role::StaticText);
    }
}
