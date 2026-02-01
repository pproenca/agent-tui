use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SnapshotStatsDto {
    pub total: usize,
    pub interactive: usize,
    pub lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilitySnapshotDto {
    pub tree: String,
    pub stats: SnapshotStatsDto,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_dto_serialization() {
        let snapshot = AccessibilitySnapshotDto {
            tree: "- button \"OK\"".to_string(),
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
            tree: "- button \"OK\"\n- input \">\"".to_string(),
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
}
