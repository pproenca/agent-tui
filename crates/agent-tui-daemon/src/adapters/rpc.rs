use agent_tui_ipc::{RpcRequest, RpcResponse, params};
use serde_json::{Value, json};

use super::snapshot_adapters::session_info_to_json;
use crate::domain::{
    ClickInput, CountInput, CountOutput, DomainElement, FillInput, FindInput, KeystrokeInput,
    KillOutput, ResizeInput, ResizeOutput, ScrollInput, ScrollOutput, SessionId, SessionsOutput,
    SnapshotInput, SnapshotOutput, SpawnInput, SpawnOutput, TypeInput, WaitInput, WaitOutput,
};
use crate::error::{DomainError, SessionError};
use crate::usecases::{AttachOutput, RestartOutput};

/// Convert an optional string session ID to an optional SessionId.
///
/// This handles the conversion from IPC layer strings to domain SessionId:
/// - None -> None (use active session)
/// - Some("") or whitespace -> None (treat empty as unspecified)
/// - Some(id) -> Some(SessionId::new(id))
pub fn parse_session_id(session: Option<String>) -> Option<SessionId> {
    session.and_then(|s| {
        if s.trim().is_empty() {
            None
        } else {
            Some(SessionId::new(s))
        }
    })
}

const MAX_TERMINAL_COLS: u16 = 500;
const MAX_TERMINAL_ROWS: u16 = 200;
const MIN_TERMINAL_COLS: u16 = 10;
const MIN_TERMINAL_ROWS: u16 = 5;

/// Convert a DomainElement to JSON representation.
pub fn element_to_json(el: &DomainElement) -> Value {
    json!({
        "ref": el.element_ref,
        "type": el.element_type.as_str(),
        "label": el.label,
        "value": el.value,
        "position": {
            "row": el.position.row,
            "col": el.position.col,
            "width": el.position.width,
            "height": el.position.height
        },
        "focused": el.focused,
        "selected": el.selected,
        "checked": el.checked,
        "disabled": el.disabled,
        "hint": el.hint
    })
}

/// Convert a DomainError to an RpcResponse.
pub fn domain_error_response(id: u64, err: &DomainError) -> RpcResponse {
    RpcResponse::domain_error(
        id,
        err.code(),
        &err.to_string(),
        err.category().as_str(),
        Some(err.context()),
        Some(err.suggestion()),
    )
}

/// Convert a SessionError to an RpcResponse.
pub fn session_error_response(id: u64, err: SessionError) -> RpcResponse {
    domain_error_response(id, &DomainError::from(err))
}

/// Create a lock timeout error response.
pub fn lock_timeout_response(id: u64, session_id: Option<&str>) -> RpcResponse {
    let err = DomainError::LockTimeout {
        session_id: session_id.map(String::from),
    };
    domain_error_response(id, &err)
}

/// Parse SpawnInput from RpcRequest using shared params type.
#[allow(clippy::result_large_err)]
pub fn parse_spawn_input(request: &RpcRequest) -> Result<SpawnInput, RpcResponse> {
    let rpc_params: params::SpawnParams = request
        .params
        .as_ref()
        .ok_or_else(|| RpcResponse::error(request.id, -32602, "Missing params"))
        .and_then(|p| {
            serde_json::from_value(p.clone()).map_err(|e| {
                RpcResponse::error(request.id, -32602, &format!("Invalid params: {}", e))
            })
        })?;

    // Use "bash" as default if command is empty
    let command = if rpc_params.command.is_empty() {
        "bash".to_string()
    } else {
        rpc_params.command
    };

    Ok(SpawnInput {
        command,
        args: rpc_params.args,
        cwd: rpc_params.cwd,
        env: None,
        session_id: parse_session_id(rpc_params.session),
        cols: rpc_params.cols.clamp(MIN_TERMINAL_COLS, MAX_TERMINAL_COLS),
        rows: rpc_params.rows.clamp(MIN_TERMINAL_ROWS, MAX_TERMINAL_ROWS),
    })
}

/// Convert SpawnOutput to RpcResponse.
pub fn spawn_output_to_response(id: u64, output: SpawnOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "session_id": output.session_id.as_str(),
            "pid": output.pid
        }),
    )
}

/// Parse SnapshotInput from RpcRequest using shared params type.
pub fn parse_snapshot_input(request: &RpcRequest) -> SnapshotInput {
    // Deserialize to shared params type, then convert to domain type
    let rpc_params: params::SnapshotParams = request
        .params
        .as_ref()
        .and_then(|p| serde_json::from_value(p.clone()).ok())
        .unwrap_or_default();

    SnapshotInput {
        session_id: parse_session_id(rpc_params.session),
        include_elements: rpc_params.include_elements,
        region: rpc_params.region,
        strip_ansi: rpc_params.strip_ansi,
        include_cursor: rpc_params.include_cursor,
    }
}

