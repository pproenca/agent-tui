use crate::domain::core::CursorPosition;
use crate::domain::core::vom::snapshot::{AccessibilitySnapshot, SnapshotStats};

use crate::domain::{DomainAccessibilitySnapshot, DomainCursorPosition, DomainSnapshotStats};

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
        stats: core_stats_to_domain(&snapshot.stats),
    }
}

pub fn core_snapshot_into_domain(snapshot: AccessibilitySnapshot) -> DomainAccessibilitySnapshot {
    DomainAccessibilitySnapshot {
        tree: snapshot.tree,
        stats: core_stats_to_domain(&snapshot.stats),
    }
}

pub fn core_cursor_to_domain(cursor: &CursorPosition) -> DomainCursorPosition {
    DomainCursorPosition {
        row: cursor.row,
        col: cursor.col,
        visible: cursor.visible,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_stats_to_domain() {
        let core_stats = SnapshotStats {
            total: 10,
            interactive: 3,
            lines: 10,
        };

        let domain = core_stats_to_domain(&core_stats);

        assert_eq!(domain.total, 10);
        assert_eq!(domain.interactive, 3);
        assert_eq!(domain.lines, 10);
    }

    #[test]
    fn test_core_snapshot_to_domain() {
        let core_snapshot = AccessibilitySnapshot {
            tree: "root".to_string(),
            stats: SnapshotStats {
                total: 2,
                interactive: 1,
                lines: 2,
            },
        };

        let domain = core_snapshot_to_domain(&core_snapshot);

        assert_eq!(domain.tree, "root".to_string());
        assert_eq!(domain.stats.total, 2);
        assert_eq!(domain.stats.interactive, 1);
        assert_eq!(domain.stats.lines, 2);
    }
}
