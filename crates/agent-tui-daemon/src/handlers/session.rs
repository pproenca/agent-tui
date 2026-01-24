use agent_tui_ipc::{RpcRequest, RpcResponse};

use crate::adapters::{
    attach_output_to_response, domain_error_response, kill_output_to_response, parse_attach_input,
    parse_resize_input, parse_session_input, parse_spawn_input, restart_output_to_response,
    session_error_response, sessions_output_to_response, spawn_output_to_response,
};
use crate::domain::ResizeOutput;
use crate::error::DomainError;
use crate::session::SessionError;
use crate::usecases::{
    AttachUseCase, KillUseCase, ResizeUseCase, RestartUseCase, SessionsUseCase, SpawnUseCase,
};

/// Handle spawn requests using the use case pattern.
///
/// This handler is a thin coordinator that:
/// 1. Parses the RPC request into a domain input using adapters
/// 2. Delegates to the use case for business logic (including error classification)
/// 3. Converts the result to an RPC response using adapters
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

/// Handle kill requests using the use case pattern.
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

/// Handle restart requests using the use case pattern.
pub fn handle_restart<U: RestartUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let input = parse_session_input(&request);

    match usecase.execute(input) {
        Ok(output) => restart_output_to_response(request.id, output),
        Err(e) => session_error_response(request.id, e),
    }
}

/// Handle sessions list requests using the use case pattern.
pub fn handle_sessions<U: SessionsUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let output = usecase.execute();
    sessions_output_to_response(request.id, output)
}

/// Handle resize requests using the use case pattern.
pub fn handle_resize<U: ResizeUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let input = parse_resize_input(&request);
    let (cols, rows) = (input.cols, input.rows);

    match usecase.execute(input) {
        Ok(output) => {
            // Build response with actual dimensions
            let full_output = ResizeOutput {
                session_id: output.session_id,
                success: output.success,
            };
            use serde_json::json;
            RpcResponse::success(
                request.id,
                json!({
                    "success": full_output.success,
                    "session_id": full_output.session_id.as_str(),
                    "size": { "cols": cols, "rows": rows }
                }),
            )
        }
        Err(e) => session_error_response(request.id, e),
    }
}

/// Handle attach requests using the use case pattern.
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
