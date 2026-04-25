//! Input handler.

use crate::adapters::rpc::RpcRequest;
use crate::adapters::rpc::RpcResponse;

use super::common;
use super::common::session_error_response;
use crate::adapters::parse_keydown_input;
use crate::adapters::parse_keystroke_input;
use crate::adapters::parse_keyup_input;
use crate::adapters::parse_type_input;
use crate::adapters::parse_mouse_click_input;
use crate::adapters::parse_mouse_move_input;
use crate::adapters::parse_mouse_down_input;
use crate::adapters::parse_mouse_up_input;
use crate::usecases::KeydownUseCase;
use crate::usecases::KeystrokeUseCase;
use crate::usecases::KeyupUseCase;
use crate::usecases::TypeUseCase;
use crate::usecases::MouseClickUseCase;
use crate::usecases::MouseMoveUseCase;
use crate::usecases::MouseDownUseCase;
use crate::usecases::MouseUpUseCase;

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

pub fn handle_mouse_click_uc<U: MouseClickUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "mouse_click").entered();
    let req_id = request.id;
    let input = match parse_mouse_click_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };

    match usecase.execute(input) {
        Ok(_) => RpcResponse::action_success(req_id),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_mouse_move_uc<U: MouseMoveUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "mouse_move").entered();
    let req_id = request.id;
    let input = match parse_mouse_move_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };

    match usecase.execute(input) {
        Ok(_) => RpcResponse::action_success(req_id),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_mouse_down_uc<U: MouseDownUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "mouse_down").entered();
    let req_id = request.id;
    let input = match parse_mouse_down_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };

    match usecase.execute(input) {
        Ok(_) => RpcResponse::action_success(req_id),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_mouse_up_uc<U: MouseUpUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "mouse_up").entered();
    let req_id = request.id;
    let input = match parse_mouse_up_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };

    match usecase.execute(input) {
        Ok(_) => RpcResponse::action_success(req_id),
        Err(e) => session_error_response(req_id, e),
    }
}
