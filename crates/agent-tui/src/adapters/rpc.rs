use crate::infra::ipc::{RpcRequest, RpcResponse, params};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use super::recording_adapters::{build_asciicast, build_raw_frames};
use super::snapshot_adapters::session_info_to_json;
use crate::domain::{
    AccessibilitySnapshotInput, AttachInput, AttachOutput, CleanupInput, CleanupOutput, ClearInput,
    ClickInput, ConsoleInput, ConsoleOutput, CountInput, CountOutput, DomainElement,
    DoubleClickInput, ElementStateInput, ErrorsInput, ErrorsOutput, FillInput, FindInput,
    FindOutput, FocusCheckOutput, FocusInput, GetFocusedOutput, GetTextOutput, GetTitleOutput,
    GetValueOutput, HealthOutput, IsCheckedOutput, IsEnabledOutput, KeydownInput, KeystrokeInput,
    KeyupInput, KillOutput, LivePreviewStartInput, LivePreviewStartOutput, LivePreviewStatusOutput,
    LivePreviewStopOutput, MetricsOutput, MultiselectInput, MultiselectOutput, PtyReadInput,
    PtyReadOutput, PtyWriteInput, PtyWriteOutput, RecordStartInput, RecordStartOutput,
    RecordStatusInput, RecordStatusOutput, RecordStopInput, RecordStopOutput, ResizeInput,
    ResizeOutput, RestartOutput, ScrollInput, ScrollIntoViewInput, ScrollIntoViewOutput,
    ScrollOutput, SelectAllInput, SelectInput, SessionId, SessionInput, SessionsOutput,
    ShutdownOutput, SnapshotInput, SnapshotOutput, SpawnInput, SpawnOutput, ToggleInput,
    ToggleOutput, TraceInput, TraceOutput, TypeInput, VisibilityOutput, WaitInput, WaitOutput,
};
use crate::infra::daemon::{DomainError, LivePreviewError, SessionError};

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

#[allow(clippy::result_large_err)]
pub fn parse_live_preview_start_input(
    request: &RpcRequest,
) -> Result<LivePreviewStartInput, RpcResponse> {
    let rpc_params: params::LivePreviewStartParams = request
        .params
        .as_ref()
        .ok_or_else(|| RpcResponse::error(request.id, -32602, "Missing params"))
        .and_then(|p| {
            serde_json::from_value(p.clone()).map_err(|e| {
                RpcResponse::error(request.id, -32602, &format!("Invalid params: {}", e))
            })
        })?;

    let listen_addr = if let Some(addr) = rpc_params.listen {
        if let Err(e) = addr.parse::<std::net::SocketAddr>() {
            return Err(RpcResponse::error(
                request.id,
                -32602,
                &format!("Invalid listen address: {}", e),
            ));
        }
        Some(addr)
    } else {
        None
    };

    Ok(LivePreviewStartInput {
        session_id: parse_session_id(rpc_params.session),
        listen_addr,
        allow_remote: rpc_params.allow_remote,
    })
}

const MAX_TERMINAL_COLS: u16 = 500;
const MAX_TERMINAL_ROWS: u16 = 200;
const MIN_TERMINAL_COLS: u16 = 10;
const MIN_TERMINAL_ROWS: u16 = 5;

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

pub fn live_preview_error_response(id: u64, err: LivePreviewError) -> RpcResponse {
    RpcResponse::domain_error(
        id,
        err.code(),
        &err.to_string(),
        err.category().as_str(),
        Some(err.context()),
        Some(err.suggestion()),
    )
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
        include_elements: rpc_params.include_elements,
        region: rpc_params.region,
        strip_ansi: rpc_params.strip_ansi,
        include_cursor: rpc_params.include_cursor,
    }
}

pub fn snapshot_output_to_response(
    id: u64,
    output: SnapshotOutput,
    strip_ansi: bool,
) -> RpcResponse {
    use crate::common::strip_ansi_codes;

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

#[allow(clippy::result_large_err)]
pub fn parse_click_input(request: &RpcRequest) -> Result<ClickInput, RpcResponse> {
    let element_ref = request.require_str("ref")?.to_string();

    Ok(ClickInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        element_ref,
    })
}

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

pub fn success_response(id: u64, message: &str) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "success": true,
            "message": message
        }),
    )
}

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

