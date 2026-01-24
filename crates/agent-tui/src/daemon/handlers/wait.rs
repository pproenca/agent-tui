use crate::ipc::{RpcRequest, RpcResponse};

use super::common::session_error_response;
use crate::daemon::adapters::{parse_wait_input, wait_output_to_response};
use crate::daemon::usecases::WaitUseCase;

/// Handle wait requests using the use case pattern.
pub fn handle_wait_uc<U: WaitUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let input = parse_wait_input(&request);
    let req_id = request.id;

    match usecase.execute(input) {
        Ok(output) => wait_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}
