use super::rpc_types::Response;

pub fn ai_friendly_error(error: &str, context: Option<&str>) -> String {
    let ctx = context.unwrap_or("unknown");

    if error.contains("not found") || error.contains("NotFound") {
        return format!(
            "Element \"{}\" not found. Run 'snapshot -i' to see current elements and their refs.",
            ctx
        );
    }

    if error.contains("Session not found") || error.contains("No active session") {
        return "No active session. Run 'sessions' to list active sessions or 'spawn <cmd>' to start a new one.".to_string();
    }

    if error.contains("timeout") || error.contains("Timeout") {
        return "Timeout waiting for condition. The app may still be loading. Try 'wait --stable' or increase timeout with '-t'.".to_string();
    }

    if error.contains("lock") {
        return "Session is busy. Try again in a moment, or run 'sessions' to check session status.".to_string();
    }

    if error.contains("Invalid key") {
        return format!(
            "Invalid key '{}'. Supported keys: Enter, Tab, Escape, Backspace, Delete, ArrowUp/Down/Left/Right, Home, End, PageUp/Down, F1-F12. Modifiers: Ctrl+, Alt+, Shift+",
            ctx
        );
    }

    if error.contains("not toggleable") || error.contains("not a select") {
        return format!(
            "Element {} is not the right type for this action. Run 'snapshot -i' to see element types and try 'click' for menuitems.",
            ctx
        );
    }

    if error.contains("PTY") || error.contains("Pty") {
        return "Terminal communication error. The session may have ended. Run 'sessions' to check status.".to_string();
    }

    format!("{}. Run 'snapshot -i' to see current screen state.", error)
}

pub fn lock_timeout_response(request_id: u64, session_context: Option<&str>) -> Response {
    let msg = match session_context {
        Some(sid) => format!(
            "Session '{}': {}",
            sid,
            ai_friendly_error("lock timeout", None)
        ),
        None => ai_friendly_error("lock timeout", None),
    };
    Response::error(request_id, -32000, &msg)
}
