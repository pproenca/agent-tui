pub mod params;
pub mod types;

pub use types::{ErrorData, RpcRequest, RpcResponse, RpcServerError};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Serialize;
use serde_json::json;

use super::snapshot_adapters::session_info_to_json;
use crate::adapters::daemon::DomainError;
use crate::domain::{
    AssertInput, AssertOutput, AttachInput, AttachOutput, CleanupInput, CleanupOutput,
    HealthOutput, KeydownInput, KeystrokeInput, KeyupInput, KillOutput, MetricsOutput,
    PtyReadInput, PtyReadOutput, PtyWriteInput, PtyWriteOutput, ResizeInput, ResizeOutput,
    RestartOutput, ScrollInput, ScrollOutput, SessionId, SessionInput, SessionsOutput,
    ShutdownOutput, SnapshotInput, SnapshotOutput, SpawnInput, SpawnOutput, TypeInput, WaitInput,
    WaitOutput,
};
use crate::usecases::ports::SessionError;

const MAX_TERMINAL_COLS: u16 = 500;
const MAX_TERMINAL_ROWS: u16 = 200;
const MIN_TERMINAL_COLS: u16 = 10;
const MIN_TERMINAL_ROWS: u16 = 5;

pub fn to_value<T: Serialize>(value: T) -> Result<serde_json::Value, serde_json::Error> {
    serde_json::to_value(value)
}

pub fn to_value_opt<T: Serialize>(
    value: Option<T>,
) -> Result<Option<serde_json::Value>, serde_json::Error> {
    value.map(serde_json::to_value).transpose()
}

pub fn parse_session_id(session: Option<String>) -> Option<SessionId> {
    session.and_then(|s| {
        if s.trim().is_empty() {
            None
        } else {
            Some(SessionId::new(s))
        }
    })
}

pub fn parse_session_input(request: &RpcRequest) -> SessionInput {
    let session_id = parse_session_id(request.param_str("session").map(String::from));
    SessionInput { session_id }
}

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

pub fn session_error_response(id: u64, err: SessionError) -> RpcResponse {
    domain_error_response(id, &DomainError::from(err))
}

pub fn lock_timeout_response(id: u64, session_id: Option<&str>) -> RpcResponse {
    let err = DomainError::LockTimeout {
        session_id: session_id.map(String::from),
    };
    domain_error_response(id, &err)
}

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

pub fn spawn_output_to_response(id: u64, output: SpawnOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "session_id": output.session_id.as_str(),
            "pid": output.pid
        }),
    )
}

pub fn parse_snapshot_input(request: &RpcRequest) -> SnapshotInput {
    let rpc_params: params::SnapshotParams = request
        .params
        .as_ref()
        .and_then(|p| serde_json::from_value(p.clone()).ok())
        .unwrap_or_default();

    SnapshotInput {
        session_id: parse_session_id(rpc_params.session),
        region: rpc_params.region,
        strip_ansi: rpc_params.strip_ansi,
        include_cursor: rpc_params.include_cursor,
        include_render: rpc_params.include_render,
    }
}

pub fn snapshot_output_to_response(
    id: u64,
    output: SnapshotOutput,
    strip_ansi: bool,
) -> RpcResponse {
    use crate::common::strip_ansi_codes;

    let screenshot = if strip_ansi {
        strip_ansi_codes(&output.screenshot)
    } else {
        output.screenshot
    };

    let mut result = json!({
        "session_id": output.session_id.as_str(),
        "screenshot": screenshot
    });

    if let Some(cursor) = output.cursor {
        result["cursor"] = json!({
            "row": cursor.row,
            "col": cursor.col,
            "visible": cursor.visible
        });
    }

    if let Some(rendered) = output.rendered {
        result["rendered"] = json!(rendered);
    }

    RpcResponse::success(id, result)
}

#[allow(clippy::result_large_err)]
pub fn parse_keystroke_input(request: &RpcRequest) -> Result<KeystrokeInput, RpcResponse> {
    let key = request.require_str("key")?.to_string();

    Ok(KeystrokeInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        key,
    })
}

#[allow(clippy::result_large_err)]
pub fn parse_type_input(request: &RpcRequest) -> Result<TypeInput, RpcResponse> {
    let text = request.require_str("text")?.to_string();

    Ok(TypeInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        text,
    })
}

#[allow(clippy::result_large_err)]
pub fn parse_keydown_input(request: &RpcRequest) -> Result<KeydownInput, RpcResponse> {
    let key = request.require_str("key")?.to_string();

    Ok(KeydownInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        key,
    })
}

