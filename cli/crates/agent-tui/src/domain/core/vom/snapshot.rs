use super::Component;

#[derive(Debug, Clone)]
pub struct SnapshotOptions {
    pub interactive_only: bool,
    pub tab_row_threshold: u16,
}

impl Default for SnapshotOptions {
    fn default() -> Self {
        Self {
            interactive_only: false,
            tab_row_threshold: 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SnapshotStats {
    pub total: usize,
    pub interactive: usize,
    pub lines: usize,
}

#[derive(Debug, Clone)]
pub struct AccessibilitySnapshot {
    pub tree: String,
    pub stats: SnapshotStats,
}

pub fn format_snapshot(
    components: &[Component],
    options: &SnapshotOptions,
) -> AccessibilitySnapshot {
    let mut lines = Vec::with_capacity(components.len());
    let mut total = 0usize;
    let mut interactive_count = 0usize;

    for component in components {
        if options.interactive_only && !component.role.is_interactive() {
            continue;
        }

        total += 1;

        if component.role.is_interactive() {
            interactive_count += 1;
        }

        let name = component.text_content.trim();
        let line = if name.is_empty() {
            format!("- {}", component.role)
        } else {
            let escaped = name.replace('"', "\\\"");
            format!("- {} \"{}\"", component.role, escaped)
        };
        lines.push(line);
    }

    let tree = lines.join("\n");
    let line_count = lines.len();

    AccessibilitySnapshot {
        tree,
        stats: SnapshotStats {
            total,
            interactive: interactive_count,
            lines: line_count,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::core::vom::{Rect, Role};
    fn make_component(role: Role, text: &str, x: u16, y: u16, width: u16) -> Component {
        Component {
            role,
            bounds: Rect::new(x, y, width, 1),
            text_content: text.to_string(),
            visual_hash: 12345,
            selected: false,
        }
    }

    #[test]
    fn test_snapshot_text_format_button() {
        let components = vec![make_component(Role::Button, "[ OK ]", 10, 5, 6)];
        let snapshot = format_snapshot(&components, &SnapshotOptions::default());

        assert!(snapshot.tree.contains("button"));
        assert!(snapshot.tree.contains("[ OK ]"));
    }

    #[test]
    fn test_snapshot_text_format_multiple() {
        let components = vec![
            make_component(Role::Button, "[ OK ]", 10, 5, 6),
            make_component(Role::Input, ">", 0, 0, 1),
            make_component(Role::StaticText, "Hello", 0, 1, 5),
        ];
        let snapshot = format_snapshot(&components, &SnapshotOptions::default());

        assert!(snapshot.tree.contains("button"));
        assert!(snapshot.tree.contains("input"));
        assert!(snapshot.tree.contains("text"));
    }

    #[test]
    fn test_snapshot_stats() {
        let components = vec![
            make_component(Role::Button, "A", 0, 0, 1),
            make_component(Role::StaticText, "B", 5, 0, 1),
        ];
        let snapshot = format_snapshot(&components, &SnapshotOptions::default());

        assert_eq!(snapshot.stats.total, 2);
        assert_eq!(snapshot.stats.interactive, 1);
    }

    #[test]
    fn test_interactive_filter_excludes_static_text() {
        let components = vec![
            make_component(Role::Button, "OK", 0, 0, 2),
            make_component(Role::StaticText, "Hello", 5, 0, 5),
            make_component(Role::Input, ">", 0, 1, 1),
        ];
        let options = SnapshotOptions {
            interactive_only: true,
            ..Default::default()
        };
        let snapshot = format_snapshot(&components, &options);

        assert_eq!(snapshot.stats.total, 2);
        assert!(!snapshot.tree.contains("text"));
        assert!(snapshot.tree.contains("button"));
        assert!(snapshot.tree.contains("input"));
    }

    #[test]
    fn test_interactive_filter_excludes_panel() {
        let components = vec![
            make_component(Role::Panel, "───", 0, 0, 3),
            make_component(Role::Button, "OK", 5, 0, 2),
        ];
        let options = SnapshotOptions {
            interactive_only: true,
            ..Default::default()
        };
        let snapshot = format_snapshot(&components, &options);

        assert_eq!(snapshot.stats.total, 1);
        assert!(!snapshot.tree.contains("panel"));
        assert!(snapshot.tree.contains("button"));
    }

    #[test]
    fn test_interactive_filter_excludes_status() {
        let components = vec![
            make_component(Role::Status, "⠋ Loading", 0, 0, 10),
            make_component(Role::Button, "Cancel", 0, 1, 6),
        ];
        let options = SnapshotOptions {
            interactive_only: true,
            ..Default::default()
        };
        let snapshot = format_snapshot(&components, &options);

        assert_eq!(snapshot.stats.total, 1);
        assert!(!snapshot.tree.contains("status"));
        assert!(snapshot.tree.contains("button"));
    }

    #[test]
    fn test_interactive_filter_includes_all_interactive() {
        let components = vec![
            make_component(Role::Button, "OK", 0, 0, 2),
            make_component(Role::Input, ">", 0, 1, 1),
            make_component(Role::Checkbox, "[x]", 0, 2, 3),
            make_component(Role::MenuItem, "> opt", 0, 3, 5),
            make_component(Role::Tab, "Tab1", 0, 4, 4),
            make_component(Role::PromptMarker, ">", 0, 5, 1),
        ];
        let options = SnapshotOptions {
            interactive_only: true,
            ..Default::default()
        };
        let snapshot = format_snapshot(&components, &options);

        assert_eq!(snapshot.stats.total, 6);
        assert_eq!(snapshot.stats.interactive, 6);
    }

    #[test]
    fn test_snapshot_escapes_quotes_in_name() {
        let components = vec![make_component(Role::Button, r#"Say "Hello""#, 0, 0, 12)];
        let snapshot = format_snapshot(&components, &SnapshotOptions::default());
        assert!(snapshot.tree.contains(r#"Say \"Hello\""#));
    }

    mod prop_tests {
        use super::*;
        use proptest::prelude::*;

        fn arb_role() -> impl Strategy<Value = Role> {
            prop_oneof![
                Just(Role::Button),
                Just(Role::Tab),
                Just(Role::Input),
                Just(Role::StaticText),
                Just(Role::Panel),
                Just(Role::Checkbox),
                Just(Role::MenuItem),
                Just(Role::Status),
                Just(Role::ToolBlock),
                Just(Role::PromptMarker),
            ]
        }

        fn arb_component() -> impl Strategy<Value = Component> {
            (
                arb_role(),
                "[a-zA-Z0-9 ]{0,20}",
                0u16..100,
                0u16..50,
                1u16..20,
            )
                .prop_map(|(role, text, x, y, width)| Component {
                    role,
                    bounds: Rect::new(x, y, width, 1),
                    text_content: text,
                    visual_hash: 12345,
                    selected: false,
                })
        }

        proptest! {
            #[test]
            fn snapshot_is_deterministic(
                components in prop::collection::vec(arb_component(), 1..20)
            ) {
                let options = SnapshotOptions::default();

                let snapshot1 = format_snapshot(&components, &options);
                let snapshot2 = format_snapshot(&components, &options);

                prop_assert_eq!(&snapshot1.tree, &snapshot2.tree);
                prop_assert_eq!(snapshot1.stats.total, snapshot2.stats.total);
                prop_assert_eq!(snapshot1.stats.interactive, snapshot2.stats.interactive);
                prop_assert_eq!(snapshot1.stats.lines, snapshot2.stats.lines);
            }

            #[test]
            fn snapshot_count_matches_components(
                components in prop::collection::vec(arb_component(), 0..20)
            ) {
                let options = SnapshotOptions::default();
                let snapshot = format_snapshot(&components, &options);

                prop_assert_eq!(snapshot.stats.total, components.len());
            }

            #[test]
            fn interactive_filter_reduces_or_maintains_count(
                components in prop::collection::vec(arb_component(), 0..20)
            ) {
                let all_snapshot = format_snapshot(&components, &SnapshotOptions::default());
                let interactive_snapshot = format_snapshot(
                    &components,
                    &SnapshotOptions { interactive_only: true, ..Default::default() }
                );

                prop_assert!(interactive_snapshot.stats.total <= all_snapshot.stats.total);
            }
        }
    }
}
