use agent_tui_ipc::{RpcRequest, RpcResponse};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

use super::common::{domain_error_response, lock_timeout_response};
use crate::error::DomainError;
use crate::lock_helpers::{LOCK_TIMEOUT, acquire_session_lock};
use crate::session::{SessionError, SessionManager};

const MAX_TERMINAL_COLS: u16 = 500;
const MAX_TERMINAL_ROWS: u16 = 200;
const MIN_TERMINAL_COLS: u16 = 10;
const MIN_TERMINAL_ROWS: u16 = 5;

pub fn handle_spawn(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let params = match request.params {
        Some(p) => p,
        None => return RpcResponse::error(request.id, -32602, "Missing params"),
    };

    let command = params
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or("bash");

    let args: Vec<String> = params
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let cwd = params.get("cwd").and_then(|v| v.as_str());

    let session_id = params
        .get("session")
        .and_then(|v| v.as_str())
        .map(String::from);

    let cols = params.get("cols").and_then(|v| v.as_u64()).unwrap_or(80) as u16;
    let rows = params.get("rows").and_then(|v| v.as_u64()).unwrap_or(24) as u16;

    let cols = cols.clamp(MIN_TERMINAL_COLS, MAX_TERMINAL_COLS);
    let rows = rows.clamp(MIN_TERMINAL_ROWS, MAX_TERMINAL_ROWS);

    match session_manager.spawn(command, &args, cwd, None, session_id, cols, rows) {
        Ok((session_id, pid)) => RpcResponse::success(
            request.id,
            json!({
                "session_id": session_id,
                "pid": pid
            }),
        ),
        Err(SessionError::LimitReached(max)) => {
            let err = DomainError::SessionLimitReached { max };
            domain_error_response(request.id, &err)
        }
        Err(e) => {
            let err_str = e.to_string();
            let domain_err = if err_str.contains("No such file") || err_str.contains("not found") {
                DomainError::CommandNotFound {
                    command: command.to_string(),
                }
            } else if err_str.contains("Permission denied") {
                DomainError::PermissionDenied {
                    command: command.to_string(),
                }
            } else {
                DomainError::PtyError {
                    operation: "spawn".to_string(),
                    reason: err_str,
                }
            };
            domain_error_response(request.id, &domain_err)
        }
    }
}

pub fn handle_kill(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let session_id = request.param_str("session");

    let session_to_kill = match session_id {
        Some(id) => id.to_string(),
        None => match session_manager.active_session_id() {
            Some(id) => id.to_string(),
            None => return domain_error_response(request.id, &DomainError::NoActiveSession),
        },
    };

    match session_manager.kill(&session_to_kill) {
        Ok(()) => RpcResponse::success(
            request.id,
            json!({
                "success": true,
                "session_id": session_to_kill
            }),
        ),
        Err(e) => domain_error_response(request.id, &DomainError::from(e)),
    }
}

pub fn handle_restart(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let session_id = request.param_str("session");

    let (old_session_id, command, cols, rows) = match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(request.id, session_id);
            };
            let (cols, rows) = sess.size();
            (sess.id.to_string(), sess.command.clone(), cols, rows)
        }
        Err(e) => {
            return domain_error_response(request.id, &DomainError::from(e));
        }
    };

    if let Err(e) = session_manager.kill(&old_session_id) {
        return domain_error_response(request.id, &DomainError::from(e));
    }

    match session_manager.spawn(&command, &[], None, None, None, cols, rows) {
        Ok((new_session_id, pid)) => RpcResponse::success(
            request.id,
            json!({
                "success": true,
                "old_session_id": old_session_id,
                "new_session_id": new_session_id,
                "command": command,
                "pid": pid
            }),
        ),
        Err(e) => domain_error_response(request.id, &DomainError::from(e)),
    }
}

pub fn handle_sessions(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let sessions = session_manager.list();
    let active_id = session_manager.active_session_id();

    RpcResponse::success(
        request.id,
        json!({
            "sessions": sessions.iter().map(|s| s.to_json()).collect::<Vec<_>>(),
            "active_session": active_id
        }),
    )
}

pub fn handle_resize(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let cols = request
        .param_u16("cols", 80)
        .clamp(MIN_TERMINAL_COLS, MAX_TERMINAL_COLS);
    let rows = request
        .param_u16("rows", 24)
        .clamp(MIN_TERMINAL_ROWS, MAX_TERMINAL_ROWS);
    let session_id = request.param_str("session");
    let req_id = request.id;

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };
            match sess.resize(cols, rows) {
                Ok(()) => RpcResponse::success(
                    req_id,
                    json!({
                        "success": true,
                        "session_id": sess.id,
                        "size": { "cols": cols, "rows": rows }
                    }),
                ),
                Err(e) => {
                    let err = DomainError::PtyError {
                        operation: "resize".to_string(),
                        reason: e.to_string(),
                    };
                    domain_error_response(req_id, &err)
                }
            }
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}

pub fn handle_attach(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    let session_id = match request.require_str("session") {
        Ok(s) => s,
        Err(resp) => return resp,
    };

    match session_manager.resolve(Some(session_id)) {
        Ok(session) => {
            let Some(mut sess) = acquire_session_lock(&session, Duration::from_millis(100)) else {
                return lock_timeout_response(request.id, Some(session_id));
            };
            if sess.is_running() {
                let _ = session_manager.set_active(session_id);
                RpcResponse::success(
                    request.id,
                    json!({
                        "success": true,
                        "session_id": session_id,
                        "message": format!("Now attached to session {}", session_id)
                    }),
                )
            } else {
                let err = DomainError::Generic {
                    message: format!("Session {} is not running", session_id),
                };
                domain_error_response(request.id, &err)
            }
        }
        Err(e) => domain_error_response(request.id, &DomainError::from(e)),
    }
}