#[allow(clippy::result_large_err)]
pub fn parse_keyup_input(request: &RpcRequest) -> Result<KeyupInput, RpcResponse> {
    let key = request.require_str("key")?.to_string();

    Ok(KeyupInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        key,
    })
}

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
    }
}

pub fn wait_output_to_response(id: u64, output: WaitOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "found": output.found,
            "elapsed_ms": output.elapsed_ms
        }),
    )
}

#[allow(clippy::result_large_err)]
pub fn parse_scroll_input(request: &RpcRequest) -> Result<ScrollInput, RpcResponse> {
    let direction = request.require_str("direction")?.to_string();

    Ok(ScrollInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        direction,
        amount: request.param_u16("amount", 1),
    })
}

pub fn scroll_output_to_response(id: u64, output: ScrollOutput) -> RpcResponse {
    RpcResponse::success(id, json!({ "success": output.success }))
}

pub fn kill_output_to_response(id: u64, output: KillOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "success": output.success,
            "session_id": output.session_id.as_str()
        }),
    )
}

pub fn sessions_output_to_response(id: u64, output: SessionsOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "sessions": output.sessions.iter().map(session_info_to_json).collect::<Vec<_>>(),
            "active_session": output.active_session.as_ref().map(|id| id.as_str())
        }),
    )
}

pub fn parse_resize_input(request: &RpcRequest) -> ResizeInput {
    let rpc_params: params::ResizeParams = request
        .params
        .as_ref()
        .and_then(|p| serde_json::from_value(p.clone()).ok())
        .unwrap_or(params::ResizeParams {
            cols: 0,
            rows: 0,
            session: None,
        });

    ResizeInput {
        session_id: parse_session_id(rpc_params.session),
        cols: rpc_params.cols,
        rows: rpc_params.rows,
    }
}

pub fn resize_output_to_response(id: u64, output: ResizeOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "success": output.success,
            "session_id": output.session_id.as_str(),
            "cols": output.cols,
            "rows": output.rows
        }),
    )
}

pub fn restart_output_to_response(id: u64, output: RestartOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "old_session_id": output.old_session_id.as_str(),
            "new_session_id": output.new_session_id.as_str(),
            "command": output.command,
            "pid": output.pid
        }),
    )
}

#[allow(clippy::result_large_err)]
pub fn parse_attach_input(request: &RpcRequest) -> Result<AttachInput, RpcResponse> {
    let session_id = request.require_str("session")?.to_string();
    Ok(AttachInput {
        session_id: SessionId::new(session_id),
    })
}

pub fn attach_output_to_response(id: u64, output: AttachOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "session_id": output.session_id.as_str(),
            "success": output.success,
            "message": output.message
        }),
    )
}

pub fn parse_cleanup_input(request: &RpcRequest) -> CleanupInput {
    let all = request.param_bool("all", false);
    CleanupInput { all }
}

pub fn cleanup_output_to_response(id: u64, output: CleanupOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "cleaned": output.cleaned,
            "failures": output.failures.iter().map(|f| json!({
                "session_id": f.session_id.as_str(),
                "error": f.error
            })).collect::<Vec<_>>()
        }),
    )
}

#[allow(clippy::result_large_err)]
pub fn parse_assert_input(request: &RpcRequest) -> Result<AssertInput, RpcResponse> {
    let condition_type = request.require_str("type")?;
    let value = request.require_str("value")?.to_string();

    let condition_type = crate::domain::AssertConditionType::parse(condition_type)
        .map_err(|e| RpcResponse::error(request.id, -32602, &format!("Invalid type: {}", e)))?;

    Ok(AssertInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        condition_type,
        value,
    })
}

pub fn assert_output_to_response(id: u64, output: AssertOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "passed": output.passed,
            "condition": output.condition
        }),
    )
}

pub fn health_output_to_response(id: u64, output: HealthOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "status": output.status,
            "pid": output.pid,
            "uptime_ms": output.uptime_ms,
            "session_count": output.session_count,
            "version": output.version,
            "commit": output.commit,
            "active_connections": output.active_connections,
            "total_requests": output.total_requests,
            "error_count": output.error_count
        }),
    )
}

pub fn metrics_output_to_response(id: u64, output: MetricsOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "requests_total": output.requests_total,
            "errors_total": output.errors_total,
            "lock_timeouts": output.lock_timeouts,
            "poison_recoveries": output.poison_recoveries,
            "uptime_ms": output.uptime_ms
        }),
    )
}

pub fn shutdown_output_to_response(id: u64, output: ShutdownOutput) -> RpcResponse {
    RpcResponse::success(id, json!({ "acknowledged": output.acknowledged }))
}

