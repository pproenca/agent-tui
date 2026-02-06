//! Snapshot handler.

use crate::adapters::rpc::RpcRequest;
use crate::adapters::rpc::RpcResponse;

use super::common;
use super::common::session_error_response;
use crate::adapters::parse_snapshot_input;
use crate::adapters::snapshot_output_to_response;
use crate::usecases::SnapshotUseCase;

pub fn handle_snapshot_uc<U: SnapshotUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "snapshot").entered();
    let input = parse_snapshot_input(&request);
    let strip_ansi = input.strip_ansi;

    match usecase.execute(input) {
        Ok(output) => snapshot_output_to_response(request.id, output, strip_ansi),
        Err(e) => session_error_response(request.id, e),
    }
}
