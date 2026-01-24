use super::Cluster;
use super::Component;
use super::Role;
use super::hash_cluster;
use super::patterns::{
    is_button_text, is_checkbox, is_code_block_border, is_diff_line, is_error_message,
    is_input_field, is_link, is_menu_item, is_panel_border, is_progress_bar, is_prompt_marker,
    is_status_indicator, is_tool_block_border,
};
use crate::core::CursorPosition;
use crate::core::style::Color;

/// ANSI indexed color for blue background (indicates tab in many TUIs)
const TAB_BG_BLUE: u8 = 4;
/// ANSI indexed color for cyan background (indicates tab in many TUIs)
const TAB_BG_CYAN: u8 = 6;

/// Options for the classification phase.
#[derive(Debug, Clone)]
pub struct ClassifyOptions {
    /// Row threshold for Tab detection (elements on row <= threshold with inverse are Tabs).
    pub tab_row_threshold: u16,
}

impl Default for ClassifyOptions {
    fn default() -> Self {
        Self {
            tab_row_threshold: 2,
        }
    }
}

pub fn classify(
    clusters: Vec<Cluster>,
    cursor: &CursorPosition,
    options: &ClassifyOptions,
) -> Vec<Component> {
    clusters
        .into_iter()
        .map(|cluster| {
            let role = infer_role(&cluster, cursor, options);
            let visual_hash = hash_cluster(&cluster);
            let selected = is_selected(&cluster);

            Component::with_selected(role, cluster.rect, cluster.text, visual_hash, selected)
        })
        .collect()
}

fn is_selected(cluster: &Cluster) -> bool {
    cluster.style.inverse || cluster.text.starts_with('❯')
}