pub fn pty_read_output_to_response(id: u64, output: PtyReadOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "session_id": output.session_id.as_str(),
            "data": STANDARD.encode(&output.data),
            "bytes_read": output.bytes_read
        }),
    )
}

pub fn pty_write_output_to_response(id: u64, output: PtyWriteOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "session_id": output.session_id.as_str(),
            "bytes_written": output.bytes_written,
            "success": output.success
        }),
    )
}

pub fn parse_pty_read_input(request: &RpcRequest) -> PtyReadInput {
    let rpc_params: params::PtyReadParams = request
        .params
        .as_ref()
        .and_then(|p| serde_json::from_value(p.clone()).ok())
        .unwrap_or(params::PtyReadParams {
            session: None,
            max_bytes: 4096,
        });

    PtyReadInput {
        session_id: parse_session_id(rpc_params.session),
        max_bytes: rpc_params.max_bytes,
        timeout_ms: request.param_u64("timeout_ms", 1000),
    }
}

#[allow(clippy::result_large_err)]
pub fn parse_pty_write_input(request: &RpcRequest) -> Result<PtyWriteInput, RpcResponse> {
    let rpc_params: params::PtyWriteParams = request
        .params
        .as_ref()
        .ok_or_else(|| RpcResponse::error(request.id, -32602, "Missing params"))
        .and_then(|p| {
            serde_json::from_value(p.clone()).map_err(|e| {
                RpcResponse::error(request.id, -32602, &format!("Invalid params: {}", e))
            })
        })?;

    let data = STANDARD
        .decode(&rpc_params.data)
        .map_err(|e| RpcResponse::error(request.id, -32602, &format!("Invalid base64: {}", e)))?;

    Ok(PtyWriteInput {
        session_id: parse_session_id(rpc_params.session),
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(id: u64, method: &str, params: Option<serde_json::Value>) -> RpcRequest {
        RpcRequest::new(id, method.to_string(), params)
    }

    #[test]
    fn test_parse_spawn_input_defaults() {
        let request = make_request(1, "spawn", Some(json!({})));
        let input = parse_spawn_input(&request).unwrap();
        assert_eq!(input.command, "bash");
        assert_eq!(input.cols, 80);
        assert_eq!(input.rows, 24);
    }

    #[test]
    fn test_parse_snapshot_input() {
        let request = make_request(
            1,
            "snapshot",
            Some(json!({"strip_ansi": true, "include_cursor": true})),
        );
        let input = parse_snapshot_input(&request);
        assert!(input.strip_ansi);
        assert!(input.include_cursor);
    }

    #[test]
    fn test_parse_wait_input() {
        let request = make_request(
            1,
            "wait",
            Some(json!({"text": "ready", "timeout_ms": 5000})),
        );
        let input = parse_wait_input(&request);
        assert_eq!(input.text.unwrap(), "ready");
        assert_eq!(input.timeout_ms, 5000);
    }

    #[test]
    fn test_parse_keydown_input() {
        let request = make_request(1, "keydown", Some(json!({"key": "Ctrl"})));
        let input = parse_keydown_input(&request).unwrap();
        assert_eq!(input.key, "Ctrl");
    }

    #[test]
    fn test_parse_keyup_input() {
        let request = make_request(1, "keyup", Some(json!({"key": "Ctrl"})));
        let input = parse_keyup_input(&request).unwrap();
        assert_eq!(input.key, "Ctrl");
    }

    #[test]
    fn test_parse_pty_read_input_defaults() {
        let request = make_request(1, "pty_read", Some(json!({})));
        let input = parse_pty_read_input(&request);
        assert_eq!(input.max_bytes, 4096);
    }

    #[test]
    fn test_parse_pty_write_input() {
        let data = STANDARD.encode(b"hello");
        let request = make_request(1, "pty_write", Some(json!({"data": data})));
        let input = parse_pty_write_input(&request).unwrap();
        assert_eq!(input.data, b"hello");
    }

    #[test]
    fn test_health_output_to_response() {
        let output = HealthOutput {
            status: "ok".to_string(),
            pid: 123,
            uptime_ms: 1000,
            session_count: 1,
            version: "1.0.0".to_string(),
            commit: "abc".to_string(),
            active_connections: 0,
            total_requests: 10,
            error_count: 0,
        };
        let response = health_output_to_response(1, output);
        assert!(response.is_success());
    }

    #[test]
    fn test_metrics_output_to_response() {
        let output = MetricsOutput {
            requests_total: 1,
            errors_total: 0,
            lock_timeouts: 0,
            poison_recoveries: 0,
            uptime_ms: 1000,
            active_connections: 0,
            session_count: 0,
        };
        let response = metrics_output_to_response(1, output);
        assert!(response.is_success());
    }
}
