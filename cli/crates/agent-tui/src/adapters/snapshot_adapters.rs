use crate::adapters::ipc::{AccessibilitySnapshotDto, SnapshotStatsDto};

use crate::domain::{DomainAccessibilitySnapshot, DomainSnapshotStats};

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
        stats: stats_to_dto(&snapshot.stats),
    }
}

pub fn snapshot_into_dto(snapshot: DomainAccessibilitySnapshot) -> AccessibilitySnapshotDto {
    AccessibilitySnapshotDto {
        tree: snapshot.tree,
        stats: stats_to_dto(&snapshot.stats),
    }
}

use crate::domain::session_types::SessionInfo;

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

    #[test]
    fn test_stats_to_dto_conversion() {
        let stats = DomainSnapshotStats {
            total: 10,
            interactive: 4,
            lines: 10,
        };
        let dto = stats_to_dto(&stats);

        assert_eq!(dto.total, 10);
        assert_eq!(dto.interactive, 4);
        assert_eq!(dto.lines, 10);
    }

    #[test]
    fn test_snapshot_to_dto_conversion() {
        let snapshot = DomainAccessibilitySnapshot {
            tree: "- button \"OK\"".to_string(),
            stats: DomainSnapshotStats {
                total: 1,
                interactive: 1,
                lines: 1,
            },
        };

        let dto = snapshot_to_dto(&snapshot);

        assert_eq!(dto.tree, snapshot.tree);
        assert_eq!(dto.stats.total, 1);
        assert_eq!(dto.stats.interactive, 1);
        assert_eq!(dto.stats.lines, 1);
    }
}
