use agent_tui_ipc::{RpcRequest, RpcResponse};
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::error::DomainError;
use crate::lock_helpers::{LOCK_TIMEOUT, acquire_session_lock};
use crate::metrics::DaemonMetrics;
use crate::session::SessionManager;

fn domain_error_response(id: u64, err: &DomainError) -> RpcResponse {
    RpcResponse::domain_error(
        id,
        err.code(),
        &err.to_string(),
        err.category().as_str(),
        Some(err.context()),
        Some(err.suggestion()),
    )
}

fn lock_timeout_response(id: u64, session_id: Option<&str>) -> RpcResponse {
    let err = DomainError::LockTimeout {
        session_id: session_id.map(String::from),
    };
    domain_error_response(id, &err)
}

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
