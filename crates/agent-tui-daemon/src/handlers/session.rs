use agent_tui_ipc::{RpcRequest, RpcResponse};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

use crate::adapters::{
    domain_error_response, lock_timeout_response, parse_spawn_input, spawn_output_to_response,
};
use crate::domain::SpawnOutput;
use crate::error::DomainError;
use crate::lock_helpers::{LOCK_TIMEOUT, acquire_session_lock};
use crate::repository::SessionRepository;
use crate::session::SessionError;

/// Handle spawn requests using the repository pattern.
pub fn handle_spawn<R: SessionRepository>(repository: &Arc<R>, request: RpcRequest) -> RpcResponse {
    let input = match parse_spawn_input(&request) {
        Ok(input) => input,
        Err(resp) => return resp,
    };

    match repository.spawn(
        &input.command,
        &input.args,
        input.cwd.as_deref(),
        input.env.as_ref(),
        input.session_id,
        input.cols,
        input.rows,
    ) {
        Ok((session_id, pid)) => {
            spawn_output_to_response(request.id, SpawnOutput { session_id, pid })
        }
        Err(SessionError::LimitReached(max)) => {
            let err = DomainError::SessionLimitReached { max };
            domain_error_response(request.id, &err)
        }
        Err(e) => {
            let err_str = e.to_string();
            let domain_err = if err_str.contains("No such file") || err_str.contains("not found") {
                DomainError::CommandNotFound {
                    command: input.command,
                }
            } else if err_str.contains("Permission denied") {
                DomainError::PermissionDenied {
                    command: input.command,
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

/// Handle kill requests using the repository pattern.
pub fn handle_kill<R: SessionRepository>(repository: &Arc<R>, request: RpcRequest) -> RpcResponse {
    let session_id = request.param_str("session");

    let session_to_kill = match session_id {
        Some(id) => id.to_string(),
        None => match repository.active_session_id() {
            Some(id) => id.to_string(),
            None => return domain_error_response(request.id, &DomainError::NoActiveSession),
        },
    };

    match repository.kill(&session_to_kill) {
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

/// Handle restart requests using the repository pattern.
pub fn handle_restart<R: SessionRepository>(
    repository: &Arc<R>,
    request: RpcRequest,
) -> RpcResponse {
    let session_id = request.param_str("session");

    let (old_session_id, command, cols, rows) = match repository.resolve(session_id) {
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

    if let Err(e) = repository.kill(&old_session_id) {
        return domain_error_response(request.id, &DomainError::from(e));
    }

    match repository.spawn(&command, &[], None, None, None, cols, rows) {
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

/// Handle sessions list requests using the repository pattern.
pub fn handle_sessions<R: SessionRepository>(
    repository: &Arc<R>,
    request: RpcRequest,
) -> RpcResponse {
    let sessions = repository.list();
    let active_id = repository.active_session_id();

    RpcResponse::success(
        request.id,
        json!({
            "sessions": sessions.iter().map(|s| s.to_json()).collect::<Vec<_>>(),
            "active_session": active_id
        }),
    )
}

const MAX_TERMINAL_COLS: u16 = 500;
const MAX_TERMINAL_ROWS: u16 = 200;
const MIN_TERMINAL_COLS: u16 = 10;
const MIN_TERMINAL_ROWS: u16 = 5;

/// Handle resize requests using the repository pattern.
pub fn handle_resize<R: SessionRepository>(
    repository: &Arc<R>,
    request: RpcRequest,
) -> RpcResponse {
    let cols = request
        .param_u16("cols", 80)
        .clamp(MIN_TERMINAL_COLS, MAX_TERMINAL_COLS);
    let rows = request
        .param_u16("rows", 24)
        .clamp(MIN_TERMINAL_ROWS, MAX_TERMINAL_ROWS);
    let session_id = request.param_str("session");
    let req_id = request.id;

    match repository.resolve(session_id) {
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

/// Handle attach requests using the repository pattern.
pub fn handle_attach<R: SessionRepository>(
    repository: &Arc<R>,
    request: RpcRequest,
) -> RpcResponse {
    let session_id = match request.require_str("session") {
        Ok(s) => s,
        Err(resp) => return resp,
    };

    match repository.resolve(Some(session_id)) {
        Ok(session) => {
            let Some(mut sess) = acquire_session_lock(&session, Duration::from_millis(100)) else {
                return lock_timeout_response(request.id, Some(session_id));
            };
            if sess.is_running() {
                let _ = repository.set_active(session_id);
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
