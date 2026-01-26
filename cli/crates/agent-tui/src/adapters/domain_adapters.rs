use crate::domain::core::vom::snapshot::{
    AccessibilitySnapshot, Bounds, ElementRef, RefMap, SnapshotStats,
};
use crate::domain::core::{CursorPosition, Element, ElementType, Position};

use crate::domain::{
    DomainAccessibilitySnapshot, DomainBounds, DomainCursorPosition, DomainElement,
    DomainElementRef, DomainElementType, DomainPosition, DomainRefMap, DomainRole,
    DomainSnapshotStats,
};

pub fn string_to_domain_role(role: &str) -> DomainRole {
    match role {
        "button" => DomainRole::Button,
        "tab" => DomainRole::Tab,
        "input" => DomainRole::Input,
        "text" => DomainRole::StaticText,
        "panel" => DomainRole::Panel,
        "checkbox" => DomainRole::Checkbox,
        "menuitem" => DomainRole::MenuItem,
        "status" => DomainRole::Status,
        "toolblock" => DomainRole::ToolBlock,
        "prompt" => DomainRole::PromptMarker,
        "progressbar" => DomainRole::ProgressBar,
        "link" => DomainRole::Link,
        "error" => DomainRole::ErrorMessage,
        "diff" => DomainRole::DiffLine,
        "codeblock" => DomainRole::CodeBlock,
        _ => DomainRole::StaticText,
    }
}

pub fn core_bounds_to_domain(bounds: &Bounds) -> DomainBounds {
    DomainBounds::new_unchecked(bounds.x, bounds.y, bounds.width, bounds.height)
}

pub fn core_element_ref_to_domain(element: &ElementRef) -> DomainElementRef {
    DomainElementRef {
        role: string_to_domain_role(&element.role),
        name: element.name.clone(),
        bounds: core_bounds_to_domain(&element.bounds),
        visual_hash: element.visual_hash,
        nth: element.nth,
        selected: element.selected,
    }
}

pub fn core_ref_map_to_domain(ref_map: &RefMap) -> DomainRefMap {
    DomainRefMap {
        refs: ref_map
            .refs
            .iter()
            .map(|(k, v)| (k.clone(), core_element_ref_to_domain(v)))
            .collect(),
    }
}

pub fn core_stats_to_domain(stats: &SnapshotStats) -> DomainSnapshotStats {
    DomainSnapshotStats {
        total: stats.total,
        interactive: stats.interactive,
        lines: stats.lines,
    }
}

pub fn core_snapshot_to_domain(snapshot: &AccessibilitySnapshot) -> DomainAccessibilitySnapshot {
    DomainAccessibilitySnapshot {
        tree: snapshot.tree.clone(),
        refs: core_ref_map_to_domain(&snapshot.refs),
        stats: core_stats_to_domain(&snapshot.stats),
    }
}

pub fn core_snapshot_into_domain(snapshot: AccessibilitySnapshot) -> DomainAccessibilitySnapshot {
    DomainAccessibilitySnapshot {
        tree: snapshot.tree,
        refs: core_ref_map_into_domain(snapshot.refs),
        stats: core_stats_to_domain(&snapshot.stats),
    }
}

fn core_ref_map_into_domain(ref_map: RefMap) -> DomainRefMap {
    DomainRefMap {
        refs: ref_map
            .refs
            .into_iter()
            .map(|(k, v)| (k, core_element_ref_into_domain(v)))
            .collect(),
    }
}

fn core_element_ref_into_domain(element: ElementRef) -> DomainElementRef {
    DomainElementRef {
        role: string_to_domain_role(&element.role),
        name: element.name,
        bounds: core_bounds_to_domain(&element.bounds),
        visual_hash: element.visual_hash,
        nth: element.nth,
        selected: element.selected,
    }
}

pub fn core_cursor_to_domain(cursor: &CursorPosition) -> DomainCursorPosition {
    DomainCursorPosition {
        row: cursor.row,
        col: cursor.col,
        visible: cursor.visible,
    }
}

pub fn core_position_to_domain(pos: &Position) -> DomainPosition {
    DomainPosition {
        row: pos.row,
        col: pos.col,
        width: pos.width,
        height: pos.height,
    }
}

pub fn core_element_type_to_domain(et: &ElementType) -> DomainElementType {
    match et {
        ElementType::Button => DomainElementType::Button,
        ElementType::Input => DomainElementType::Input,
        ElementType::Checkbox => DomainElementType::Checkbox,
        ElementType::Radio => DomainElementType::Radio,
        ElementType::Select => DomainElementType::Select,
        ElementType::MenuItem => DomainElementType::MenuItem,
        ElementType::ListItem => DomainElementType::ListItem,
        ElementType::Spinner => DomainElementType::Spinner,
        ElementType::Progress => DomainElementType::Progress,
        ElementType::Link => DomainElementType::Link,
    }
}

pub fn core_element_to_domain(el: &Element) -> DomainElement {
    DomainElement {
        element_ref: el.element_ref.clone(),
        element_type: core_element_type_to_domain(&el.element_type),
        label: el.label.clone(),
        value: el.value.clone(),
        position: core_position_to_domain(&el.position),
        focused: el.focused,
        selected: el.selected,
        checked: el.checked,
        disabled: el.disabled,
        hint: el.hint.clone(),
    }
}

pub fn core_elements_to_domain(elements: &[Element]) -> Vec<DomainElement> {
    elements.iter().map(core_element_to_domain).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_core_bounds_to_domain() {
        let core_bounds = Bounds {
            x: 10,
            y: 5,
            width: 20,
            height: 3,
        };
        let domain = core_bounds_to_domain(&core_bounds);

        assert_eq!(domain.x(), 10);
        assert_eq!(domain.y(), 5);
        assert_eq!(domain.width(), 20);
        assert_eq!(domain.height(), 3);
    }

    #[test]
    fn test_core_element_ref_to_domain() {
        let core_ref = ElementRef {
            role: "button".to_string(),
            name: Some("OK".to_string()),
            bounds: Bounds {
                x: 5,
                y: 10,
                width: 4,
                height: 1,
            },
            visual_hash: 12345,
            nth: Some(2),
            selected: false,
        };
        let domain = core_element_ref_to_domain(&core_ref);

        assert_eq!(domain.role, DomainRole::Button);
        assert_eq!(domain.name, Some("OK".to_string()));
        assert_eq!(domain.bounds.x(), 5);
        assert_eq!(domain.visual_hash, 12345);
        assert_eq!(domain.nth, Some(2));
        assert!(!domain.selected);
    }

    #[test]
    fn test_core_snapshot_to_domain() {
        let mut refs = HashMap::new();
        refs.insert(
            "e1".to_string(),
            ElementRef {
                role: "button".to_string(),
                name: Some("Submit".to_string()),
                bounds: Bounds {
                    x: 0,
                    y: 0,
                    width: 6,
                    height: 1,
                },
                visual_hash: 12345,
                nth: None,
                selected: false,
            },
        );
        let core_snapshot = AccessibilitySnapshot {
            tree: "- button \"Submit\" [ref=e1]".to_string(),
            refs: RefMap { refs },
            stats: SnapshotStats {
                total: 1,
                interactive: 1,
                lines: 1,
            },
        };
        let domain = core_snapshot_to_domain(&core_snapshot);

        assert_eq!(domain.tree, "- button \"Submit\" [ref=e1]");
        assert_eq!(domain.stats.total, 1);
        assert!(domain.refs.get("e1").is_some());
    }
}
