use agent_tui_core::vom::snapshot::{
    AccessibilitySnapshot, Bounds, ElementRef, RefMap, SnapshotStats,
};

use crate::domain::{
    DomainAccessibilitySnapshot, DomainBounds, DomainElementRef, DomainRefMap, DomainSnapshotStats,
};

pub fn core_bounds_to_domain(bounds: &Bounds) -> DomainBounds {
    DomainBounds {
        x: bounds.x,
        y: bounds.y,
        width: bounds.width,
        height: bounds.height,
    }
}

pub fn core_element_ref_to_domain(element: &ElementRef) -> DomainElementRef {
    DomainElementRef {
        role: element.role.clone(),
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

        assert_eq!(domain.x, 10);
        assert_eq!(domain.y, 5);
        assert_eq!(domain.width, 20);
        assert_eq!(domain.height, 3);
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

        assert_eq!(domain.role, "button");
        assert_eq!(domain.name, Some("OK".to_string()));
        assert_eq!(domain.bounds.x, 5);
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
