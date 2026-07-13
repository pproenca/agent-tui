//! Input handler.

use crate::adapters::rpc::RpcRequest;
use crate::adapters::rpc::RpcResponse;

use super::common;
use super::common::session_error_response;
use crate::adapters::parse_keydown_input;
use crate::adapters::parse_keystroke_input;
use crate::adapters::parse_keyup_input;
use crate::adapters::parse_type_input;
use crate::usecases::input as input_usecase;
use crate::usecases::ports::SessionRepository;

pub fn handle_keystroke_uc<R: SessionRepository + ?Sized>(
    repository: &R,
    request: RpcRequest,
) -> RpcResponse {
    let _span = common::handler_span(&request, "keystroke").entered();
    let req_id = request.id.clone();
    let input = match parse_keystroke_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };

    match input_usecase::keystroke(repository, input) {
        Ok(()) => RpcResponse::success(req_id, serde_json::Value::Null),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_type_uc<R: SessionRepository + ?Sized>(
    repository: &R,
    request: RpcRequest,
) -> RpcResponse {
    let _span = common::handler_span(&request, "type").entered();
    let req_id = request.id.clone();
    let input = match parse_type_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };

    match input_usecase::type_text(repository, input) {
        Ok(()) => RpcResponse::success(req_id, serde_json::Value::Null),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_keydown_uc<R: SessionRepository + ?Sized>(
    repository: &R,
    request: RpcRequest,
) -> RpcResponse {
    let _span = common::handler_span(&request, "keydown").entered();
    let req_id = request.id.clone();
    let input = match parse_keydown_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };

    match input_usecase::keydown(repository, input) {
        Ok(()) => RpcResponse::success(req_id, serde_json::Value::Null),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_keyup_uc<R: SessionRepository + ?Sized>(
    repository: &R,
    request: RpcRequest,
) -> RpcResponse {
    let _span = common::handler_span(&request, "keyup").entered();
    let req_id = request.id.clone();
    let input = match parse_keyup_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };

    match input_usecase::keyup(repository, input) {
        Ok(()) => RpcResponse::success(req_id, serde_json::Value::Null),
        Err(e) => session_error_response(req_id, e),
    }
}
