use agent_tui_ipc::{RpcRequest, RpcResponse};
use serde_json::json;

use super::common::session_error_response;
use crate::adapters::parse_wait_input;
use crate::usecases::WaitUseCase;

/// Handle wait requests using the use case pattern.
pub fn handle_wait_uc<U: WaitUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let input = parse_wait_input(&request);
    let req_id = request.id;

    match usecase.execute(input) {
        Ok(output) => RpcResponse::success(
            req_id,
            json!({
                "found": output.found,
                "elapsed_ms": output.elapsed_ms
            }),
        ),
        Err(e) => session_error_response(req_id, e),
    }
}
