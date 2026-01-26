use crate::infra::ipc::{RpcRequest, RpcResponse};

use super::common;
use super::common::session_error_response;
use crate::adapters::{
    parse_keydown_input, parse_keystroke_input, parse_keyup_input, parse_type_input,
};
use crate::usecases::{KeydownUseCase, KeystrokeUseCase, KeyupUseCase, TypeUseCase};

pub fn handle_keystroke_uc<U: KeystrokeUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "keystroke").entered();
    let req_id = request.id;
    let input = match parse_keystroke_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };

    match usecase.execute(input) {
        Ok(_) => RpcResponse::action_success(req_id),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_type_uc<U: TypeUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "type").entered();
    let req_id = request.id;
    let input = match parse_type_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };

    match usecase.execute(input) {
        Ok(_) => RpcResponse::action_success(req_id),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_keydown_uc<U: KeydownUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "keydown").entered();
    let req_id = request.id;
    let input = match parse_keydown_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };

    match usecase.execute(input) {
        Ok(_) => RpcResponse::action_success(req_id),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_keyup_uc<U: KeyupUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "keyup").entered();
    let req_id = request.id;
    let input = match parse_keyup_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };

    match usecase.execute(input) {
        Ok(_) => RpcResponse::action_success(req_id),
        Err(e) => session_error_response(req_id, e),
    }
}
