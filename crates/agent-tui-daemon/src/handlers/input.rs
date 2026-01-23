use agent_tui_ipc::{RpcRequest, RpcResponse};

use super::common::session_error_response;
use crate::domain::{KeydownInput, KeystrokeInput, KeyupInput, TypeInput};
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
