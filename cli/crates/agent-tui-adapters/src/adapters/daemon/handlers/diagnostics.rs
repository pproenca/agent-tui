//! Diagnostics handler.

use crate::adapters::rpc::RpcRequest;
use crate::adapters::rpc::RpcResponse;

use super::common;
use super::common::session_error_response;
use crate::adapters::parse_terminal_write_input;
use crate::adapters::shutdown_output_to_response;
use crate::adapters::terminal_write_output_to_response;
use crate::domain::ShutdownInput;
use crate::usecases::ShutdownUseCase;
use crate::usecases::TerminalWriteUseCase;

pub fn handle_terminal_write_uc<U: TerminalWriteUseCase>(
    usecase: &U,
    request: RpcRequest,
) -> RpcResponse {
    let _span = common::handler_span(&request, "pty_write").entered();
    let req_id = request.id.clone();
    let input = match parse_terminal_write_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };

    match usecase.execute(input) {
        Ok(output) => terminal_write_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_shutdown_uc<U: ShutdownUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "shutdown").entered();
    let req_id = request.id;
    let output = usecase.execute(ShutdownInput);
    shutdown_output_to_response(req_id, output)
}

#[cfg(test)]
#[path = "diagnostics_tests.rs"]
mod tests;
