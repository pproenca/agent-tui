use crate::adapters::rpc::RpcRequest;
use crate::adapters::rpc::RpcResponse;

use super::common;
use super::common::session_error_response;
use crate::adapters::parse_wait_input;
use crate::adapters::wait_output_to_response;
use crate::usecases::WaitUseCase;

pub fn handle_wait_uc<U: WaitUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "wait").entered();
    let input = match parse_wait_input(&request) {
        Ok(input) => input,
        Err(response) => return response,
    };
    let req_id = request.id;

    match usecase.execute(input) {
        Ok(output) => wait_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}
