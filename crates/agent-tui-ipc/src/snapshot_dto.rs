use std::collections::HashMap;

use agent_tui_core::vom::snapshot::{Bounds, ElementRef, AccessibilitySnapshot, RefMap, SnapshotStats};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundsDto {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementRefDto {
    pub role: String,
    pub name: Option<String>,
    pub bounds: BoundsDto,
    pub visual_hash: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nth: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefMapDto {
    #[serde(flatten)]
    pub refs: HashMap<String, ElementRefDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotStatsDto {
    pub total: usize,
    pub interactive: usize,
    pub lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilitySnapshotDto {
    pub tree: String,
    pub refs: RefMapDto,
    pub stats: SnapshotStatsDto,
}

impl From<Bounds> for BoundsDto {
    fn from(b: Bounds) -> Self {
        Self {
            x: b.x,
            y: b.y,
            width: b.width,
            height: b.height,
        }
    }
}

impl From<ElementRef> for ElementRefDto {
    fn from(e: ElementRef) -> Self {
        Self {
            role: e.role,
            name: e.name,
            bounds: e.bounds.into(),
            visual_hash: e.visual_hash,
            nth: e.nth,
        }
    }
}

impl From<RefMap> for RefMapDto {
    fn from(r: RefMap) -> Self {
        Self {
            refs: r.refs.into_iter().map(|(k, v)| (k, v.into())).collect(),
        }
    }
}

impl From<SnapshotStats> for SnapshotStatsDto {
    fn from(s: SnapshotStats) -> Self {
        Self {
            total: s.total,
            interactive: s.interactive,
            lines: s.lines,
        }
    }
}

impl From<AccessibilitySnapshot> for AccessibilitySnapshotDto {
    fn from(s: AccessibilitySnapshot) -> Self {
        Self {
            tree: s.tree,
            refs: s.refs.into(),
            stats: s.stats.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_dto_serialization() {
        let snapshot = AccessibilitySnapshotDto {
            tree: "- button \"OK\" [ref=e1]".to_string(),
            refs: RefMapDto {
                refs: {
                    let mut map = HashMap::new();
                    map.insert(
                        "e1".to_string(),
                        ElementRefDto {
                            role: "button".to_string(),
                            name: Some("OK".to_string()),
                            bounds: BoundsDto {
                                x: 10,
                                y: 5,
                                width: 2,
                                height: 1,
                            },
                            visual_hash: 12345,
                            nth: None,
                        },
                    );
                    map
                },
            },
            stats: SnapshotStatsDto {
                total: 1,
                interactive: 1,
                lines: 1,
            },
        };

        let json = serde_json::to_string(&snapshot).unwrap();

        assert!(json.contains("\"tree\""));
        assert!(json.contains("\"stats\""));
        assert!(json.contains("\"total\""));
        assert!(json.contains("\"interactive\""));
        assert!(json.contains("button"));
    }

    #[test]
    fn test_snapshot_dto_roundtrip() {
        let snapshot = AccessibilitySnapshotDto {
            tree: "- button \"OK\" [ref=e1]\n- input \">\" [ref=e2]".to_string(),
            refs: RefMapDto {
                refs: {
                    let mut map = HashMap::new();
                    map.insert(
                        "e1".to_string(),
                        ElementRefDto {
                            role: "button".to_string(),
                            name: Some("OK".to_string()),
                            bounds: BoundsDto {
                                x: 10,
                                y: 5,
                                width: 2,
                                height: 1,
                            },
                            visual_hash: 12345,
                            nth: None,
                        },
                    );
                    map.insert(
                        "e2".to_string(),
                        ElementRefDto {
                            role: "input".to_string(),
                            name: Some(">".to_string()),
                            bounds: BoundsDto {
                                x: 0,
                                y: 0,
                                width: 1,
                                height: 1,
                            },
                            visual_hash: 67890,
                            nth: None,
                        },
                    );
                    map
                },
            },
            stats: SnapshotStatsDto {
                total: 2,
                interactive: 2,
                lines: 2,
            },
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: AccessibilitySnapshotDto = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.tree, snapshot.tree);
        assert_eq!(restored.stats.total, snapshot.stats.total);
        assert_eq!(restored.stats.interactive, snapshot.stats.interactive);
    }

    #[test]
    fn test_refs_structure() {
        let snapshot = AccessibilitySnapshotDto {
            tree: "- button \"Submit\" [ref=e1]".to_string(),
            refs: RefMapDto {
                refs: {
                    let mut map = HashMap::new();
                    map.insert(
                        "e1".to_string(),
                        ElementRefDto {
                            role: "button".to_string(),
                            name: Some("Submit".to_string()),
                            bounds: BoundsDto {
                                x: 0,
                                y: 0,
                                width: 6,
                                height: 1,
                            },
                            visual_hash: 12345,
                            nth: None,
                        },
                    );
                    map
                },
            },
            stats: SnapshotStatsDto {
                total: 1,
                interactive: 1,
                lines: 1,
            },
        };

        let json = serde_json::to_string_pretty(&snapshot).unwrap();

        assert!(json.contains("\"e1\""));
        assert!(json.contains("\"role\": \"button\""));
        assert!(json.contains("\"name\": \"Submit\""));
        assert!(json.contains("\"bounds\""));
    }

    #[test]
    fn test_bounds_from_domain() {
        let domain_bounds = Bounds {
            x: 10,
            y: 5,
            width: 20,
            height: 3,
        };
        let dto: BoundsDto = domain_bounds.into();

        assert_eq!(dto.x, 10);
        assert_eq!(dto.y, 5);
        assert_eq!(dto.width, 20);
        assert_eq!(dto.height, 3);
    }

    #[test]
    fn test_element_ref_from_domain() {
        let domain_ref = ElementRef {
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
        };
        let dto: ElementRefDto = domain_ref.into();

        assert_eq!(dto.role, "button");
        assert_eq!(dto.name, Some("OK".to_string()));
        assert_eq!(dto.bounds.x, 5);
        assert_eq!(dto.visual_hash, 12345);
        assert_eq!(dto.nth, Some(2));
    }

    #[test]
    fn test_refmap_from_domain() {
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
            },
        );
        let domain_refmap = RefMap { refs };
        let dto: RefMapDto = domain_refmap.into();

        assert!(dto.refs.contains_key("e1"));
        assert_eq!(dto.refs["e1"].role, "input");
    }

    #[test]
    fn test_enhanced_snapshot_from_domain() {
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
            },
        );
        let domain_snapshot = AccessibilitySnapshot {
            tree: "- button \"Submit\" [ref=e1]".to_string(),
            refs: RefMap { refs },
            stats: SnapshotStats {
                total: 1,
                interactive: 1,
                lines: 1,
            },
        };
        let dto: AccessibilitySnapshotDto = domain_snapshot.into();

        assert_eq!(dto.tree, "- button \"Submit\" [ref=e1]");
        assert_eq!(dto.stats.total, 1);
        assert!(dto.refs.refs.contains_key("e1"));
    }

    #[test]
    fn test_domain_to_dto_json_serialization() {
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
            },
        );
        let domain_snapshot = AccessibilitySnapshot {
            tree: "- checkbox \"[x] Enabled\" [ref=e1]".to_string(),
            refs: RefMap { refs },
            stats: SnapshotStats {
                total: 1,
                interactive: 1,
                lines: 1,
            },
        };
        let dto: AccessibilitySnapshotDto = domain_snapshot.into();
        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"tree\""));
        assert!(json.contains("checkbox"));
        assert!(json.contains("\"e1\""));
        assert!(json.contains("\"visual_hash\":99999"));
    }
}
