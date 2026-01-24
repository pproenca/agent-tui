use crate::ipc::{AccessibilitySnapshotDto, BoundsDto, ElementRefDto, RefMapDto, SnapshotStatsDto};

use crate::daemon::domain::{
    DomainAccessibilitySnapshot, DomainBounds, DomainElementRef, DomainRefMap, DomainSnapshotStats,
};

pub fn bounds_to_dto(bounds: &DomainBounds) -> BoundsDto {
    BoundsDto {
        x: bounds.x(),
        y: bounds.y(),
        width: bounds.width(),
        height: bounds.height(),
    }
}

pub fn element_ref_to_dto(element: &DomainElementRef) -> ElementRefDto {
    ElementRefDto {
        role: element.role.to_string(),
        name: element.name.clone(),
        bounds: bounds_to_dto(&element.bounds),
        visual_hash: element.visual_hash,
        nth: element.nth,
        selected: element.selected,
    }
}

pub fn ref_map_to_dto(ref_map: &DomainRefMap) -> RefMapDto {
    RefMapDto {
        refs: ref_map
            .refs
            .iter()
            .map(|(k, v)| (k.clone(), element_ref_to_dto(v)))
            .collect(),
    }
}

pub fn stats_to_dto(stats: &DomainSnapshotStats) -> SnapshotStatsDto {
    SnapshotStatsDto {
        total: stats.total,
        interactive: stats.interactive,
        lines: stats.lines,
    }
}

pub fn snapshot_to_dto(snapshot: &DomainAccessibilitySnapshot) -> AccessibilitySnapshotDto {
    AccessibilitySnapshotDto {
        tree: snapshot.tree.clone(),
        refs: ref_map_to_dto(&snapshot.refs),
        stats: stats_to_dto(&snapshot.stats),
    }
}

/// Converts a Domain snapshot into a DTO, consuming the input.
///
/// Use this variant when ownership can be transferred to avoid cloning strings.
pub fn snapshot_into_dto(snapshot: DomainAccessibilitySnapshot) -> AccessibilitySnapshotDto {
    AccessibilitySnapshotDto {
        tree: snapshot.tree,
        refs: ref_map_into_dto(snapshot.refs),
        stats: stats_to_dto(&snapshot.stats),
    }
}

fn ref_map_into_dto(ref_map: DomainRefMap) -> RefMapDto {
    RefMapDto {
        refs: ref_map
            .refs
            .into_iter()
            .map(|(k, v)| (k, element_ref_into_dto(v)))
            .collect(),
    }
}

fn element_ref_into_dto(element: DomainElementRef) -> ElementRefDto {
    ElementRefDto {
        role: element.role.to_string(),
        name: element.name,
        bounds: bounds_to_dto(&element.bounds),
        visual_hash: element.visual_hash,
        nth: element.nth,
        selected: element.selected,
    }
}

use crate::daemon::domain::session_types::SessionInfo;

