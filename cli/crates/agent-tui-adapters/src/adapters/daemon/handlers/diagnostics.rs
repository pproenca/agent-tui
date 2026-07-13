//! Diagnostics handler.

use crate::adapters::rpc::RpcRequest;
use crate::adapters::rpc::RpcResponse;

use super::common;
use super::common::session_error_response;
use crate::adapters::parse_terminal_write_input;
use crate::adapters::terminal_write_output_to_response;
use crate::domain::ShutdownInput;
use crate::usecases::diagnostics;
use crate::usecases::ports::SessionRepository;
use crate::usecases::ports::ShutdownNotifier;
use crate::usecases::shutdown;
use std::sync::atomic::AtomicBool;

pub fn handle_terminal_write_uc<R: SessionRepository + ?Sized>(
    repository: &R,
    request: RpcRequest,
) -> RpcResponse {
    let _span = common::handler_span(&request, "pty_write").entered();
    let req_id = request.id.clone();
    let input = match parse_terminal_write_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };

    match diagnostics::terminal_write(repository, input) {
        Ok(output) => terminal_write_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_shutdown_uc<N: ShutdownNotifier + ?Sized>(
    shutdown_flag: &AtomicBool,
    notifier: &N,
    request: RpcRequest,
) -> RpcResponse {
    let _span = common::handler_span(&request, "shutdown").entered();
    let req_id = request.id;
    match shutdown::shutdown(shutdown_flag, notifier, ShutdownInput) {
        Ok(()) => RpcResponse::success(req_id, serde_json::Value::Null),
        Err(error) => RpcResponse::error(
            req_id,
            -32603,
            &format!("Failed to notify daemon shutdown: {error}"),
        ),
    }
}

#[cfg(test)]
#[path = "diagnostics_tests.rs"]
mod tests;
