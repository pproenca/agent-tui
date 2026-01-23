use crate::style::Color;

use crate::vom::Cluster;
use crate::vom::Component;
use crate::vom::Role;
use crate::vom::hash_cluster;

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

fn infer_role(cluster: &Cluster, cursor_row: u16, cursor_col: u16) -> Role {
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
        if cluster.rect.y <= 2 {
            return Role::Tab;
        }

        return Role::MenuItem;
    }

    if let Some(Color::Indexed(idx)) = &cluster.style.bg_color {
        if *idx == 4 || *idx == 6 {
            return Role::Tab;
        }
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

    if is_menu_item(text) {
        return Role::MenuItem;
    }

    if is_tool_block_border(text) {
        return Role::ToolBlock;
    }

    if is_panel_border(text) {
        return Role::Panel;
    }

    if is_status_indicator(text) {
        return Role::Status;
    }

    Role::StaticText
}

fn is_button_text(text: &str) -> bool {
    if text.len() <= 2 {
        return false;
    }

    let inner = || text[1..text.len() - 1].trim();

    if text.starts_with('[') && text.ends_with(']') {
        return !matches!(inner(), "x" | "X" | " " | "" | "✓" | "✔");
    }

    if text.starts_with('(') && text.ends_with(')') {
        return !matches!(inner(), "" | " " | "o" | "O" | "●" | "◉");
    }

    text.starts_with('<') && text.ends_with('>')
}

fn is_input_field(text: &str) -> bool {
    if text.contains("___") {
        return true;
    }

    if !text.is_empty() && text.chars().all(|ch| ch == '_') {
        return true;
    }

    if text.ends_with(": _") || text.ends_with(":_") {
        return true;
    }

    false
}

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

fn is_menu_item(text: &str) -> bool {
    text.starts_with('>')
        || text.starts_with('❯')
        || text.starts_with('›')
        || text.starts_with('→')
        || text.starts_with('▶')
        || text.starts_with("• ")
        || text.starts_with("* ")
        || text.starts_with("- ")
}

fn is_panel_border(text: &str) -> bool {
    let box_chars = [
        '─', '│', '┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼', '═', '║', '╔', '╗', '╚', '╝', '╠',
        '╣', '╦', '╩', '╬',
    ];

    let total = text.chars().filter(|c| !c.is_whitespace()).count();
    if total == 0 {
        return false;
    }

    let box_count = text.chars().filter(|c| box_chars.contains(c)).count();
    box_count > total / 2
}

fn is_status_indicator(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }

    let first_char = text.chars().next().unwrap();

    const BRAILLE_SPINNERS: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    const CIRCLE_SPINNERS: [char; 4] = ['◐', '◑', '◒', '◓'];
    const STATUS_CHARS: [char; 4] = ['✓', '✔', '✗', '✘'];

    BRAILLE_SPINNERS.contains(&first_char)
        || CIRCLE_SPINNERS.contains(&first_char)
        || STATUS_CHARS.contains(&first_char)
}

fn is_tool_block_border(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }

    let first_char = text.chars().next().unwrap();
    let last_char = text.chars().last().unwrap();

    const ROUNDED_CORNERS: [char; 4] = ['╭', '╮', '╰', '╯'];

    ROUNDED_CORNERS.contains(&first_char) || ROUNDED_CORNERS.contains(&last_char)
}

