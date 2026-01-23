use agent_tui_core::vom::snapshot::{
    AccessibilitySnapshot, Bounds, ElementRef, RefMap, SnapshotStats,
};
use agent_tui_ipc::{
    AccessibilitySnapshotDto, BoundsDto, ElementRefDto, RefMapDto, SnapshotStatsDto,
};

pub fn bounds_to_dto(bounds: &Bounds) -> BoundsDto {
    BoundsDto {
        x: bounds.x,
        y: bounds.y,
        width: bounds.width,
        height: bounds.height,
    }
}

pub fn element_ref_to_dto(element: ElementRef) -> ElementRefDto {
    ElementRefDto {
        role: element.role,
        name: element.name,
        bounds: bounds_to_dto(&element.bounds),
        visual_hash: element.visual_hash,
        nth: element.nth,
        selected: element.selected,
    }
}

pub fn ref_map_to_dto(ref_map: RefMap) -> RefMapDto {
    RefMapDto {
        refs: ref_map
            .refs
            .into_iter()
            .map(|(k, v)| (k, element_ref_to_dto(v)))
            .collect(),
    }
}

pub fn stats_to_dto(stats: &SnapshotStats) -> SnapshotStatsDto {
    SnapshotStatsDto {
        total: stats.total,
        interactive: stats.interactive,
        lines: stats.lines,
    }
}

pub fn snapshot_to_dto(snapshot: AccessibilitySnapshot) -> AccessibilitySnapshotDto {
    AccessibilitySnapshotDto {
        tree: snapshot.tree,
        refs: ref_map_to_dto(snapshot.refs),
        stats: stats_to_dto(&snapshot.stats),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_bounds_to_dto_conversion() {
        let core_bounds = Bounds {
            x: 10,
            y: 5,
            width: 20,
            height: 3,
        };
        let dto = bounds_to_dto(&core_bounds);

        assert_eq!(dto.x, 10);
        assert_eq!(dto.y, 5);
        assert_eq!(dto.width, 20);
        assert_eq!(dto.height, 3);
    }

    #[test]
    fn test_element_ref_to_dto_conversion() {
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
        let dto = element_ref_to_dto(core_ref);

        assert_eq!(dto.role, "button");
        assert_eq!(dto.name, Some("OK".to_string()));
        assert_eq!(dto.bounds.x, 5);
        assert_eq!(dto.visual_hash, 12345);
        assert_eq!(dto.nth, Some(2));
    }

    #[test]
    fn test_element_ref_to_dto_with_none_name() {
        let core_ref = ElementRef {
            role: "panel".to_string(),
            name: None,
            bounds: Bounds {
                x: 0,
                y: 0,
                width: 10,
                height: 5,
            },
            visual_hash: 99999,
            nth: None,
            selected: false,
        };
        let dto = element_ref_to_dto(core_ref);

        assert_eq!(dto.role, "panel");
        assert_eq!(dto.name, None);
        assert_eq!(dto.nth, None);
    }

    #[test]
    fn test_ref_map_to_dto_conversion() {
        let mut refs = HashMap::new();
        refs.insert(
            "e1".to_string(),
            ElementRef {
                role: "input".to_string(),
                name: None,
                bounds: Bounds {
                    x: 0,
                    y: 0,
                    width: 10,
                    height: 1,
                },
                visual_hash: 111,
                nth: None,
                selected: false,
            },
        );
        let core_refmap = RefMap { refs };
        let dto = ref_map_to_dto(core_refmap);

        assert!(dto.refs.contains_key("e1"));
        assert_eq!(dto.refs["e1"].role, "input");
    }

    #[test]
    fn test_ref_map_to_dto_multiple_entries() {
        let mut refs = HashMap::new();
        refs.insert(
            "e1".to_string(),
            ElementRef {
                role: "button".to_string(),
                name: Some("OK".to_string()),
                bounds: Bounds {
                    x: 0,
                    y: 0,
                    width: 2,
                    height: 1,
                },
                visual_hash: 111,
                nth: None,
                selected: false,
            },
        );
        refs.insert(
            "e2".to_string(),
            ElementRef {
                role: "input".to_string(),
                name: Some(">".to_string()),
                bounds: Bounds {
                    x: 5,
                    y: 0,
                    width: 10,
                    height: 1,
                },
                visual_hash: 222,
                nth: None,
                selected: false,
            },
        );
        let core_refmap = RefMap { refs };
        let dto = ref_map_to_dto(core_refmap);

        assert_eq!(dto.refs.len(), 2);
        assert!(dto.refs.contains_key("e1"));
        assert!(dto.refs.contains_key("e2"));
    }

    #[test]
    fn test_stats_to_dto_conversion() {
        let core_stats = SnapshotStats {
            total: 10,
            interactive: 5,
            lines: 10,
        };
        let dto = stats_to_dto(&core_stats);

        assert_eq!(dto.total, 10);
        assert_eq!(dto.interactive, 5);
        assert_eq!(dto.lines, 10);
    }

    #[test]
    fn test_snapshot_to_dto_conversion() {
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
        let dto = snapshot_to_dto(core_snapshot);

        assert_eq!(dto.tree, "- button \"Submit\" [ref=e1]");
        assert_eq!(dto.stats.total, 1);
        assert!(dto.refs.refs.contains_key("e1"));
    }

    #[test]
    fn test_snapshot_to_dto_json_serialization() {
        let mut refs = HashMap::new();
        refs.insert(
            "e1".to_string(),
            ElementRef {
                role: "checkbox".to_string(),
                name: Some("[x] Enabled".to_string()),
                bounds: Bounds {
                    x: 5,
                    y: 10,
                    width: 12,
                    height: 1,
                },
                visual_hash: 99999,
                nth: None,
                selected: false,
            },
        );
        let core_snapshot = AccessibilitySnapshot {
            tree: "- checkbox \"[x] Enabled\" [ref=e1]".to_string(),
            refs: RefMap { refs },
            stats: SnapshotStats {
                total: 1,
                interactive: 1,
                lines: 1,
            },
        };
        let dto = snapshot_to_dto(core_snapshot);
        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"tree\""));
        assert!(json.contains("checkbox"));
        assert!(json.contains("\"e1\""));
        assert!(json.contains("\"visual_hash\":99999"));
    }
}
