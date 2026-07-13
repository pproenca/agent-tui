//! Snapshot handler.

use crate::adapters::rpc::RpcRequest;
use crate::adapters::rpc::RpcResponse;

use super::common;
use super::common::session_error_response;
use crate::adapters::parse_snapshot_input;
use crate::adapters::snapshot_output_to_response;
use crate::usecases::ports::SessionRepository;
use crate::usecases::snapshot;

pub fn handle_snapshot_uc<R: SessionRepository + ?Sized>(
    repository: &R,
    request: RpcRequest,
) -> RpcResponse {
    let _span = common::handler_span(&request, "snapshot").entered();
    let input = match parse_snapshot_input(&request) {
        Ok(input) => input,
        Err(response) => return response,
    };
    let strip_ansi = input.strip_ansi;
    let retain_ansi = input.retain_ansi;

    match snapshot::snapshot(repository, input) {
        Ok(output) => snapshot_output_to_response(request.id, output, strip_ansi, retain_ansi),
        Err(e) => session_error_response(request.id, e),
    }
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;