/// Infers the role of a cluster based on its content and style.
///
/// # Classification Priority Order
///
/// The order of checks is important because some patterns overlap. The priority is:
///
/// 1. **Cursor position** → Input (cursor within cluster bounds)
/// 2. **Button text** → Button (bracketed text like `[OK]`, `<Cancel>`)
/// 3. **Inverse style** → Tab or MenuItem (based on row threshold)
/// 4. **Tab background color** → Tab (blue/cyan background)
/// 5. **Error prefixes** → ErrorMessage (`Error:`, `✗`)
/// 6. **Input field patterns** → Input (`___`, `: _`)
/// 7. **Checkbox markers** → Checkbox (`[x]`, `☐`)
/// 8. **Prompt marker** → PromptMarker (`>` alone) - BEFORE MenuItem!
/// 9. **Menu item prefixes** → MenuItem (`> `, `- `, `• `) - BEFORE Link/DiffLine!
/// 10. **URL/file paths** → Link (`https://`, `src/main.rs`)
/// 11. **Progress bar chars** → ProgressBar (`████░░░░`)
/// 12. **Diff line markers** → DiffLine (`+`, `-` without space, `@@`)
/// 13. **Tool block borders** → ToolBlock (rounded corners `╭╮╰╯`)
/// 14. **Code block borders** → CodeBlock (vertical line `│`)
/// 15. **Panel borders** → Panel (box drawing chars)
/// 16. **Status indicators** → Status (spinners, checkmarks)
/// 17. **Default** → StaticText
///
/// # Why Order Matters
///
/// - `PromptMarker` must precede `MenuItem` because `>` alone is a prompt, not a menu
/// - `MenuItem` must precede `Link` because `> src/main.rs` is a menu item, not a link
/// - `MenuItem` must precede `DiffLine` because `- List item` is a menu, not a diff deletion
fn infer_role(cluster: &Cluster, cursor: &CursorPosition, options: &ClassifyOptions) -> Role {
    let text = cluster.text.trim();

    // If cursor is within this cluster's bounds, it's an input field
    if cluster.rect.y == cursor.row
        && cursor.col >= cluster.rect.x
        && cursor.col < cluster.rect.x + cluster.rect.width
    {
        return Role::Input;
    }

    if is_button_text(text) {
        return Role::Button;
    }

    if cluster.style.inverse {
        if cluster.rect.y <= options.tab_row_threshold {
            return Role::Tab;
        }
        return Role::MenuItem;
    }

    if let Some(Color::Indexed(idx)) = &cluster.style.bg_color {
        if *idx == TAB_BG_BLUE || *idx == TAB_BG_CYAN {
            return Role::Tab;
        }
    }

    if is_error_message(text) {
        return Role::ErrorMessage;
    }

    if is_input_field(text) {
        return Role::Input;
    }

    if is_checkbox(text) {
        return Role::Checkbox;
    }

    // PromptMarker must be checked before MenuItem because ">" alone is a prompt,
    // not a menu item. MenuItem requires content after the prefix.
    if is_prompt_marker(text) {
        return Role::PromptMarker;
    }

    // Menu items are checked before Link and DiffLine because they use distinctive
    // prefixes (>, ❯, -, •, *) that could otherwise match those patterns.
    // For example, "> src/main.rs" should be MenuItem, not Link.
    // And "- List item" should be MenuItem, not DiffLine.
    if is_menu_item(text) {
        return Role::MenuItem;
    }

    if is_link(text) {
        return Role::Link;
    }

    if is_progress_bar(text) {
        return Role::ProgressBar;
    }

    if is_diff_line(text) {
        return Role::DiffLine;
    }

    if is_tool_block_border(text) {
        return Role::ToolBlock;
    }

    if is_code_block_border(text) {
        return Role::CodeBlock;
    }

    if is_panel_border(text) {
        return Role::Panel;
    }

    if is_status_indicator(text) {
        return Role::Status;
    }

    Role::StaticText
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::style::CellStyle;
    use crate::core::vom::Rect;

    fn make_cluster(text: &str, style: CellStyle, x: u16, y: u16) -> Cluster {
        Cluster {
            rect: Rect::new(x, y, text.len() as u16, 1),
            text: text.to_string(),
            style,
            is_whitespace: false,
        }
    }

    fn default_opts() -> ClassifyOptions {
        ClassifyOptions::default()
    }

    fn cursor(row: u16, col: u16) -> CursorPosition {
        CursorPosition {
            row,
            col,
            visible: true,
        }
    }

    fn no_cursor() -> CursorPosition {
        cursor(99, 99)
    }

    #[test]
    fn test_button_detection() {
        let cluster = make_cluster("[Submit]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::Button);
    }

    #[test]
    fn test_checkbox_not_button() {
        let cluster = make_cluster("[x]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::Checkbox);
    }

    #[test]
    fn test_input_from_cursor() {
        let cluster = make_cluster("Hello", CellStyle::default(), 0, 0);

        let role = infer_role(&cluster, &cursor(0, 2), &default_opts());
        assert_eq!(role, Role::Input);
    }

    #[test]
    fn test_input_from_underscores() {
        let cluster = make_cluster("Name: ___", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
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
            0,
        );
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::Tab);
    }

    #[test]
    fn test_tab_from_blue_bg() {
        let cluster = make_cluster(
            "Tab2",
            CellStyle {
                bg_color: Some(Color::Indexed(4)),
                ..Default::default()
            },
            0,
            0,
        );
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::Tab);
    }

    #[test]
    fn test_menu_item() {
        let cluster = make_cluster("> Option 1", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::MenuItem);
    }

    #[test]
    fn test_static_text_default() {
        let cluster = make_cluster("Hello World", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::StaticText);
    }

    #[test]
    fn test_classify_multiple() {
        let clusters = vec![
            make_cluster("[OK]", CellStyle::default(), 0, 0),
            make_cluster("Cancel", CellStyle::default(), 10, 0),
            make_cluster("[ ]", CellStyle::default(), 20, 0),
        ];

        let components = classify(clusters, &no_cursor(), &default_opts());

        assert_eq!(components.len(), 3);
        assert_eq!(components[0].role, Role::Button);
        assert_eq!(components[1].role, Role::StaticText);
        assert_eq!(components[2].role, Role::Checkbox);
    }

    #[test]
    fn test_cursor_at_cluster_start_boundary() {
        let cluster = make_cluster("Hello", CellStyle::default(), 10, 5);

        let role = infer_role(&cluster, &cursor(5, 10), &default_opts());
        assert_eq!(role, Role::Input);
    }

    #[test]
    fn test_cursor_at_cluster_end_boundary() {
        let cluster = make_cluster("Hello", CellStyle::default(), 10, 5);

        let role = infer_role(&cluster, &cursor(5, 14), &default_opts());
        assert_eq!(role, Role::Input);
    }

    #[test]
    fn test_cursor_past_cluster_end() {
        let cluster = make_cluster("Hello", CellStyle::default(), 10, 5);

        let role = infer_role(&cluster, &cursor(5, 15), &default_opts());
        assert_eq!(role, Role::StaticText);
    }

    #[test]
    fn test_status_spinner_braille() {
        // Braille spinner characters used in CLI loaders
        for spinner in ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'] {
            let text = format!("{} Loading...", spinner);
            let cluster = make_cluster(&text, CellStyle::default(), 0, 0);
            let role = infer_role(&cluster, &no_cursor(), &default_opts());
            assert_eq!(role, Role::Status, "Failed for spinner: {}", spinner);
        }
    }

    #[test]
    fn test_status_spinner_circle() {
        // Circle spinner characters
        for spinner in ['◐', '◑', '◒', '◓'] {
            let text = format!("{} Processing", spinner);
            let cluster = make_cluster(&text, CellStyle::default(), 0, 0);
            let role = infer_role(&cluster, &no_cursor(), &default_opts());
            assert_eq!(role, Role::Status, "Failed for spinner: {}", spinner);
        }
    }

    #[test]
    fn test_status_thinking_text() {
        let cluster = make_cluster("⠋ Thinking...", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::Status);
    }

    #[test]
    fn test_status_done_indicator() {
        let cluster = make_cluster("✓ Done", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::Status);
    }

    #[test]
    fn test_status_checkmark_complete() {
        let cluster = make_cluster("✔ Complete", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::Status);
    }

    #[test]
    fn test_status_not_regular_text() {
        // Regular text should NOT be detected as status
        let cluster = make_cluster("Hello World", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_ne!(role, Role::Status);
    }

    #[test]
    fn test_tool_block_top_border() {
        // Rounded top border with title: ╭─ Write ─╮
        let cluster = make_cluster(
            "╭─ Write ─────────────────────╮",
            CellStyle::default(),
            0,
            0,
        );
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::ToolBlock);
    }

    #[test]
    fn test_tool_block_bottom_border() {
        // Rounded bottom border: ╰──────────────────────────────╯
        let cluster = make_cluster(
            "╰──────────────────────────────╯",
            CellStyle::default(),
            0,
            0,
        );
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::ToolBlock);
    }

    #[test]
    fn test_tool_block_not_regular_panel() {
        // Regular panel border (square corners) should be Panel, not ToolBlock
        let cluster = make_cluster(
            "┌──────────────────────────────┐",
            CellStyle::default(),
            0,
            0,
        );
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::Panel);
    }

    #[test]
    fn test_prompt_marker_simple() {
        // Simple prompt marker at start of line
        let cluster = make_cluster(">", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::PromptMarker);
    }

    #[test]
    fn test_prompt_marker_with_space() {
        // Prompt marker with trailing space
        let cluster = make_cluster("> ", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::PromptMarker);
    }

    #[test]
    fn test_prompt_marker_not_menu_item() {
        // Menu item with content after > should be MenuItem, not PromptMarker
        let cluster = make_cluster("> Option 1", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::MenuItem);
    }

    #[test]
    fn test_prompt_marker_is_interactive() {
        assert!(Role::PromptMarker.is_interactive());
    }

    #[test]
    fn test_yn_button_y_with_spaces() {
        let cluster = make_cluster("[ Y ]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::Button);
    }

    #[test]
    fn test_yn_button_n_with_spaces() {
        let cluster = make_cluster("[ N ]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::Button);
    }

    #[test]
    fn test_yn_button_yes() {
        let cluster = make_cluster("[Yes]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::Button);
    }

    #[test]
    fn test_yn_button_no() {
        let cluster = make_cluster("[No]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::Button);
    }

    #[test]
    fn test_yn_not_checkbox() {
        // Single letter checkboxes should still be detected
        let cluster = make_cluster("[x]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::Checkbox);
    }

    // ============================================================
    // NEW ROLE TESTS - Phase 1: RED (failing tests for new roles)
    // ============================================================

    #[test]
    fn test_progress_bar_detection() {
        let cluster = make_cluster("████░░░░", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::ProgressBar);
    }

    #[test]
    fn test_progress_bar_bracket_detection() {
        let cluster = make_cluster("[===>    ]", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::ProgressBar);
    }

    #[test]
    fn test_link_url_detection() {
        let cluster = make_cluster("https://example.com", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::Link);
    }

    #[test]
    fn test_link_file_path_detection() {
        let cluster = make_cluster("src/main.rs:42", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::Link);
    }

    #[test]
    fn test_link_is_interactive() {
        assert!(Role::Link.is_interactive());
    }

    #[test]
    fn test_error_message_detection() {
        let cluster = make_cluster("Error: something failed", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::ErrorMessage);
    }

    #[test]
    fn test_error_message_failure_marker() {
        let cluster = make_cluster("✗ Failed to compile", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::ErrorMessage);
    }

    #[test]
    fn test_error_message_not_interactive() {
        assert!(!Role::ErrorMessage.is_interactive());
    }

    #[test]
    fn test_diff_line_addition_detection() {
        let cluster = make_cluster("+ added line", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::DiffLine);
    }

    #[test]
    fn test_diff_line_deletion_detection() {
        // Use pattern without space after dash - "- text" is now classified as MenuItem
        // because TUI menus commonly use "- " as bullet prefix
        let cluster = make_cluster("-removed_line", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::DiffLine);
    }

    #[test]
    fn test_diff_line_header_detection() {
        let cluster = make_cluster("@@ -1,5 +1,6 @@", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::DiffLine);
    }

    #[test]
    fn test_diff_line_not_interactive() {
        assert!(!Role::DiffLine.is_interactive());
    }

    #[test]
    fn test_code_block_detection() {
        let cluster = make_cluster("│ let x = 5;", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::CodeBlock);
    }

    #[test]
    fn test_code_block_not_interactive() {
        assert!(!Role::CodeBlock.is_interactive());
    }

    #[test]
    fn test_menu_item_selected_via_inverse() {
        let cluster = make_cluster(
            "Option 1",
            CellStyle {
                inverse: true,
                ..Default::default()
            },
            0,
            5,
        );
        let components = classify(vec![cluster], &no_cursor(), &default_opts());
        assert!(components[0].selected);
    }

    #[test]
    fn test_menu_item_selected_via_prefix() {
        let cluster = make_cluster("❯ Selected Option", CellStyle::default(), 0, 5);
        let components = classify(vec![cluster], &no_cursor(), &default_opts());
        assert!(components[0].selected);
    }

    #[test]
    fn test_menu_item_not_selected_by_default() {
        let cluster = make_cluster("Normal Option", CellStyle::default(), 0, 5);
        let components = classify(vec![cluster], &no_cursor(), &default_opts());
        assert!(!components[0].selected);
    }

    #[test]
    fn test_tab_row_threshold_configurable() {
        // Element on row 5 with inverse should be MenuItem with default threshold (2)
        let cluster = make_cluster(
            "Option",
            CellStyle {
                inverse: true,
                ..Default::default()
            },
            0,
            5,
        );
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(role, Role::MenuItem);

        // Same element should be Tab with threshold = 5
        let opts = ClassifyOptions {
            tab_row_threshold: 5,
        };
        let role = infer_role(&cluster, &no_cursor(), &opts);
        assert_eq!(role, Role::Tab);
    }

    #[test]
    fn test_menu_item_with_file_path_not_link() {
        // Menu items with file paths should be MenuItem, not Link
        // This tests the classification priority: is_menu_item() before is_link()
        let cluster = make_cluster("> src/main.rs", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(
            role,
            Role::MenuItem,
            "Menu item with file path should be MenuItem, not Link"
        );
    }

    #[test]
    fn test_menu_item_with_file_path_and_line_number() {
        // Menu items with file:line notation should still be MenuItem
        let cluster = make_cluster("> src/lib.rs:42", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(
            role,
            Role::MenuItem,
            "Menu item with file:line should be MenuItem"
        );
    }

    #[test]
    fn test_dash_list_item_not_diff_line() {
        // "- List item" with space after dash should be MenuItem, not DiffLine
        // This tests classification priority: is_menu_item() before is_diff_line()
        let cluster = make_cluster("- List item", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(
            role,
            Role::MenuItem,
            "Dash list item should be MenuItem, not DiffLine"
        );
    }

    #[test]
    fn test_dash_list_navigation() {
        // Common TUI navigation patterns with dash prefix
        let cluster = make_cluster("- Select option", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, &no_cursor(), &default_opts());
        assert_eq!(
            role,
            Role::MenuItem,
            "Dash navigation item should be MenuItem"
        );
    }

    mod prop_tests {
        use super::*;
        use proptest::prelude::*;

        fn arb_cluster() -> impl Strategy<Value = Cluster> {
            (
                "[a-zA-Z0-9 ]{1,20}",
                any::<bool>(),
                any::<bool>(),
                0u16..100,
                0u16..50,
            )
                .prop_map(|(text, bold, inverse, x, y)| Cluster {
                    rect: Rect::new(x, y, text.len() as u16, 1),
                    text,
                    style: CellStyle {
                        bold,
                        underline: false,
                        inverse,
                        fg_color: None,
                        bg_color: None,
                    },
                    is_whitespace: false,
                })
        }

        proptest! {
            #[test]
            fn classification_is_deterministic(
                clusters in prop::collection::vec(arb_cluster(), 1..10),
                cursor_row in 0u16..50,
                cursor_col in 0u16..100
            ) {
                let clusters_clone: Vec<Cluster> = clusters.iter().map(|c| Cluster {
                    rect: c.rect,
                    text: c.text.clone(),
                    style: c.style.clone(),
                    is_whitespace: c.is_whitespace,
                }).collect();
                let opts = ClassifyOptions::default();
                let cur = cursor(cursor_row, cursor_col);

                let result1 = classify(clusters, &cur, &opts);
                let result2 = classify(clusters_clone, &cur, &opts);

                prop_assert_eq!(result1.len(), result2.len());
                for (a, b) in result1.iter().zip(result2.iter()) {
                    prop_assert_eq!(a.role, b.role);
                    prop_assert_eq!(a.bounds, b.bounds);
                    prop_assert_eq!(&a.text_content, &b.text_content);
                    prop_assert_eq!(a.visual_hash, b.visual_hash);
                }
            }

            #[test]
            fn classify_preserves_count(
                clusters in prop::collection::vec(arb_cluster(), 0..20),
                cursor_row in 0u16..50,
                cursor_col in 0u16..100
            ) {
                let count = clusters.len();
                let opts = ClassifyOptions::default();
                let cur = cursor(cursor_row, cursor_col);
                let components = classify(clusters, &cur, &opts);
                prop_assert_eq!(components.len(), count);
            }

            #[test]
            fn component_ids_unique(
                clusters in prop::collection::vec(arb_cluster(), 2..10),
                cursor_row in 0u16..50,
                cursor_col in 0u16..100
            ) {
                let opts = ClassifyOptions::default();
                let cur = cursor(cursor_row, cursor_col);
                let components = classify(clusters, &cur, &opts);
                let ids: Vec<_> = components.iter().map(|c| c.id).collect();

                for (i, id) in ids.iter().enumerate() {
                    prop_assert!(
                        !ids[i + 1..].contains(id),
                        "Duplicate component ID found"
                    );
                }
            }
        }
    }
}
