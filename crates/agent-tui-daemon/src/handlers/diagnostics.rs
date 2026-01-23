use agent_tui_ipc::{RpcRequest, RpcResponse};
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use super::common::{domain_error_response, lock_timeout_response, session_error_response};
use crate::domain::{ConsoleInput, ErrorsInput, PtyReadInput, PtyWriteInput, TraceInput};
use crate::error::DomainError;
use crate::lock_helpers::{LOCK_TIMEOUT, acquire_session_lock};
use crate::metrics::DaemonMetrics;
use crate::session::SessionManager;
use crate::usecases::{
    ConsoleUseCase, ErrorsUseCase, PtyReadUseCase, PtyWriteUseCase, TraceUseCase,
};

pub fn handle_health(
    session_manager: &Arc<SessionManager>,
    metrics: &Arc<DaemonMetrics>,
    start_time: Instant,
    active_connections: &std::sync::atomic::AtomicUsize,
    request: RpcRequest,
) -> RpcResponse {
    let uptime_ms = start_time.elapsed().as_millis() as u64;
    RpcResponse::success(
        request.id,
        json!({
            "status": "healthy",
            "pid": std::process::id(),
            "uptime_ms": uptime_ms,
            "session_count": session_manager.session_count(),
            "version": env!("CARGO_PKG_VERSION"),
            "active_connections": active_connections.load(Ordering::Relaxed),
            "total_requests": metrics.requests(),
            "error_count": metrics.errors()
        }),
    )
}

pub fn handle_metrics(
    session_manager: &Arc<SessionManager>,
    metrics: &Arc<DaemonMetrics>,
    start_time: Instant,
    active_connections: &std::sync::atomic::AtomicUsize,
    request: RpcRequest,
) -> RpcResponse {
    RpcResponse::success(
        request.id,
        json!({
            "requests_total": metrics.requests(),
            "errors_total": metrics.errors(),
            "lock_timeouts": metrics.lock_timeouts(),
            "poison_recoveries": metrics.poison_recoveries(),
            "uptime_ms": start_time.elapsed().as_millis() as u64,
            "active_connections": active_connections.load(Ordering::Relaxed),
            "session_count": session_manager.session_count()
        }),
    )
}

/// Handle trace requests using the use case pattern.
pub fn handle_trace_uc<U: TraceUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let session_id = request.param_str("session").map(String::from);
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
    let session_id = request.param_str("session").map(String::from);
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
    let session_id = request.param_str("session").map(String::from);
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
    let session_id = request.param_str("session").map(String::from);
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
                "session_id": output.session_id,
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
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = PtyWriteInput { session_id, data };

    match usecase.execute(input) {
        Ok(output) => RpcResponse::success(
            req_id,
            json!({
                "session_id": output.session_id,
                "bytes_written": output.bytes_written,
                "success": output.success
            }),
        ),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_trace(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let session_id = request.param_str("session");
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };

            let traces = sess.get_trace_entries(1000);
            let output: Vec<_> = traces
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
                    "session_id": sess.id,
                    "trace": output,
                    "count": output.len()
                }),
            )
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

pub fn handle_console(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let session_id = request.param_str("session");
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };

            if let Err(e) = sess.update() {
                eprintln!("Warning: Session update failed during console: {}", e);
            }

            let screen_text = sess.screen_text();
            let lines: Vec<&str> = screen_text.lines().collect();

            RpcResponse::success(
                req_id,
                json!({
                    "session_id": sess.id,
                    "output": lines,
                    "line_count": lines.len()
                }),
            )
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

pub fn handle_errors(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let session_id = request.param_str("session");
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };

            let errors = sess.get_errors(1000);
            let output: Vec<_> = errors
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
                    "session_id": sess.id,
                    "errors": output,
                    "count": output.len()
                }),
            )
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

pub fn handle_pty_read(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let session_id = request.param_str("session");
    let max_bytes = request.param_u64("max_bytes", 4096) as usize;
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };

            let mut buf = vec![0u8; max_bytes];
            match sess.pty_try_read(&mut buf, 100) {
                Ok(bytes_read) => {
                    buf.truncate(bytes_read);
                    let text = String::from_utf8_lossy(&buf);
                    RpcResponse::success(
                        req_id,
                        json!({
                            "session_id": sess.id,
                            "data": text,
                            "bytes_read": bytes_read
                        }),
                    )
                }
                Err(e) => {
                    let err = DomainError::PtyError {
                        operation: "pty_read".to_string(),
                        reason: e.to_string(),
                    };
                    domain_error_response(req_id, &err)
                }
            }
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

pub fn handle_pty_write(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let data = match request.require_str("data") {
        Ok(d) => d.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session");
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };

            match sess.pty_write(data.as_bytes()) {
                Ok(()) => RpcResponse::success(
                    req_id,
                    json!({
                        "session_id": sess.id,
                        "bytes_written": data.len(),
                        "success": true
                    }),
                ),
                Err(e) => {
                    let err = DomainError::PtyError {
                        operation: "pty_write".to_string(),
                        reason: e.to_string(),
                    };
                    domain_error_response(req_id, &err)
                }
            }
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}
