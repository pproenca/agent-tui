use agent_tui_ipc::{RpcRequest, RpcResponse};
use std::sync::Arc;

use super::common::{domain_error_response, lock_timeout_response};
use crate::error::DomainError;
use crate::lock_helpers::{LOCK_TIMEOUT, acquire_session_lock};
use crate::session::SessionManager;

pub fn handle_keystroke(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    with_session_action(session_manager, &request, "key", |sess, key| {
        sess.keystroke(key).map_err(|e| e.into())
    })
}

pub fn handle_keydown(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    with_session_action(session_manager, &request, "key", |sess, key| {
        sess.keydown(key).map_err(|e| e.into())
    })
}

pub fn handle_keyup(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    with_session_action(session_manager, &request, "key", |sess, key| {
        sess.keyup(key).map_err(|e| e.into())
    })
}

pub fn handle_type(session_manager: &Arc<SessionManager>, request: RpcRequest) -> RpcResponse {
    with_session_action(session_manager, &request, "text", |sess, text| {
        sess.type_text(text).map_err(|e| e.into())
    })
}

fn with_session_action<F>(
    session_manager: &Arc<SessionManager>,
    request: &RpcRequest,
    param: &str,
    f: F,
) -> RpcResponse
where
    F: FnOnce(&mut crate::session::Session, &str) -> Result<(), Box<dyn std::error::Error>>,
{
    let req_id = request.id;
    let value = match request.require_str(param) {
        Ok(v) => v.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session");

    match session_manager.resolve(session_id) {
        Ok(session) => {
            let Some(mut sess) = acquire_session_lock(&session, LOCK_TIMEOUT) else {
                return lock_timeout_response(req_id, session_id);
            };
            match f(&mut sess, &value) {
                Ok(()) => RpcResponse::action_success(req_id),
                Err(e) => {
                    let err_str = e.to_string();
                    let domain_err = if err_str.contains("Invalid key") {
                        DomainError::InvalidKey { key: value.clone() }
                    } else {
                        DomainError::PtyError {
                            operation: param.to_string(),
                            reason: err_str,
                        }
                    };
                    domain_error_response(req_id, &domain_err)
                }
            }
        }
        Err(e) => domain_error_response(req_id, &DomainError::from(e)),
    }
}
