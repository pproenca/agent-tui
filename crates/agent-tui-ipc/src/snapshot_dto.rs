use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Returns true if the boolean is false (used by serde skip_serializing_if).
#[inline]
fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BoundsDto {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementRefDto {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub bounds: BoundsDto,
    pub visual_hash: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nth: Option<usize>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefMapDto {
    #[serde(flatten)]
    pub refs: HashMap<String, ElementRefDto>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounds_dto_independent_creation() {
        let bounds = BoundsDto {
            x: 10,
            y: 5,
            width: 20,
            height: 3,
        };
        assert_eq!(bounds.x, 10);
        assert_eq!(bounds.y, 5);
        assert_eq!(bounds.width, 20);
        assert_eq!(bounds.height, 3);
    }

    #[test]
    fn test_element_ref_dto_independent_creation() {
        let element = ElementRefDto {
            role: "button".to_string(),
            name: Some("OK".to_string()),
            bounds: BoundsDto {
                x: 5,
                y: 10,
                width: 4,
                height: 1,
            },
            visual_hash: 12345,
            nth: Some(2),
            selected: false,
        };
        assert_eq!(element.role, "button");
        assert_eq!(element.name, Some("OK".to_string()));
        assert_eq!(element.bounds.x, 5);
        assert_eq!(element.visual_hash, 12345);
        assert_eq!(element.nth, Some(2));
    }

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
                            selected: false,
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
                            selected: false,
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
                            selected: false,
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
                            selected: false,
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
}
