use std::collections::HashMap;

use crate::vom::{Component, Rect};

#[derive(Debug, Clone, Default)]
pub struct SnapshotOptions {
    pub interactive: bool,
}

#[derive(Debug, Clone)]
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
}

#[derive(Debug, Clone)]
pub struct RefMap {
    pub refs: HashMap<String, ElementRef>,
}

impl RefMap {
    pub fn new() -> Self {
        Self {
            refs: HashMap::new(),
        }
    }

    pub fn get(&self, ref_id: &str) -> Option<&ElementRef> {
        self.refs.get(ref_id)
    }
}

impl Default for RefMap {
    fn default() -> Self {
        Self::new()
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

pub fn format_snapshot(components: &[Component], options: &SnapshotOptions) -> AccessibilitySnapshot {
    let mut refs = RefMap::new();
    let mut lines = Vec::new();
    let mut ref_counter = 0usize;
    let mut interactive_count = 0usize;

    for component in components {
        if options.interactive && !component.role.is_interactive() {
            continue;
        }

        ref_counter += 1;
        let ref_id = format!("e{}", ref_counter);

        if component.role.is_interactive() {
            interactive_count += 1;
        }

        let name = component.text_content.trim();
        let line = if name.is_empty() {
            format!("- {} [ref={}]", component.role, ref_id)
        } else {
            format!("- {} \"{}\" [ref={}]", component.role, name, ref_id)
        };
        lines.push(line);

        refs.refs.insert(
            ref_id,
            ElementRef {
                role: component.role.to_string(),
                name: if name.is_empty() {
                    None
                } else {
                    Some(name.to_string())
                },
                bounds: component.bounds.into(),
                visual_hash: component.visual_hash,
                nth: None,
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
    use crate::vom::Role;
    use uuid::Uuid;

    fn make_component(role: Role, text: &str, x: u16, y: u16, width: u16) -> Component {
        Component {
            id: Uuid::new_v4(),
            role,
            bounds: Rect::new(x, y, width, 1),
            text_content: text.to_string(),
            visual_hash: 12345,
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
        let options = SnapshotOptions { interactive: true };
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
        let options = SnapshotOptions { interactive: true };
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
        let options = SnapshotOptions { interactive: true };
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
        let options = SnapshotOptions { interactive: true };
        let snapshot = format_snapshot(&components, &options);

        assert_eq!(snapshot.stats.total, 6);
        assert_eq!(snapshot.stats.interactive, 6);
    }

    #[test]
    fn test_interactive_filter_refs_renumbered() {
        let components = vec![
            make_component(Role::StaticText, "A", 0, 0, 1),
            make_component(Role::Button, "B", 0, 1, 1),
            make_component(Role::StaticText, "C", 0, 2, 1),
            make_component(Role::Input, "D", 0, 3, 1),
        ];
        let options = SnapshotOptions { interactive: true };
        let snapshot = format_snapshot(&components, &options);

        assert!(snapshot.refs.get("e1").is_some());
        assert!(snapshot.refs.get("e2").is_some());
        assert!(snapshot.refs.get("e3").is_none());
    }
}
