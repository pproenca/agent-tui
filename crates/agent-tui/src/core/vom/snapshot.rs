use std::collections::HashMap;

use super::{Component, Rect};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl From<Rect> for Bounds {
    fn from(r: Rect) -> Self {
        Self {
            x: r.x,
            y: r.y,
            width: r.width,
            height: r.height,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ElementRef {
    pub role: String,
    pub name: Option<String>,
    pub bounds: Bounds,
    pub visual_hash: u64,
    pub nth: Option<usize>,
    pub selected: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RefMap {
    pub refs: HashMap<String, ElementRef>,
}

impl RefMap {
    pub fn get(&self, ref_id: &str) -> Option<&ElementRef> {
        self.refs.get(ref_id)
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
    pub refs: RefMap,
    pub stats: SnapshotStats,
}

pub fn format_snapshot(
    components: &[Component],
    options: &SnapshotOptions,
) -> AccessibilitySnapshot {
    let mut refs = RefMap::default();
    let mut lines = Vec::with_capacity(components.len());
    let mut ref_counter = 0usize;
    let mut interactive_count = 0usize;

    let mut role_counts: HashMap<String, usize> = HashMap::with_capacity(16);

    for component in components {
        if options.interactive_only && !component.role.is_interactive() {
            continue;
        }

        ref_counter += 1;
        let ref_id = format!("e{}", ref_counter);

        if component.role.is_interactive() {
            interactive_count += 1;
        }

        let name = component.text_content.trim();
        let role_str = component.role.to_string();

        let entry = role_counts.entry(role_str.clone()).or_insert(0);
        let nth = *entry;
        *entry += 1;

        let line = if name.is_empty() {
            format!("- {} [ref={}]", component.role, ref_id)
        } else {
            let escaped = name.replace('"', "\\\"");
            format!("- {} \"{}\" [ref={}]", component.role, escaped, ref_id)
        };
        lines.push(line);

        refs.refs.insert(
            ref_id,
            ElementRef {
                role: role_str,
                name: (!name.is_empty()).then(|| name.to_string()),
                bounds: component.bounds.into(),
                visual_hash: component.visual_hash,
                nth: Some(nth),
                selected: component.selected,
            },
        );
    }

    let tree = lines.join("\n");
    let line_count = lines.len();

    AccessibilitySnapshot {
        tree,
        refs,
        stats: SnapshotStats {
            total: ref_counter,
            interactive: interactive_count,
            lines: line_count,
        },
    }
}

pub fn parse_ref(arg: &str) -> Option<String> {
    if let Some(stripped) = arg.strip_prefix('@') {
        Some(stripped.to_string())
    } else if let Some(stripped) = arg.strip_prefix("ref=") {
        Some(stripped.to_string())
    } else if let Some(suffix) = arg.strip_prefix('e') {
        if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
            Some(arg.to_string())
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::vom::Role;
    use uuid::Uuid;

    fn make_component(role: Role, text: &str, x: u16, y: u16, width: u16) -> Component {
        Component {
            id: Uuid::new_v4(),
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
        assert!(snapshot.tree.contains("[ref=e1]"));
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

        assert!(snapshot.tree.contains("[ref=e1]"));
        assert!(snapshot.tree.contains("[ref=e2]"));
        assert!(snapshot.tree.contains("[ref=e3]"));
    }

    #[test]
    fn test_snapshot_refs_sequential() {
        let components = vec![
            make_component(Role::Button, "A", 0, 0, 1),
            make_component(Role::Button, "B", 5, 0, 1),
            make_component(Role::Input, "C", 10, 0, 1),
        ];
        let snapshot = format_snapshot(&components, &SnapshotOptions::default());

        assert!(snapshot.refs.get("e1").is_some());
        assert!(snapshot.refs.get("e2").is_some());
        assert!(snapshot.refs.get("e3").is_some());
        assert!(snapshot.refs.get("e4").is_none());
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
    fn test_parse_ref_at_prefix() {
        assert_eq!(parse_ref("@e1"), Some("e1".to_string()));
        assert_eq!(parse_ref("@e42"), Some("e42".to_string()));
    }

    #[test]
    fn test_parse_ref_ref_equals() {
        assert_eq!(parse_ref("ref=e1"), Some("e1".to_string()));
    }

    #[test]
    fn test_parse_ref_bare() {
        assert_eq!(parse_ref("e1"), Some("e1".to_string()));
        assert_eq!(parse_ref("e123"), Some("e123".to_string()));
    }

    #[test]
    fn test_parse_ref_invalid() {
        assert_eq!(parse_ref("button"), None);
        assert_eq!(parse_ref("1"), None);
        assert_eq!(parse_ref(""), None);
    }

    #[test]
    fn test_refmap_contains_bounds() {
        let components = vec![make_component(Role::Button, "OK", 10, 5, 6)];
        let snapshot = format_snapshot(&components, &SnapshotOptions::default());

        let elem = snapshot.refs.get("e1").unwrap();
        assert_eq!(elem.bounds.x, 10);
        assert_eq!(elem.bounds.y, 5);
        assert_eq!(elem.bounds.width, 6);
        assert_eq!(elem.bounds.height, 1);
    }

    #[test]
    fn test_refmap_contains_role() {
        let components = vec![make_component(Role::Input, ">", 0, 0, 1)];
        let snapshot = format_snapshot(&components, &SnapshotOptions::default());

        let elem = snapshot.refs.get("e1").unwrap();
        assert_eq!(elem.role, "input");
    }

    #[test]
    fn test_refmap_contains_name() {
        let components = vec![make_component(Role::Button, "Submit", 0, 0, 6)];
        let snapshot = format_snapshot(&components, &SnapshotOptions::default());

        let elem = snapshot.refs.get("e1").unwrap();
        assert_eq!(elem.name, Some("Submit".to_string()));
    }

    #[test]
    fn test_refmap_empty_name_is_none() {
        let components = vec![make_component(Role::Panel, "", 0, 0, 10)];
        let snapshot = format_snapshot(&components, &SnapshotOptions::default());

        let elem = snapshot.refs.get("e1").unwrap();
        assert_eq!(elem.name, None);
    }

    #[test]
    fn test_refmap_lookup_missing() {
        let components = vec![make_component(Role::Button, "OK", 0, 0, 2)];
        let snapshot = format_snapshot(&components, &SnapshotOptions::default());

        assert!(snapshot.refs.get("e999").is_none());
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

    #[test]
    fn test_interactive_filter_refs_renumbered() {
        let components = vec![
            make_component(Role::StaticText, "A", 0, 0, 1),
            make_component(Role::Button, "B", 0, 1, 1),
            make_component(Role::StaticText, "C", 0, 2, 1),
            make_component(Role::Input, "D", 0, 3, 1),
        ];
        let options = SnapshotOptions {
            interactive_only: true,
            ..Default::default()
        };
        let snapshot = format_snapshot(&components, &options);

        assert!(snapshot.refs.get("e1").is_some());
        assert!(snapshot.refs.get("e2").is_some());
        assert!(snapshot.refs.get("e3").is_none());
    }

    #[test]
    fn test_nth_field_populated() {
        let components = vec![
            make_component(Role::Button, "A", 0, 0, 1),
            make_component(Role::StaticText, "text", 5, 0, 4),
            make_component(Role::Button, "B", 10, 0, 1),
            make_component(Role::Button, "C", 15, 0, 1),
        ];
        let snapshot = format_snapshot(&components, &SnapshotOptions::default());

        let e1 = snapshot.refs.get("e1").unwrap();
        assert_eq!(e1.nth, Some(0));

        let e2 = snapshot.refs.get("e2").unwrap();
        assert_eq!(e2.nth, Some(0));

        let e3 = snapshot.refs.get("e3").unwrap();
        assert_eq!(e3.nth, Some(1));

        let e4 = snapshot.refs.get("e4").unwrap();
        assert_eq!(e4.nth, Some(2));
    }

    #[test]
    fn test_selected_state_from_inverse() {
        let mut comp = make_component(Role::MenuItem, "Option 1", 0, 0, 8);
        comp.selected = true;
        let components = vec![comp];
        let snapshot = format_snapshot(&components, &SnapshotOptions::default());
        let elem = snapshot.refs.get("e1").unwrap();
        assert!(elem.selected);
    }

    #[test]
    fn test_selected_state_default_false() {
        let comp = make_component(Role::MenuItem, "Option 1", 0, 0, 8);
        let components = vec![comp];
        let snapshot = format_snapshot(&components, &SnapshotOptions::default());
        let elem = snapshot.refs.get("e1").unwrap();
        assert!(!elem.selected);
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
                    id: Uuid::new_v4(),
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
                prop_assert_eq!(snapshot1.refs.refs.len(), snapshot2.refs.refs.len());
            }

            #[test]
            fn snapshot_ref_count_matches_components(
                components in prop::collection::vec(arb_component(), 0..20)
            ) {
                let options = SnapshotOptions::default();
                let snapshot = format_snapshot(&components, &options);

                prop_assert_eq!(snapshot.refs.refs.len(), components.len());
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
                prop_assert!(interactive_snapshot.refs.refs.len() <= all_snapshot.refs.refs.len());
            }

            #[test]
            fn refs_are_sequential_starting_at_e1(
                components in prop::collection::vec(arb_component(), 1..10)
            ) {
                let snapshot = format_snapshot(&components, &SnapshotOptions::default());

                for i in 1..=components.len() {
                    let ref_key = format!("e{}", i);
                    prop_assert!(
                        snapshot.refs.get(&ref_key).is_some(),
                        "Missing ref: {}", ref_key
                    );
                }

                let extra_ref = format!("e{}", components.len() + 1);
                prop_assert!(snapshot.refs.get(&extra_ref).is_none());
            }

            #[test]
            fn tree_contains_all_refs(
                components in prop::collection::vec(arb_component(), 1..10)
            ) {
                let snapshot = format_snapshot(&components, &SnapshotOptions::default());

                for i in 1..=components.len() {
                    let ref_marker = format!("[ref=e{}]", i);
                    prop_assert!(
                        snapshot.tree.contains(&ref_marker),
                        "Tree missing ref marker: {}", ref_marker
                    );
                }
            }

            #[test]
            fn nth_is_sequential_per_role(
                components in prop::collection::vec(arb_component(), 1..20)
            ) {
                let snapshot = format_snapshot(&components, &SnapshotOptions::default());


                let mut by_role: HashMap<String, Vec<usize>> = HashMap::new();
                for elem in snapshot.refs.refs.values() {
                    by_role.entry(elem.role.clone())
                        .or_default()
                        .push(elem.nth.unwrap());
                }


                for (role, mut nths) in by_role {
                    nths.sort();
                    let expected: Vec<usize> = (0..nths.len()).collect();
                    prop_assert_eq!(nths, expected, "Non-sequential nth for role: {}", role);
                }
            }
        }
    }
}