pub fn fill_success_response(id: u64, element_ref: &str) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "success": true,
            "message": format!("Filled {} with value", element_ref)
        }),
    )
}

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

pub fn resize_output_to_response(id: u64, output: ResizeOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "success": output.success,
            "session_id": output.session_id.as_str(),
            "size": { "cols": output.cols, "rows": output.rows }
        }),
    )
}

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

#[allow(clippy::result_large_err)]
pub fn parse_attach_input(request: &RpcRequest) -> Result<AttachInput, RpcResponse> {
    let session_id = request.require_str("session")?;
    Ok(AttachInput {
        session_id: SessionId::new(session_id),
    })
}

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

pub fn live_preview_start_output_to_response(
    id: u64,
    output: LivePreviewStartOutput,
) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "session_id": output.session_id.as_str(),
            "listen": output.listen_addr,
            "running": true
        }),
    )
}

pub fn live_preview_stop_output_to_response(id: u64, output: LivePreviewStopOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "stopped": output.stopped,
            "session_id": output.session_id.as_ref().map(|id| id.as_str())
        }),
    )
}

pub fn live_preview_status_output_to_response(
    id: u64,
    output: LivePreviewStatusOutput,
) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "running": output.running,
            "session_id": output.session_id.as_ref().map(|id| id.as_str()),
            "listen": output.listen_addr
        }),
    )
}

pub fn parse_cleanup_input(request: &RpcRequest) -> CleanupInput {
    let all = request.param_bool("all", false);
    CleanupInput { all }
}

pub fn cleanup_output_to_response(id: u64, output: CleanupOutput) -> RpcResponse {
    let failures_json: Vec<Value> = output
        .failures
        .iter()
        .map(|f| {
            json!({
                "session": f.session_id.as_str(),
                "error": f.error
            })
        })
        .collect();

    RpcResponse::success(
        id,
        json!({
            "sessions_cleaned": output.cleaned,
            "sessions_failed": output.failures.len(),
            "failures": failures_json
        }),
    )
}

use crate::domain::{AssertConditionType, AssertInput, AssertOutput};

#[allow(clippy::result_large_err)]
pub fn parse_assert_input(request: &RpcRequest) -> Result<AssertInput, RpcResponse> {
    let condition = request.param_str("condition").unwrap_or("");
    let session = request.param_str("session").map(String::from);

    let parts: Vec<&str> = condition.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(RpcResponse::error(
            request.id,
            -32602,
            "Invalid condition format. Use: text:pattern, element:ref, or session:id",
        ));
    }

    let (cond_type_str, value) = (parts[0], parts[1]);

    let condition_type = match AssertConditionType::parse(cond_type_str) {
        Ok(ct) => ct,
        Err(msg) => {
            return Err(RpcResponse::error(request.id, -32602, &msg));
        }
    };

    Ok(AssertInput {
        session_id: parse_session_id(session),
        condition_type,
        value: value.to_string(),
    })
}

pub fn assert_output_to_response(id: u64, output: AssertOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "condition": output.condition,
            "passed": output.passed
        }),
    )
}

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

pub fn count_output_to_response(id: u64, output: CountOutput) -> RpcResponse {
    RpcResponse::success(id, json!({ "count": output.count }))
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
            "uptime_ms": output.uptime_ms,
            "active_connections": output.active_connections,
            "session_count": output.session_count
        }),
    )
}

pub fn shutdown_output_to_response(id: u64, output: ShutdownOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "acknowledged": output.acknowledged
        }),
    )
}

pub fn trace_output_to_response(id: u64, output: TraceOutput) -> RpcResponse {
    let trace_json: Vec<_> = output
        .entries
        .iter()
        .map(|t| {
            json!({
                "timestamp_ms": t.timestamp_ms,
                "action": t.action,
                "details": t.details
            })
        })
        .collect();

    RpcResponse::success(
        id,
        json!({
            "trace": trace_json,
            "count": trace_json.len()
        }),
    )
}

pub fn console_output_to_response(id: u64, output: ConsoleOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "output": output.lines,
            "line_count": output.lines.len()
        }),
    )
}

