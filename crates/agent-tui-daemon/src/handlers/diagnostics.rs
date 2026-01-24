use agent_tui_ipc::{RpcRequest, RpcResponse};
use serde_json::json;

use super::common::session_error_response;
use crate::adapters::parse_session_id;
use crate::domain::{
    ConsoleInput, ErrorsInput, HealthInput, MetricsInput, PtyReadInput, PtyWriteInput, TraceInput,
};
use crate::usecases::{
    ConsoleUseCase, ErrorsUseCase, HealthUseCase, MetricsUseCase, PtyReadUseCase, PtyWriteUseCase,
    TraceUseCase,
};

/// Handle health requests using the use case pattern.
pub fn handle_health_uc<U: HealthUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let input = HealthInput;

    match usecase.execute(input) {
        Ok(output) => RpcResponse::success(
            request.id,
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
        ),
        Err(e) => session_error_response(request.id, e),
    }
}

/// Handle metrics requests using the use case pattern.
pub fn handle_metrics_uc<U: MetricsUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let input = MetricsInput;

    match usecase.execute(input) {
        Ok(output) => RpcResponse::success(
            request.id,
            json!({
                "requests_total": output.requests_total,
                "errors_total": output.errors_total,
                "lock_timeouts": output.lock_timeouts,
                "poison_recoveries": output.poison_recoveries,
                "uptime_ms": output.uptime_ms,
                "active_connections": output.active_connections,
                "session_count": output.session_count
            }),
        ),
        Err(e) => session_error_response(request.id, e),
    }
}

/// Handle trace requests using the use case pattern.
pub fn handle_trace_uc<U: TraceUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let session_id = parse_session_id(request.param_str("session").map(String::from));
    let count = request.param_u64("count", 1000) as usize;
    let req_id = request.id;

    let input = TraceInput {
        session_id,
        start: false,
        stop: false,
        count,
    };

    match usecase.execute(input) {
        Ok(output) => {
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
                req_id,
                json!({
                    "trace": trace_json,
                    "count": trace_json.len()
                }),
            )
        }
        Err(e) => session_error_response(req_id, e),
    }
}

/// Handle console requests using the use case pattern.
pub fn handle_console_uc<U: ConsoleUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let session_id = parse_session_id(request.param_str("session").map(String::from));
    let req_id = request.id;

    let input = ConsoleInput {
        session_id,
        count: 0,
        clear: false,
    };

    match usecase.execute(input) {
        Ok(output) => RpcResponse::success(
            req_id,
            json!({
                "output": output.lines,
                "line_count": output.lines.len()
            }),
        ),
        Err(e) => session_error_response(req_id, e),
    }
}

/// Handle errors requests using the use case pattern.
pub fn handle_errors_uc<U: ErrorsUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let session_id = parse_session_id(request.param_str("session").map(String::from));
    let count = request.param_u64("count", 1000) as usize;
    let req_id = request.id;

    let input = ErrorsInput {
        session_id,
        count,
        clear: false,
    };

    match usecase.execute(input) {
        Ok(output) => {
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
                req_id,
                json!({
                    "errors": errors_json,
                    "count": errors_json.len()
                }),
            )
        }
        Err(e) => session_error_response(req_id, e),
    }
}

/// Handle pty_read requests using the use case pattern.
pub fn handle_pty_read_uc<U: PtyReadUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let session_id = parse_session_id(request.param_str("session").map(String::from));
    let max_bytes = request.param_u64("max_bytes", 4096) as usize;
    let req_id = request.id;

    let input = PtyReadInput {
        session_id,
        max_bytes,
    };

    match usecase.execute(input) {
        Ok(output) => RpcResponse::success(
            req_id,
            json!({
                "session_id": output.session_id.as_str(),
                "data": output.data,
                "bytes_read": output.bytes_read
            }),
        ),
        Err(e) => session_error_response(req_id, e),
    }
}

/// Handle pty_write requests using the use case pattern.
pub fn handle_pty_write_uc<U: PtyWriteUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let data = match request.require_str("data") {
        Ok(d) => d.to_string(),
        Err(resp) => return resp,
    };
    let session_id = parse_session_id(request.param_str("session").map(String::from));
    let req_id = request.id;

    let input = PtyWriteInput { session_id, data };

    match usecase.execute(input) {
        Ok(output) => RpcResponse::success(
            req_id,
            json!({
                "session_id": output.session_id.as_str(),
                "bytes_written": output.bytes_written,
                "success": output.success
            }),
        ),
        Err(e) => session_error_response(req_id, e),
    }
}