fn is_prompt_marker(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed == ">" || trimmed == "> "
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

    #[test]
    fn test_button_detection() {
        let cluster = make_cluster("[Submit]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99);
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
            0,
        );
        let role = infer_role(&cluster, 99, 99);
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
        let cluster = make_cluster("Hello", CellStyle::default(), 10, 5);

        let role = infer_role(&cluster, 5, 10);
        assert_eq!(role, Role::Input);
    }

    #[test]
    fn test_cursor_at_cluster_end_boundary() {
        let cluster = make_cluster("Hello", CellStyle::default(), 10, 5);

        let role = infer_role(&cluster, 5, 14);
        assert_eq!(role, Role::Input);
    }

    #[test]
    fn test_cursor_past_cluster_end() {
        let cluster = make_cluster("Hello", CellStyle::default(), 10, 5);

        let role = infer_role(&cluster, 5, 15);
        assert_eq!(role, Role::StaticText);
    }

    // ============================================================
    // Status role detection tests
    // ============================================================

    #[test]
    fn test_status_spinner_braille() {
        // Braille spinner characters used in CLI loaders
        for spinner in ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'] {
            let text = format!("{} Loading...", spinner);
            let cluster = make_cluster(&text, CellStyle::default(), 0, 0);
            let role = infer_role(&cluster, 99, 99);
            assert_eq!(role, Role::Status, "Failed for spinner: {}", spinner);
        }
    }

    #[test]
    fn test_status_spinner_circle() {
        // Circle spinner characters
        for spinner in ['◐', '◑', '◒', '◓'] {
            let text = format!("{} Processing", spinner);
            let cluster = make_cluster(&text, CellStyle::default(), 0, 0);
            let role = infer_role(&cluster, 99, 99);
            assert_eq!(role, Role::Status, "Failed for spinner: {}", spinner);
        }
    }

    #[test]
    fn test_status_thinking_text() {
        let cluster = make_cluster("⠋ Thinking...", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::Status);
    }

    #[test]
    fn test_status_done_indicator() {
        let cluster = make_cluster("✓ Done", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::Status);
    }

    #[test]
    fn test_status_checkmark_complete() {
        let cluster = make_cluster("✔ Complete", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::Status);
    }

    #[test]
    fn test_status_not_regular_text() {
        // Regular text should NOT be detected as status
        let cluster = make_cluster("Hello World", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99);
        assert_ne!(role, Role::Status);
    }

    // ============================================================
    // ToolBlock role detection tests
    // ============================================================

    #[test]
    fn test_tool_block_top_border() {
        // Rounded top border with title: ╭─ Write ─╮
        let cluster = make_cluster("╭─ Write ─────────────────────╮", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::ToolBlock);
    }

    #[test]
    fn test_tool_block_bottom_border() {
        // Rounded bottom border: ╰──────────────────────────────╯
        let cluster = make_cluster("╰──────────────────────────────╯", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::ToolBlock);
    }

    #[test]
    fn test_tool_block_not_regular_panel() {
        // Regular panel border (square corners) should be Panel, not ToolBlock
        let cluster = make_cluster("┌──────────────────────────────┐", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::Panel);
    }

    // ============================================================
    // PromptMarker role detection tests
    // ============================================================

    #[test]
    fn test_prompt_marker_simple() {
        // Simple prompt marker at start of line
        let cluster = make_cluster(">", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::PromptMarker);
    }

    #[test]
    fn test_prompt_marker_with_space() {
        // Prompt marker with trailing space
        let cluster = make_cluster("> ", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::PromptMarker);
    }

    #[test]
    fn test_prompt_marker_not_menu_item() {
        // Menu item with content after > should be MenuItem, not PromptMarker
        let cluster = make_cluster("> Option 1", CellStyle::default(), 0, 5);
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::MenuItem);
    }

    #[test]
    fn test_prompt_marker_is_interactive() {
        assert!(Role::PromptMarker.is_interactive());
    }

    // ============================================================
    // Y/N button detection tests
    // ============================================================

    #[test]
    fn test_yn_button_y_with_spaces() {
        let cluster = make_cluster("[ Y ]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::Button);
    }

    #[test]
    fn test_yn_button_n_with_spaces() {
        let cluster = make_cluster("[ N ]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::Button);
    }

    #[test]
    fn test_yn_button_yes() {
        let cluster = make_cluster("[Yes]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::Button);
    }

    #[test]
    fn test_yn_button_no() {
        let cluster = make_cluster("[No]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::Button);
    }

    #[test]
    fn test_yn_not_checkbox() {
        // Single letter checkboxes should still be detected
        let cluster = make_cluster("[x]", CellStyle::default(), 0, 0);
        let role = infer_role(&cluster, 99, 99);
        assert_eq!(role, Role::Checkbox);
    }

    // ============================================================
    // Property-based tests
    // ============================================================

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

                let result1 = classify(clusters, cursor_row, cursor_col);
                let result2 = classify(clusters_clone, cursor_row, cursor_col);

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
                let components = classify(clusters, cursor_row, cursor_col);
                prop_assert_eq!(components.len(), count);
            }

            #[test]
            fn component_ids_unique(
                clusters in prop::collection::vec(arb_cluster(), 2..10),
                cursor_row in 0u16..50,
                cursor_col in 0u16..100
            ) {
                let components = classify(clusters, cursor_row, cursor_col);
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