pub fn errors_output_to_response(id: u64, output: ErrorsOutput) -> RpcResponse {
    let errors_json: Vec<_> = output
        .errors
        .iter()
        .map(|e| {
            json!({
                "timestamp": e.timestamp,
                "message": e.message,
                "source": e.source
            })
        })
        .collect();

    RpcResponse::success(
        id,
        json!({
            "errors": errors_json,
            "count": errors_json.len()
        }),
    )
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

pub fn record_start_output_to_response(id: u64, output: RecordStartOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "success": output.success,
            "session_id": output.session_id.as_str(),
            "recording": true
        }),
    )
}

pub fn record_status_output_to_response(id: u64, output: RecordStatusOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "recording": output.is_recording,
            "frame_count": output.frame_count,
            "duration_ms": output.duration_ms
        }),
    )
}

pub fn record_stop_output_to_response(id: u64, output: RecordStopOutput) -> RpcResponse {
    let recording_data = if output.format == "asciicast" {
        build_asciicast(
            output.session_id.as_ref(),
            output.cols,
            output.rows,
            &output.frames,
        )
    } else {
        build_raw_frames(&output.frames)
    };

    RpcResponse::success(
        id,
        json!({
            "success": true,
            "session_id": output.session_id.as_str(),
            "frame_count": output.frame_count,
            "recording": recording_data
        }),
    )
}

pub fn find_output_to_response(id: u64, output: FindOutput) -> RpcResponse {
    let elements_json: Vec<Value> = output.elements.iter().map(element_to_json).collect();
    RpcResponse::success(
        id,
        json!({
            "elements": elements_json,
            "count": output.count
        }),
    )
}

pub fn get_text_output_to_response(
    id: u64,
    element_ref: &str,
    output: GetTextOutput,
) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({ "ref": element_ref, "text": output.text, "found": output.found }),
    )
}

pub fn get_value_output_to_response(
    id: u64,
    element_ref: &str,
    output: GetValueOutput,
) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({ "ref": element_ref, "value": output.value, "found": output.found }),
    )
}

pub fn visibility_output_to_response(
    id: u64,
    element_ref: &str,
    output: VisibilityOutput,
) -> RpcResponse {
    RpcResponse::success(id, json!({ "ref": element_ref, "visible": output.visible }))
}

pub fn focus_check_output_to_response(
    id: u64,
    element_ref: &str,
    output: FocusCheckOutput,
) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({ "ref": element_ref, "focused": output.focused, "found": output.found }),
    )
}

pub fn is_enabled_output_to_response(
    id: u64,
    element_ref: &str,
    output: IsEnabledOutput,
) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({ "ref": element_ref, "enabled": output.enabled, "found": output.found }),
    )
}

pub fn is_checked_output_to_response(
    id: u64,
    element_ref: &str,
    output: IsCheckedOutput,
) -> RpcResponse {
    let mut response =
        json!({ "ref": element_ref, "checked": output.checked, "found": output.found });
    if let Some(msg) = output.message {
        response["message"] = json!(msg);
    }
    RpcResponse::success(id, response)
}

pub fn get_focused_output_to_response(id: u64, output: GetFocusedOutput) -> RpcResponse {
    if let Some(el) = output.element {
        RpcResponse::success(
            id,
            json!({
                "ref": el.element_ref,
                "type": el.element_type.as_str(),
                "label": el.label,
                "value": el.value,
                "found": true
            }),
        )
    } else {
        RpcResponse::success(
            id,
            json!({
                "found": false,
                "message": "No focused element found."
            }),
        )
    }
}

pub fn get_title_output_to_response(id: u64, output: GetTitleOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "session_id": output.session_id.as_str(),
            "title": output.title
        }),
    )
}

pub fn toggle_output_to_response(id: u64, element_ref: &str, output: ToggleOutput) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({ "success": true, "ref": element_ref, "checked": output.checked }),
    )
}

pub fn select_output_to_response(id: u64, element_ref: &str, option: &str) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({ "success": true, "ref": element_ref, "option": option }),
    )
}

pub fn scroll_into_view_output_to_response(
    id: u64,
    element_ref: &str,
    output: ScrollIntoViewOutput,
) -> RpcResponse {
    if output.success {
        RpcResponse::success(
            id,
            json!({
                "success": true,
                "ref": element_ref,
                "scrolls_needed": output.scrolls_needed
            }),
        )
    } else {
        RpcResponse::success(
            id,
            json!({
                "success": false,
                "message": output.message.unwrap_or_else(|| "Element not found after scrolling".to_string())
            }),
        )
    }
}

