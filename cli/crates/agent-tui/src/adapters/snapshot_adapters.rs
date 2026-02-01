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
