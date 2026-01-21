//! AI-friendly error messages with actionable hints

use super::rpc_types::Response;

/// Convert technical errors into AI-friendly messages with actionable hints.
/// This follows agent-browser's pattern of always suggesting a next action.
pub fn ai_friendly_error(error: &str, context: Option<&str>) -> String {
    let ctx = context.unwrap_or("unknown");

    // Element not found errors
    if error.contains("not found") || error.contains("NotFound") {
        return format!(
            "Element \"{}\" not found. Run 'snapshot -i' to see current elements and their refs.",
            ctx
        );
    }

    // Session not found errors
    if error.contains("Session not found") || error.contains("No active session") {
        return "No active session. Run 'sessions' to list active sessions or 'spawn <cmd>' to start a new one.".to_string();
    }

    // Timeout errors
    if error.contains("timeout") || error.contains("Timeout") {
        return "Timeout waiting for condition. The app may still be loading. Try 'wait --stable' or increase timeout with '-t'.".to_string();
    }

    // Lock acquisition errors
    if error.contains("lock") {
        return "Session is busy. Try again in a moment, or run 'sessions' to check session status.".to_string();
    }

    // Invalid key errors
    if error.contains("Invalid key") {
        return format!(
            "Invalid key '{}'. Supported keys: Enter, Tab, Escape, Backspace, Delete, ArrowUp/Down/Left/Right, Home, End, PageUp/Down, F1-F12. Modifiers: Ctrl+, Alt+, Shift+",
            ctx
        );
    }

    // Element type mismatch
    if error.contains("not toggleable") || error.contains("not a select") {
        return format!(
            "Element {} cannot perform this action. Run 'snapshot -i' to see element types.",
            ctx
        );
    }

    // PTY errors
    if error.contains("PTY") || error.contains("Pty") {
        return "Terminal communication error. The session may have ended. Run 'sessions' to check status.".to_string();
    }

    // Default: return original error with generic hint
    format!("{}. Run 'snapshot -i' to see current screen state.", error)
}

/// Create a standardized lock timeout error response
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
