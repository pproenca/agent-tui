//! Snapshot adapter helpers.

use crate::domain::session_types::SessionInfo;

pub(crate) fn session_info_to_json(info: &SessionInfo) -> serde_json::Value {
    serde_json::json!({
        "id": info.id.as_str(),
        "command": info.command,
        "pid": info.pid,
        "running": info.running,
        "created_at": info.created_at,
        "size": { "cols": info.size.cols(), "rows": info.size.rows() }
    })
}
