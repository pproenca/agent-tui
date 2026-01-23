use agent_tui_ipc::{RpcRequest, RpcResponse};
use std::sync::Arc;

use super::common::{domain_error_response, lock_timeout_response, session_error_response};
use crate::domain::{KeydownInput, KeystrokeInput, KeyupInput, TypeInput};
use crate::error::DomainError;
use crate::lock_helpers::{LOCK_TIMEOUT, acquire_session_lock};
use crate::session::SessionManager;
use crate::usecases::{KeydownUseCase, KeystrokeUseCase, KeyupUseCase, TypeUseCase};

/// Handle keystroke requests using the use case pattern.
pub fn handle_keystroke_uc<U: KeystrokeUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let key = match request.require_str("key") {
        Ok(k) => k.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = KeystrokeInput { session_id, key };

    match usecase.execute(input) {
        Ok(_) => RpcResponse::action_success(req_id),
        Err(e) => session_error_response(req_id, e),
    }
}

/// Handle type requests using the use case pattern.
pub fn handle_type_uc<U: TypeUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let text = match request.require_str("text") {
        Ok(t) => t.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = TypeInput { session_id, text };

    match usecase.execute(input) {
        Ok(_) => RpcResponse::action_success(req_id),
        Err(e) => session_error_response(req_id, e),
    }
}

/// Handle keydown requests using the use case pattern.
pub fn handle_keydown_uc<U: KeydownUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let key = match request.require_str("key") {
        Ok(k) => k.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = KeydownInput { session_id, key };

    match usecase.execute(input) {
        Ok(_) => RpcResponse::action_success(req_id),
        Err(e) => session_error_response(req_id, e),
    }
}

/// Handle keyup requests using the use case pattern.
pub fn handle_keyup_uc<U: KeyupUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let key = match request.require_str("key") {
        Ok(k) => k.to_string(),
        Err(resp) => return resp,
    };
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = KeyupInput { session_id, key };

    match usecase.execute(input) {
        Ok(_) => RpcResponse::action_success(req_id),
        Err(e) => session_error_response(req_id, e),
    }
}

/// Legacy handle_keystroke using SessionManager directly.
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