/// Convert SnapshotOutput to RpcResponse.
///
/// If `strip_ansi` is true, ANSI escape codes will be removed from the screen output.
pub fn snapshot_output_to_response(
    id: u64,
    output: SnapshotOutput,
    strip_ansi: bool,
) -> RpcResponse {
    use agent_tui_common::strip_ansi_codes;

    let screen = if strip_ansi {
        strip_ansi_codes(&output.screen)
    } else {
        output.screen
    };

    let mut result = json!({
        "session_id": output.session_id.as_str(),
        "screen": screen
    });

    if let Some(elements) = output.elements {
        result["elements"] = json!(elements.iter().map(element_to_json).collect::<Vec<_>>());
    }

    if let Some(cursor) = output.cursor {
        result["cursor"] = json!({
            "row": cursor.row,
            "col": cursor.col,
            "visible": cursor.visible
        });
    }

    RpcResponse::success(id, result)
}

/// Parse ClickInput from RpcRequest.
#[allow(clippy::result_large_err)]
pub fn parse_click_input(request: &RpcRequest) -> Result<ClickInput, RpcResponse> {
    let element_ref = request.require_str("ref")?.to_string();

    Ok(ClickInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        element_ref,
    })
}

/// Parse FillInput from RpcRequest.
#[allow(clippy::result_large_err)]
pub fn parse_fill_input(request: &RpcRequest) -> Result<FillInput, RpcResponse> {
    let element_ref = request.require_str("ref")?.to_string();
    let value = request.require_str("value")?.to_string();

    Ok(FillInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        element_ref,
        value,
    })
}

/// Parse FindInput from RpcRequest using shared params type.
pub fn parse_find_input(request: &RpcRequest) -> FindInput {
    let rpc_params: params::FindParams = request
        .params
        .as_ref()
        .and_then(|p| serde_json::from_value(p.clone()).ok())
        .unwrap_or_default();

    FindInput {
        session_id: parse_session_id(rpc_params.session),
        role: rpc_params.role,
        name: rpc_params.name,
        text: rpc_params.text,
        placeholder: rpc_params.placeholder,
        focused: rpc_params.focused,
        nth: rpc_params.nth,
        exact: rpc_params.exact,
    }
}

/// Parse KeystrokeInput from RpcRequest.
#[allow(clippy::result_large_err)]
pub fn parse_keystroke_input(request: &RpcRequest) -> Result<KeystrokeInput, RpcResponse> {
    let key = request.require_str("key")?.to_string();

    Ok(KeystrokeInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        key,
    })
}

/// Parse TypeInput from RpcRequest.
#[allow(clippy::result_large_err)]
pub fn parse_type_input(request: &RpcRequest) -> Result<TypeInput, RpcResponse> {
    let text = request.require_str("text")?.to_string();

    Ok(TypeInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        text,
    })
}

/// Parse WaitInput from RpcRequest using shared params type.
pub fn parse_wait_input(request: &RpcRequest) -> WaitInput {
    let rpc_params: params::WaitParams = request
        .params
        .as_ref()
        .and_then(|p| serde_json::from_value(p.clone()).ok())
        .unwrap_or_default();

    WaitInput {
        session_id: parse_session_id(rpc_params.session),
        text: rpc_params.text,
        timeout_ms: rpc_params.timeout_ms,
        condition: rpc_params.condition,
        target: rpc_params.target,
    }
}

/// Convert WaitOutput to RpcResponse.
pub fn wait_output_to_response(id: u64, output: WaitOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "found": output.found,
            "elapsed_ms": output.elapsed_ms
        }),
    )
}

/// Parse ScrollInput from RpcRequest.
#[allow(clippy::result_large_err)]
pub fn parse_scroll_input(request: &RpcRequest) -> Result<ScrollInput, RpcResponse> {
    let direction = request.require_str("direction")?.to_string();

    Ok(ScrollInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        direction,
        amount: request.param_u16("amount", 1),
    })
}

/// Convert KillOutput to RpcResponse.
pub fn kill_output_to_response(id: u64, output: KillOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "success": output.success,
            "session_id": output.session_id.as_str()
        }),
    )
}

/// Convert SessionsOutput to RpcResponse.
pub fn sessions_output_to_response(id: u64, output: SessionsOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "sessions": output.sessions.iter().map(session_info_to_json).collect::<Vec<_>>(),
            "active_session": output.active_session.as_ref().map(|id| id.as_str())
        }),
    )
}

/// Create a simple success response.
pub fn success_response(id: u64, message: &str) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "success": true,
            "message": message
        }),
    )
}

/// Create a click success response.
pub fn click_success_response(id: u64, element_ref: &str, warning: Option<&str>) -> RpcResponse {
    let mut result = json!({
        "success": true,
        "message": format!("Clicked {}", element_ref)
    });
    if let Some(w) = warning {
        result["warning"] = json!(w);
    }
    RpcResponse::success(id, result)
}

