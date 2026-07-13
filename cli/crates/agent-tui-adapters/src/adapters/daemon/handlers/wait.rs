//! Wait handler.

use crate::adapters::rpc::RpcRequest;
use crate::adapters::rpc::RpcResponse;

use super::common;
use super::common::session_error_response;
use crate::adapters::parse_wait_input;
use crate::adapters::wait_output_to_response;
use crate::usecases::ports::Clock;
use crate::usecases::ports::SessionRepository;
use crate::usecases::wait;

pub fn handle_wait_uc<R, C>(repository: &R, clock: &C, request: RpcRequest) -> RpcResponse
where
    R: SessionRepository + ?Sized,
    C: Clock + ?Sized,
{
    let _span = common::handler_span(&request, "wait").entered();
    let input = match parse_wait_input(&request) {
        Ok(input) => input,
        Err(response) => return response,
    };
    let req_id = request.id;

    match wait::wait(repository, clock, input) {
        Ok(output) => wait_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}