pub fn multiselect_output_to_response(
    id: u64,
    element_ref: &str,
    output: MultiselectOutput,
) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({ "success": true, "ref": element_ref, "selected_options": output.selected_options }),
    )
}

#[allow(clippy::result_large_err)]
pub fn parse_double_click_input(request: &RpcRequest) -> Result<DoubleClickInput, RpcResponse> {
    let element_ref = request.require_str("ref")?.to_string();

    Ok(DoubleClickInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        element_ref,
    })
}

#[allow(clippy::result_large_err)]
pub fn parse_focus_input(request: &RpcRequest) -> Result<FocusInput, RpcResponse> {
    let element_ref = request.require_str("ref")?.to_string();

    Ok(FocusInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        element_ref,
    })
}

#[allow(clippy::result_large_err)]
pub fn parse_clear_input(request: &RpcRequest) -> Result<ClearInput, RpcResponse> {
    let element_ref = request.require_str("ref")?.to_string();

    Ok(ClearInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        element_ref,
    })
}

#[allow(clippy::result_large_err)]
pub fn parse_select_all_input(request: &RpcRequest) -> Result<SelectAllInput, RpcResponse> {
    let element_ref = request.require_str("ref")?.to_string();

    Ok(SelectAllInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        element_ref,
    })
}

#[allow(clippy::result_large_err)]
pub fn parse_scroll_into_view_input(
    request: &RpcRequest,
) -> Result<ScrollIntoViewInput, RpcResponse> {
    let element_ref = request.require_str("ref")?.to_string();

    Ok(ScrollIntoViewInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        element_ref,
    })
}

#[allow(clippy::result_large_err)]
pub fn parse_element_state_input(request: &RpcRequest) -> Result<ElementStateInput, RpcResponse> {
    let element_ref = request.require_str("ref")?.to_string();

    Ok(ElementStateInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        element_ref,
    })
}

#[allow(clippy::result_large_err)]
pub fn parse_toggle_input(request: &RpcRequest) -> Result<ToggleInput, RpcResponse> {
    let element_ref = request.require_str("ref")?.to_string();

    Ok(ToggleInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        element_ref,
        state: request.param_bool_opt("state"),
    })
}

#[allow(clippy::result_large_err)]
pub fn parse_select_input(request: &RpcRequest) -> Result<SelectInput, RpcResponse> {
    let element_ref = request.require_str("ref")?.to_string();
    let option = request.require_str("option")?.to_string();

    Ok(SelectInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        element_ref,
        option,
    })
}

#[allow(clippy::result_large_err)]
pub fn parse_multiselect_input(request: &RpcRequest) -> Result<MultiselectInput, RpcResponse> {
    let options: Vec<String> = request
        .require_array("options")?
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();

    if options.is_empty() {
        return Err(RpcResponse::error(
            request.id,
            -32602,
            "Options array cannot be empty",
        ));
    }

    let element_ref = request.require_str("ref")?.to_string();

    Ok(MultiselectInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        element_ref,
        options,
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

pub fn parse_record_start_input(request: &RpcRequest) -> RecordStartInput {
    RecordStartInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
    }
}

pub fn parse_record_stop_input(request: &RpcRequest) -> RecordStopInput {
    RecordStopInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        format: request.param_str("format").map(String::from),
    }
}

pub fn parse_record_status_input(request: &RpcRequest) -> RecordStatusInput {
    RecordStatusInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
    }
}

pub fn parse_accessibility_snapshot_input(request: &RpcRequest) -> AccessibilitySnapshotInput {
    AccessibilitySnapshotInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        interactive_only: request.param_bool("interactive", false),
    }
}

pub fn parse_trace_input(request: &RpcRequest) -> TraceInput {
    TraceInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        start: false,
        stop: false,
        count: request.param_u64("count", 1000) as usize,
    }
}

pub fn parse_console_input(request: &RpcRequest) -> ConsoleInput {
    ConsoleInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        count: 0,
        clear: false,
    }
}

pub fn parse_errors_input(request: &RpcRequest) -> ErrorsInput {
    ErrorsInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        count: request.param_u64("count", 1000) as usize,
        clear: false,
    }
}

