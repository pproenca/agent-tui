use crate::style::Color;

/// ANSI indexed color for blue background (indicates tab in many TUIs)
const TAB_BG_BLUE: u8 = 4;
/// ANSI indexed color for cyan background (indicates tab in many TUIs)
const TAB_BG_CYAN: u8 = 6;

use crate::vom::Cluster;
use crate::vom::Component;
use crate::vom::Role;
use crate::vom::hash_cluster;
use crate::vom::patterns::{
    is_button_text, is_checkbox, is_code_block_border, is_diff_line, is_error_message,
    is_input_field, is_link, is_menu_item, is_panel_border, is_progress_bar, is_prompt_marker,
    is_status_indicator, is_tool_block_border,
};

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
    cursor_row: u16,
    cursor_col: u16,
    options: &ClassifyOptions,
) -> Vec<Component> {
    clusters
        .into_iter()
        .map(|cluster| {
            let role = infer_role(&cluster, cursor_row, cursor_col, options);
            let visual_hash = hash_cluster(&cluster);
            let selected = is_selected(&cluster);

            Component::with_selected(role, cluster.rect, cluster.text, visual_hash, selected)
        })
        .collect()
}

fn is_selected(cluster: &Cluster) -> bool {
    cluster.style.inverse || cluster.text.starts_with('❯')
}

