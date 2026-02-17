//! Session handler.

use super::common;
use crate::adapters::rpc::RpcRequest;
use crate::adapters::rpc::RpcResponse;

use crate::adapters::assert_output_to_response;
use crate::adapters::attach_output_to_response;
use crate::adapters::cleanup_output_to_response;
use crate::adapters::daemon::DomainError;
use crate::adapters::domain_error_response;
use crate::adapters::kill_output_to_response;
use crate::adapters::parse_assert_input;
use crate::adapters::parse_attach_input;
use crate::adapters::parse_cleanup_input;
use crate::adapters::parse_resize_input;
use crate::adapters::parse_session_input;
use crate::adapters::parse_spawn_input;
use crate::adapters::resize_output_to_response;
use crate::adapters::restart_output_to_response;
use crate::adapters::session_error_response;
use crate::adapters::sessions_output_to_response;
use crate::adapters::spawn_output_to_response;
use crate::usecases::AssertUseCase;
use crate::usecases::AttachUseCase;
use crate::usecases::CleanupUseCase;
use crate::usecases::KillUseCase;
use crate::usecases::ResizeUseCase;
use crate::usecases::RestartUseCase;
use crate::usecases::SessionsUseCase;
use crate::usecases::SpawnUseCase;
use crate::usecases::ports::SessionError;

pub fn handle_spawn<U: SpawnUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "spawn").entered();
    let input = match parse_spawn_input(&request) {
        Ok(input) => input,
        Err(resp) => return resp,
    };

    match usecase.execute(input) {
        Ok(output) => spawn_output_to_response(request.id, output),
        Err(e) => domain_error_response(request.id, &DomainError::from(e)),
    }
}

pub fn handle_kill<U: KillUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "kill").entered();
    let input = parse_session_input(&request);

    match usecase.execute(input) {
        Ok(output) => kill_output_to_response(request.id, output),
        Err(SessionError::NoActiveSession) => {
            domain_error_response(request.id, &DomainError::NoActiveSession)
        }
        Err(e) => session_error_response(request.id, e),
    }
}

pub fn handle_restart<U: RestartUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "restart").entered();
    let input = parse_session_input(&request);

    match usecase.execute(input) {
        Ok(output) => restart_output_to_response(request.id, output),
        Err(e) => session_error_response(request.id, e),
    }
}

pub fn handle_sessions<U: SessionsUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "sessions").entered();
    let output = usecase.execute();
    sessions_output_to_response(request.id, output)
}

pub fn handle_resize<U: ResizeUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "resize").entered();
    let input = parse_resize_input(&request);

    match usecase.execute(input) {
        Ok(output) => resize_output_to_response(request.id, output),
        Err(e) => session_error_response(request.id, e),
    }
}

pub fn handle_attach<U: AttachUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "attach").entered();
    let req_id = request.id;
    let input = match parse_attach_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };

    match usecase.execute(input) {
        Ok(output) => attach_output_to_response(req_id, &output),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_cleanup<U: CleanupUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "cleanup").entered();
    let input = parse_cleanup_input(&request);
    let output = usecase.execute(input);
    cleanup_output_to_response(request.id, output)
}

pub fn handle_assert<U: AssertUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "assert").entered();
    let req_id = request.id;
    let input = match parse_assert_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };

    match usecase.execute(input) {
        Ok(output) => assert_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}