/// Convert SessionInfo to JSON representation.
///
/// This adapter function handles serialization at the boundary,
/// keeping the domain SessionInfo free of framework dependencies.
pub fn session_info_to_json(info: &SessionInfo) -> serde_json::Value {
    serde_json::json!({
        "id": info.id.as_str(),
        "command": info.command,
        "pid": info.pid,
        "running": info.running,
        "created_at": info.created_at,
        "size": { "cols": info.size.0, "rows": info.size.1 }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::domain::DomainRole;
    use std::collections::HashMap;

    #[test]
    fn test_bounds_to_dto_conversion() {
        let bounds = DomainBounds::new(10, 5, 20, 3).expect("valid bounds");
        let dto = bounds_to_dto(&bounds);

        assert_eq!(dto.x, 10);
        assert_eq!(dto.y, 5);
        assert_eq!(dto.width, 20);
        assert_eq!(dto.height, 3);
    }

    #[test]
    fn test_element_ref_to_dto_conversion() {
        let elem_ref = DomainElementRef {
            role: DomainRole::Button,
            name: Some("OK".to_string()),
            bounds: DomainBounds::new(5, 10, 4, 1).expect("valid bounds"),
            visual_hash: 12345,
            nth: Some(2),
            selected: false,
        };
        let dto = element_ref_to_dto(&elem_ref);

        assert_eq!(dto.role, "button");
        assert_eq!(dto.name, Some("OK".to_string()));
        assert_eq!(dto.bounds.x, 5);
        assert_eq!(dto.visual_hash, 12345);
        assert_eq!(dto.nth, Some(2));
    }

    #[test]
    fn test_element_ref_to_dto_with_none_name() {
        let elem_ref = DomainElementRef {
            role: DomainRole::Panel,
            name: None,
            bounds: DomainBounds::new(0, 0, 10, 5).expect("valid bounds"),
            visual_hash: 99999,
            nth: None,
            selected: false,
        };
        let dto = element_ref_to_dto(&elem_ref);

        assert_eq!(dto.role, "panel");
        assert_eq!(dto.name, None);
        assert_eq!(dto.nth, None);
    }

    #[test]
    fn test_ref_map_to_dto_conversion() {
        let mut refs = HashMap::new();
        refs.insert(
            "e1".to_string(),
            DomainElementRef {
                role: DomainRole::Input,
                name: None,
                bounds: DomainBounds::new(0, 0, 10, 1).expect("valid bounds"),
                visual_hash: 111,
                nth: None,
                selected: false,
            },
        );
        let refmap = DomainRefMap { refs };
        let dto = ref_map_to_dto(&refmap);

        assert!(dto.refs.contains_key("e1"));
        assert_eq!(dto.refs["e1"].role, "input");
    }

    #[test]
    fn test_ref_map_to_dto_multiple_entries() {
        let mut refs = HashMap::new();
        refs.insert(
            "e1".to_string(),
            DomainElementRef {
                role: DomainRole::Button,
                name: Some("OK".to_string()),
                bounds: DomainBounds::new(0, 0, 2, 1).expect("valid bounds"),
                visual_hash: 111,
                nth: None,
                selected: false,
            },
        );
        refs.insert(
            "e2".to_string(),
            DomainElementRef {
                role: DomainRole::Input,
                name: Some(">".to_string()),
                bounds: DomainBounds::new(5, 0, 10, 1).expect("valid bounds"),
                visual_hash: 222,
                nth: None,
                selected: false,
            },
        );
        let refmap = DomainRefMap { refs };
        let dto = ref_map_to_dto(&refmap);

        assert_eq!(dto.refs.len(), 2);
        assert!(dto.refs.contains_key("e1"));
        assert!(dto.refs.contains_key("e2"));
    }

    #[test]
    fn test_stats_to_dto_conversion() {
        let stats = DomainSnapshotStats {
            total: 10,
            interactive: 5,
            lines: 10,
        };
        let dto = stats_to_dto(&stats);

        assert_eq!(dto.total, 10);
        assert_eq!(dto.interactive, 5);
        assert_eq!(dto.lines, 10);
    }

    #[test]
    fn test_snapshot_to_dto_conversion() {
        let mut refs = HashMap::new();
        refs.insert(
            "e1".to_string(),
            DomainElementRef {
                role: DomainRole::Button,
                name: Some("Submit".to_string()),
                bounds: DomainBounds::new(0, 0, 6, 1).expect("valid bounds"),
                visual_hash: 12345,
                nth: None,
                selected: false,
            },
        );
        let snapshot = DomainAccessibilitySnapshot {
            tree: "- button \"Submit\" [ref=e1]".to_string(),
            refs: DomainRefMap { refs },
            stats: DomainSnapshotStats {
                total: 1,
                interactive: 1,
                lines: 1,
            },
        };
        let dto = snapshot_to_dto(&snapshot);

        assert_eq!(dto.tree, "- button \"Submit\" [ref=e1]");
        assert_eq!(dto.stats.total, 1);
        assert!(dto.refs.refs.contains_key("e1"));
    }

    #[test]
    fn test_snapshot_to_dto_json_serialization() {
        let mut refs = HashMap::new();
        refs.insert(
            "e1".to_string(),
            DomainElementRef {
                role: DomainRole::Checkbox,
                name: Some("[x] Enabled".to_string()),
                bounds: DomainBounds::new(5, 10, 12, 1).expect("valid bounds"),
                visual_hash: 99999,
                nth: None,
                selected: false,
            },
        );
        let snapshot = DomainAccessibilitySnapshot {
            tree: "- checkbox \"[x] Enabled\" [ref=e1]".to_string(),
            refs: DomainRefMap { refs },
            stats: DomainSnapshotStats {
                total: 1,
                interactive: 1,
                lines: 1,
            },
        };
        let dto = snapshot_to_dto(&snapshot);
        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"tree\""));
        assert!(json.contains("checkbox"));
        assert!(json.contains("\"e1\""));
        assert!(json.contains("\"visual_hash\":99999"));
    }

    #[test]
    fn test_session_info_to_json() {
        use crate::daemon::domain::session_types::{SessionId, SessionInfo};

        let info = SessionInfo {
            id: SessionId::new("test"),
            command: "bash".to_string(),
            pid: 1234,
            running: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            size: (80, 24),
        };
        let json = session_info_to_json(&info);
        assert_eq!(json["id"], "test");
        assert_eq!(json["command"], "bash");
        assert_eq!(json["pid"], 1234);
        assert_eq!(json["running"], true);
        assert_eq!(json["size"]["cols"], 80);
        assert_eq!(json["size"]["rows"], 24);
    }
}