fn infer_role(
    cluster: &Cluster,
    cursor_row: u16,
    cursor_col: u16,
    options: &ClassifyOptions,
) -> Role {
    let text = cluster.text.trim();

    if cluster.rect.y == cursor_row
        && cursor_col >= cluster.rect.x
        && cursor_col < cluster.rect.x + cluster.rect.width
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

    if is_link(text) {
        return Role::Link;
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

    if is_prompt_marker(text) {
        return Role::PromptMarker;
    }

    if is_progress_bar(text) {
        return Role::ProgressBar;
    }

    if is_diff_line(text) {
        return Role::DiffLine;
    }

    if is_menu_item(text) {
        return Role::MenuItem;
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
    use crate::style::CellStyle;
    use crate::vom::Rect;

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

    #[test]
    fn test_button_detection() {
        let cluster = make_cluster("[Submit]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::Button);
    }

    #[test]
    fn test_checkbox_not_button() {
        let cluster = make_cluster("[x]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::Checkbox);
    }

    #[test]
    fn test_input_from_cursor() {
        let cluster = make_cluster("Hello", CellStyle::default(), 0, 0);

        let role = infer_role(&cluster, 0, 2, &default_opts());
        assert_eq!(role, Role::Input);
    }

    #[test]
    fn test_input_from_underscores() {
        let cluster = make_cluster("Name: ___", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99, &default_opts());
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
        let role = infer_role(&cluster, 99, 99, &default_opts());
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
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::Tab);
    }

    #[test]
    fn test_menu_item() {
        let cluster = make_cluster("> Option 1", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::MenuItem);
    }

    #[test]
    fn test_static_text_default() {
        let cluster = make_cluster("Hello World", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::StaticText);
    }

    #[test]
    fn test_classify_multiple() {
        let clusters = vec![
            make_cluster("[OK]", CellStyle::default(), 0, 0),
            make_cluster("Cancel", CellStyle::default(), 10, 0),
            make_cluster("[ ]", CellStyle::default(), 20, 0),
        ];

        let components = classify(clusters, 99, 99, &default_opts());

        assert_eq!(components.len(), 3);
        assert_eq!(components[0].role, Role::Button);
        assert_eq!(components[1].role, Role::StaticText);
        assert_eq!(components[2].role, Role::Checkbox);
    }

    #[test]
    fn test_cursor_at_cluster_start_boundary() {
        let cluster = make_cluster("Hello", CellStyle::default(), 10, 5);

        let role = infer_role(&cluster, 5, 10, &default_opts());
        assert_eq!(role, Role::Input);
    }

    #[test]
    fn test_cursor_at_cluster_end_boundary() {
        let cluster = make_cluster("Hello", CellStyle::default(), 10, 5);

        let role = infer_role(&cluster, 5, 14, &default_opts());
        assert_eq!(role, Role::Input);
    }

    #[test]
    fn test_cursor_past_cluster_end() {
        let cluster = make_cluster("Hello", CellStyle::default(), 10, 5);

        let role = infer_role(&cluster, 5, 15, &default_opts());
        assert_eq!(role, Role::StaticText);
    }

    #[test]
    fn test_status_spinner_braille() {
        // Braille spinner characters used in CLI loaders
        for spinner in ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'] {
            let text = format!("{} Loading...", spinner);
            let cluster = make_cluster(&text, CellStyle::default(), 0, 0);
            let role = infer_role(&cluster, 99, 99, &default_opts());
            assert_eq!(role, Role::Status, "Failed for spinner: {}", spinner);
        }
    }

    #[test]
    fn test_status_spinner_circle() {
        // Circle spinner characters
        for spinner in ['◐', '◑', '◒', '◓'] {
            let text = format!("{} Processing", spinner);
            let cluster = make_cluster(&text, CellStyle::default(), 0, 0);
            let role = infer_role(&cluster, 99, 99, &default_opts());
            assert_eq!(role, Role::Status, "Failed for spinner: {}", spinner);
        }
    }

    #[test]
    fn test_status_thinking_text() {
        let cluster = make_cluster("⠋ Thinking...", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::Status);
    }

    #[test]
    fn test_status_done_indicator() {
        let cluster = make_cluster("✓ Done", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::Status);
    }

    #[test]
    fn test_status_checkmark_complete() {
        let cluster = make_cluster("✔ Complete", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::Status);
    }

    #[test]
    fn test_status_not_regular_text() {
        // Regular text should NOT be detected as status
        let cluster = make_cluster("Hello World", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99, &default_opts());
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
        let role = infer_role(&cluster, 99, 99, &default_opts());
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
        let role = infer_role(&cluster, 99, 99, &default_opts());
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
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::Panel);
    }

    #[test]
    fn test_prompt_marker_simple() {
        // Simple prompt marker at start of line
        let cluster = make_cluster(">", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::PromptMarker);
    }

    #[test]
    fn test_prompt_marker_with_space() {
        // Prompt marker with trailing space
        let cluster = make_cluster("> ", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::PromptMarker);
    }

    #[test]
    fn test_prompt_marker_not_menu_item() {
        // Menu item with content after > should be MenuItem, not PromptMarker
        let cluster = make_cluster("> Option 1", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::MenuItem);
    }

    #[test]
    fn test_prompt_marker_is_interactive() {
        assert!(Role::PromptMarker.is_interactive());
    }

    #[test]
    fn test_yn_button_y_with_spaces() {
        let cluster = make_cluster("[ Y ]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::Button);
    }

    #[test]
    fn test_yn_button_n_with_spaces() {
        let cluster = make_cluster("[ N ]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::Button);
    }

    #[test]
    fn test_yn_button_yes() {
        let cluster = make_cluster("[Yes]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::Button);
    }

    #[test]
    fn test_yn_button_no() {
        let cluster = make_cluster("[No]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::Button);
    }

    #[test]
    fn test_yn_not_checkbox() {
        // Single letter checkboxes should still be detected
        let cluster = make_cluster("[x]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::Checkbox);
    }

    // ============================================================
    // NEW ROLE TESTS - Phase 1: RED (failing tests for new roles)
    // ============================================================

    #[test]
    fn test_progress_bar_detection() {
        let cluster = make_cluster("████░░░░", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::ProgressBar);
    }

    #[test]
    fn test_progress_bar_bracket_detection() {
        let cluster = make_cluster("[===>    ]", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::ProgressBar);
    }

    #[test]
    fn test_link_url_detection() {
        let cluster = make_cluster("https://example.com", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::Link);
    }

    #[test]
    fn test_link_file_path_detection() {
        let cluster = make_cluster("src/main.rs:42", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::Link);
    }

    #[test]
    fn test_link_is_interactive() {
        assert!(Role::Link.is_interactive());
    }

    #[test]
    fn test_error_message_detection() {
        let cluster = make_cluster("Error: something failed", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::ErrorMessage);
    }

    #[test]
    fn test_error_message_failure_marker() {
        let cluster = make_cluster("✗ Failed to compile", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::ErrorMessage);
    }

    #[test]
    fn test_error_message_not_interactive() {
        assert!(!Role::ErrorMessage.is_interactive());
    }

    #[test]
    fn test_diff_line_addition_detection() {
        let cluster = make_cluster("+ added line", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::DiffLine);
    }

    #[test]
    fn test_diff_line_deletion_detection() {
        let cluster = make_cluster("- removed line", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::DiffLine);
    }

    #[test]
    fn test_diff_line_header_detection() {
        let cluster = make_cluster("@@ -1,5 +1,6 @@", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::DiffLine);
    }

    #[test]
    fn test_diff_line_not_interactive() {
        assert!(!Role::DiffLine.is_interactive());
    }

    #[test]
    fn test_code_block_detection() {
        let cluster = make_cluster("│ let x = 5;", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99, &default_opts());
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
        let components = classify(vec![cluster], 99, 99, &default_opts());
        assert!(components[0].selected);
    }

    #[test]
    fn test_menu_item_selected_via_prefix() {
        let cluster = make_cluster("❯ Selected Option", CellStyle::default(), 0, 5);
        let components = classify(vec![cluster], 99, 99, &default_opts());
        assert!(components[0].selected);
    }

    #[test]
    fn test_menu_item_not_selected_by_default() {
        let cluster = make_cluster("Normal Option", CellStyle::default(), 0, 5);
        let components = classify(vec![cluster], 99, 99, &default_opts());
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
        let role = infer_role(&cluster, 99, 99, &default_opts());
        assert_eq!(role, Role::MenuItem);

        // Same element should be Tab with threshold = 5
        let opts = ClassifyOptions {
            tab_row_threshold: 5,
        };
        let role = infer_role(&cluster, 99, 99, &opts);
        assert_eq!(role, Role::Tab);
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

                let result1 = classify(clusters, cursor_row, cursor_col, &opts);
                let result2 = classify(clusters_clone, cursor_row, cursor_col, &opts);

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
                let components = classify(clusters, cursor_row, cursor_col, &opts);
                prop_assert_eq!(components.len(), count);
            }

            #[test]
            fn component_ids_unique(
                clusters in prop::collection::vec(arb_cluster(), 2..10),
                cursor_row in 0u16..50,
                cursor_col in 0u16..100
            ) {
                let opts = ClassifyOptions::default();
                let components = classify(clusters, cursor_row, cursor_col, &opts);
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