pub fn parse_pty_read_input(request: &RpcRequest) -> PtyReadInput {
    PtyReadInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        max_bytes: request.param_u64("max_bytes", 4096) as usize,
        timeout_ms: request.param_u64("timeout_ms", 100),
    }
}

#[allow(clippy::result_large_err)]
pub fn parse_pty_write_input(request: &RpcRequest) -> Result<PtyWriteInput, RpcResponse> {
    let data_b64 = request.require_str("data")?;
    let data = STANDARD.decode(data_b64).map_err(|e| {
        RpcResponse::error(
            request.id,
            -32602, // JSON-RPC Invalid Params
            &format!("Invalid base64 data: {}", e),
        )
    })?;

    Ok(PtyWriteInput {
        session_id: parse_session_id(request.param_str("session").map(String::from)),
        data,
    })
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

    #[test]
    fn test_parse_double_click_input() {
        let request = make_request(
            1,
            "dbl_click",
            Some(json!({ "ref": "@btn1", "session": "sess1" })),
        );
        let input = parse_double_click_input(&request).unwrap();
        assert_eq!(input.element_ref, "@btn1");
        assert_eq!(input.session_id, Some(SessionId::new("sess1")));
    }

    #[test]
    fn test_parse_double_click_input_missing_ref() {
        let request = make_request(1, "dbl_click", Some(json!({})));
        let result = parse_double_click_input(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_focus_input() {
        let request = make_request(1, "focus", Some(json!({ "ref": "@input1" })));
        let input = parse_focus_input(&request).unwrap();
        assert_eq!(input.element_ref, "@input1");
        assert!(input.session_id.is_none());
    }

    #[test]
    fn test_parse_clear_input() {
        let request = make_request(1, "clear", Some(json!({ "ref": "@field" })));
        let input = parse_clear_input(&request).unwrap();
        assert_eq!(input.element_ref, "@field");
    }

    #[test]
    fn test_parse_select_all_input() {
        let request = make_request(1, "select_all", Some(json!({ "ref": "@textarea" })));
        let input = parse_select_all_input(&request).unwrap();
        assert_eq!(input.element_ref, "@textarea");
    }

    #[test]
    fn test_parse_scroll_into_view_input() {
        let request = make_request(1, "scroll_into_view", Some(json!({ "ref": "@item" })));
        let input = parse_scroll_into_view_input(&request).unwrap();
        assert_eq!(input.element_ref, "@item");
    }

    #[test]
    fn test_parse_element_state_input() {
        let request = make_request(1, "get_text", Some(json!({ "ref": "@label" })));
        let input = parse_element_state_input(&request).unwrap();
        assert_eq!(input.element_ref, "@label");
    }

    #[test]
    fn test_parse_toggle_input_with_state() {
        let request = make_request(
            1,
            "toggle",
            Some(json!({ "ref": "@checkbox", "state": true })),
        );
        let input = parse_toggle_input(&request).unwrap();
        assert_eq!(input.element_ref, "@checkbox");
        assert_eq!(input.state, Some(true));
    }

    #[test]
    fn test_parse_toggle_input_without_state() {
        let request = make_request(1, "toggle", Some(json!({ "ref": "@checkbox" })));
        let input = parse_toggle_input(&request).unwrap();
        assert!(input.state.is_none());
    }

    #[test]
    fn test_parse_select_input() {
        let request = make_request(
            1,
            "select",
            Some(json!({ "ref": "@dropdown", "option": "choice1" })),
        );
        let input = parse_select_input(&request).unwrap();
        assert_eq!(input.element_ref, "@dropdown");
        assert_eq!(input.option, "choice1");
    }

    #[test]
    fn test_parse_select_input_missing_option() {
        let request = make_request(1, "select", Some(json!({ "ref": "@dropdown" })));
        let result = parse_select_input(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_multiselect_input() {
        let request = make_request(
            1,
            "multiselect",
            Some(json!({ "ref": "@list", "options": ["a", "b", "c"] })),
        );
        let input = parse_multiselect_input(&request).unwrap();
        assert_eq!(input.element_ref, "@list");
        assert_eq!(input.options, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_multiselect_input_empty_options() {
        let request = make_request(
            1,
            "multiselect",
            Some(json!({ "ref": "@list", "options": [] })),
        );
        let result = parse_multiselect_input(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_keydown_input() {
        let request = make_request(1, "keydown", Some(json!({ "key": "Shift" })));
        let input = parse_keydown_input(&request).unwrap();
        assert_eq!(input.key, "Shift");
    }

    #[test]
    fn test_parse_keyup_input() {
        let request = make_request(1, "keyup", Some(json!({ "key": "Control" })));
        let input = parse_keyup_input(&request).unwrap();
        assert_eq!(input.key, "Control");
    }

    #[test]
    fn test_parse_record_start_input() {
        let request = make_request(1, "record_start", Some(json!({ "session": "rec-session" })));
        let input = parse_record_start_input(&request);
        assert_eq!(input.session_id, Some(SessionId::new("rec-session")));
    }

    #[test]
    fn test_parse_record_stop_input() {
        let request = make_request(1, "record_stop", Some(json!({ "format": "asciicast" })));
        let input = parse_record_stop_input(&request);
        assert_eq!(input.format, Some("asciicast".to_string()));
    }

    #[test]
    fn test_parse_record_status_input() {
        let request = make_request(1, "record_status", None);
        let input = parse_record_status_input(&request);
        assert!(input.session_id.is_none());
    }

    #[test]
    fn test_parse_accessibility_snapshot_input() {
        let request = make_request(
            1,
            "accessibility_snapshot",
            Some(json!({ "interactive": true })),
        );
        let input = parse_accessibility_snapshot_input(&request);
        assert!(input.interactive_only);
    }

    #[test]
    fn test_parse_trace_input() {
        let request = make_request(1, "trace", Some(json!({ "count": 500 })));
        let input = parse_trace_input(&request);
        assert_eq!(input.count, 500);
    }

    #[test]
    fn test_parse_trace_input_defaults() {
        let request = make_request(1, "trace", None);
        let input = parse_trace_input(&request);
        assert_eq!(input.count, 1000);
    }

    #[test]
    fn test_parse_console_input() {
        let request = make_request(1, "console", None);
        let input = parse_console_input(&request);
        assert_eq!(input.count, 0);
        assert!(!input.clear);
    }

    #[test]
    fn test_parse_errors_input() {
        let request = make_request(1, "errors", Some(json!({ "count": 50 })));
        let input = parse_errors_input(&request);
        assert_eq!(input.count, 50);
    }

    #[test]
    fn test_parse_pty_read_input() {
        let request = make_request(
            1,
            "pty_read",
            Some(json!({ "max_bytes": 8192, "timeout_ms": 250 })),
        );
        let input = parse_pty_read_input(&request);
        assert_eq!(input.max_bytes, 8192);
        assert_eq!(input.timeout_ms, 250);
    }

    #[test]
    fn test_parse_pty_read_input_defaults() {
        let request = make_request(1, "pty_read", None);
        let input = parse_pty_read_input(&request);
        assert_eq!(input.max_bytes, 4096);
        assert_eq!(input.timeout_ms, 100);
    }

    #[test]
    fn test_parse_pty_write_input() {
        // "hello" encoded as base64 is "aGVsbG8="
        let request = make_request(1, "pty_write", Some(json!({ "data": "aGVsbG8=" })));
        let input = parse_pty_write_input(&request).unwrap();
        assert_eq!(input.data, b"hello");
    }

    #[test]
    fn test_parse_pty_write_input_missing_data() {
        let request = make_request(1, "pty_write", Some(json!({})));
        let result = parse_pty_write_input(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_pty_write_input_invalid_base64() {
        let request = make_request(
            1,
            "pty_write",
            Some(json!({ "data": "not-valid-base64!!!" })),
        );
        let result = parse_pty_write_input(&request);
        assert!(result.is_err());
    }

    fn extract_result(response: RpcResponse) -> serde_json::Value {
        let json_str = serde_json::to_string(&response).expect("serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("parse");
        parsed["result"].clone()
    }

    #[test]
    fn test_health_output_to_response() {
        let output = HealthOutput {
            status: "healthy".to_string(),
            pid: 1234,
            uptime_ms: 5000,
            session_count: 2,
            version: "0.1.0".to_string(),
            active_connections: 1,
            total_requests: 100,
            error_count: 5,
        };
        let response = health_output_to_response(1, output);
        let result = extract_result(response);
        assert_eq!(result["status"], "healthy");
        assert_eq!(result["pid"], 1234);
    }

    #[test]
    fn test_metrics_output_to_response() {
        let output = MetricsOutput {
            requests_total: 100,
            errors_total: 5,
            lock_timeouts: 2,
            poison_recoveries: 0,
            uptime_ms: 10000,
            active_connections: 3,
            session_count: 2,
        };
        let response = metrics_output_to_response(1, output);
        let result = extract_result(response);
        assert_eq!(result["requests_total"], 100);
        assert_eq!(result["errors_total"], 5);
    }

    #[test]
    fn test_console_output_to_response() {
        let output = ConsoleOutput {
            lines: vec!["line1".to_string(), "line2".to_string()],
        };
        let response = console_output_to_response(1, output);
        let result = extract_result(response);
        assert_eq!(result["line_count"], 2);
    }

    #[test]
    fn test_pty_read_output_to_response() {
        let output = PtyReadOutput {
            session_id: SessionId::new("sess1"),
            data: b"output".to_vec(),
            bytes_read: 6,
        };
        let response = pty_read_output_to_response(1, output);
        let result = extract_result(response);
        assert_eq!(result["session_id"], "sess1");
        assert_eq!(result["bytes_read"], 6);
        // "output" encoded as base64 is "b3V0cHV0"
        assert_eq!(result["data"], "b3V0cHV0");
    }

    #[test]
    fn test_pty_write_output_to_response() {
        let output = PtyWriteOutput {
            session_id: SessionId::new("sess1"),
            bytes_written: 5,
            success: true,
        };
        let response = pty_write_output_to_response(1, output);
        let result = extract_result(response);
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["bytes_written"], 5);
    }

    #[test]
    fn test_record_start_output_to_response() {
        let output = RecordStartOutput {
            session_id: SessionId::new("rec1"),
            success: true,
        };
        let response = record_start_output_to_response(1, output);
        let result = extract_result(response);
        assert!(result["recording"].as_bool().unwrap());
    }

    #[test]
    fn test_get_text_output_to_response() {
        let output = GetTextOutput {
            found: true,
            text: "hello".to_string(),
        };
        let response = get_text_output_to_response(1, "@label", output);
        let result = extract_result(response);
        assert_eq!(result["ref"], "@label");
        assert_eq!(result["text"], "hello");
        assert!(result["found"].as_bool().unwrap());
    }

    #[test]
    fn test_visibility_output_to_response() {
        let output = VisibilityOutput {
            found: true,
            visible: true,
        };
        let response = visibility_output_to_response(1, "@btn", output);
        let result = extract_result(response);
        assert_eq!(result["ref"], "@btn");
        assert!(result["visible"].as_bool().unwrap());
    }

    #[test]
    fn test_toggle_output_to_response() {
        let output = ToggleOutput {
            success: true,
            checked: true,
            message: None,
        };
        let response = toggle_output_to_response(1, "@checkbox", output);
        let result = extract_result(response);
        assert!(result["checked"].as_bool().unwrap());
    }

    #[test]
    fn test_scroll_into_view_output_to_response_success() {
        let output = ScrollIntoViewOutput {
            success: true,
            scrolls_needed: 3,
            message: None,
        };
        let response = scroll_into_view_output_to_response(1, "@item", output);
        let result = extract_result(response);
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["scrolls_needed"], 3);
    }

    #[test]
    fn test_scroll_into_view_output_to_response_failure() {
        let output = ScrollIntoViewOutput {
            success: false,
            scrolls_needed: 0,
            message: Some("Not found".to_string()),
        };
        let response = scroll_into_view_output_to_response(1, "@item", output);
        let result = extract_result(response);
        assert!(!result["success"].as_bool().unwrap());
        assert_eq!(result["message"], "Not found");
    }

    #[test]
    fn test_multiselect_output_to_response() {
        let output = MultiselectOutput {
            success: true,
            selected_options: vec!["a".to_string(), "b".to_string()],
            message: None,
        };
        let response = multiselect_output_to_response(1, "@list", output);
        let result = extract_result(response);
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["selected_options"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_get_title_output_to_response() {
        let output = GetTitleOutput {
            session_id: SessionId::new("sess1"),
            title: "My Terminal".to_string(),
        };
        let response = get_title_output_to_response(1, output);
        let result = extract_result(response);
        assert_eq!(result["session_id"], "sess1");
        assert_eq!(result["title"], "My Terminal");
    }
}
