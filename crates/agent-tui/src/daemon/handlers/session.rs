use crate::ipc::{RpcRequest, RpcResponse};

use crate::daemon::adapters::{
    assert_output_to_response, attach_output_to_response, cleanup_output_to_response,
    domain_error_response, kill_output_to_response, parse_assert_input, parse_attach_input,
    parse_cleanup_input, parse_resize_input, parse_session_input, parse_spawn_input,
    resize_output_to_response, restart_output_to_response, session_error_response,
    sessions_output_to_response, spawn_output_to_response,
};
use crate::daemon::error::DomainError;
use crate::daemon::session::SessionError;
use crate::daemon::usecases::{
    AssertUseCase, AttachUseCase, CleanupUseCase, KillUseCase, ResizeUseCase, RestartUseCase,
    SessionsUseCase, SpawnUseCase,
};

pub fn handle_spawn<U: SpawnUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let input = match parse_spawn_input(&request) {
        Ok(input) => input,
        Err(resp) => return resp,
    };

    match usecase.execute(input) {
        Ok(output) => spawn_output_to_response(request.id, output),
        Err(e) => domain_error_response(request.id, &e),
    }
}

pub fn handle_kill<U: KillUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
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
    let input = parse_session_input(&request);

    match usecase.execute(input) {
        Ok(output) => restart_output_to_response(request.id, output),
        Err(e) => session_error_response(request.id, e),
    }
}

pub fn handle_sessions<U: SessionsUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let output = usecase.execute();
    sessions_output_to_response(request.id, output)
}

pub fn handle_resize<U: ResizeUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let input = parse_resize_input(&request);

    match usecase.execute(input) {
        Ok(output) => resize_output_to_response(request.id, output),
        Err(e) => session_error_response(request.id, e),
    }
}

pub fn handle_attach<U: AttachUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let req_id = request.id;
    let input = match parse_attach_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };

    match usecase.execute(input) {
        Ok(output) => attach_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_cleanup<U: CleanupUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let input = parse_cleanup_input(&request);
    let output = usecase.execute(input);
    cleanup_output_to_response(request.id, output)
}

pub fn handle_assert<U: AssertUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
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