/// Create a fill success response.
pub fn fill_success_response(id: u64, element_ref: &str) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "success": true,
            "message": format!("Filled {} with value", element_ref)
        }),
    )
}

/// Parse ResizeInput from RpcRequest using shared params type.
pub fn parse_resize_input(request: &RpcRequest) -> ResizeInput {
    let rpc_params: params::ResizeParams = request
        .params
        .as_ref()
        .and_then(|p| serde_json::from_value(p.clone()).ok())
        .unwrap_or(params::ResizeParams {
            cols: 80,
            rows: 24,
            session: None,
        });

    ResizeInput {
        session_id: parse_session_id(rpc_params.session),
        cols: rpc_params.cols.clamp(MIN_TERMINAL_COLS, MAX_TERMINAL_COLS),
        rows: rpc_params.rows.clamp(MIN_TERMINAL_ROWS, MAX_TERMINAL_ROWS),
    }
}

/// Convert ResizeOutput to RpcResponse.
pub fn resize_output_to_response(id: u64, output: ResizeOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "success": output.success,
            "session_id": output.session_id.as_str()
        }),
    )
}

/// Convert RestartOutput to RpcResponse.
pub fn restart_output_to_response(id: u64, output: RestartOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "success": true,
            "old_session_id": output.old_session_id.as_str(),
            "new_session_id": output.new_session_id.as_str(),
            "command": output.command,
            "pid": output.pid
        }),
    )
}

/// Convert AttachOutput to RpcResponse.
pub fn attach_output_to_response(id: u64, output: AttachOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "success": output.success,
            "session_id": output.session_id.as_str(),
            "message": output.message
        }),
    )
}

/// Convert ScrollOutput to RpcResponse.
pub fn scroll_output_to_response(
    id: u64,
    output: ScrollOutput,
    direction: &str,
    amount: u16,
) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "success": output.success,
            "direction": direction,
            "amount": amount
        }),
    )
}

/// Parse CountInput from RpcRequest using shared params type.
pub fn parse_count_input(request: &RpcRequest) -> CountInput {
    let rpc_params: params::CountParams = request
        .params
        .as_ref()
        .and_then(|p| serde_json::from_value(p.clone()).ok())
        .unwrap_or_default();

    CountInput {
        session_id: parse_session_id(rpc_params.session),
        role: rpc_params.role,
        name: rpc_params.name,
        text: rpc_params.text,
    }
}

/// Convert CountOutput to RpcResponse.
pub fn count_output_to_response(id: u64, output: CountOutput) -> RpcResponse {
    RpcResponse::success(id, json!({ "count": output.count }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SessionId;

    fn make_request(id: u64, method: &str, params: Option<serde_json::Value>) -> RpcRequest {
        RpcRequest::new(id, method.to_string(), params)
    }

    #[test]
    fn test_parse_spawn_input_defaults() {
        let request = make_request(1, "spawn", Some(json!({})));
        let input = parse_spawn_input(&request).unwrap();
        assert_eq!(input.command, "bash");
        assert!(input.args.is_empty());
        assert_eq!(input.cols, 80);
        assert_eq!(input.rows, 24);
    }

    #[test]
    fn test_parse_spawn_input_custom() {
        let request = make_request(
            1,
            "spawn",
            Some(json!({
                "command": "vim",
                "args": ["file.txt"],
                "cols": 120,
                "rows": 40,
                "cwd": "/home/user"
            })),
        );
        let input = parse_spawn_input(&request).unwrap();
        assert_eq!(input.command, "vim");
        assert_eq!(input.args, vec!["file.txt"]);
        assert_eq!(input.cols, 120);
        assert_eq!(input.rows, 40);
        assert_eq!(input.cwd, Some("/home/user".to_string()));
    }

    #[test]
    fn test_parse_spawn_input_clamps_values() {
        let request = make_request(
            1,
            "spawn",
            Some(json!({
                "cols": 1000,
                "rows": 500
            })),
        );
        let input = parse_spawn_input(&request).unwrap();
        assert_eq!(input.cols, MAX_TERMINAL_COLS);
        assert_eq!(input.rows, MAX_TERMINAL_ROWS);
    }

    #[test]
    fn test_parse_snapshot_input() {
        let request = make_request(
            1,
            "snapshot",
            Some(json!({
                "session": "abc123",
                "include_elements": true,
                "include_cursor": true
            })),
        );
        let input = parse_snapshot_input(&request);
        assert_eq!(input.session_id, Some(SessionId::new("abc123")));
        assert!(input.include_elements);
        assert!(input.include_cursor);
    }

    #[test]
    fn test_parse_wait_input() {
        let request = make_request(
            1,
            "wait",
            Some(json!({
                "text": "Ready",
                "timeout_ms": 5000
            })),
        );
        let input = parse_wait_input(&request);
        assert_eq!(input.text, Some("Ready".to_string()));
        assert_eq!(input.timeout_ms, 5000);
    }
}
