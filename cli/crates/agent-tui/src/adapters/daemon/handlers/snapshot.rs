use crate::adapters::ipc::{RpcRequest, RpcResponse};

use super::common;
use super::common::session_error_response;
use crate::adapters::accessibility_snapshot_output_to_response;
use crate::adapters::{
    parse_accessibility_snapshot_input, parse_snapshot_input, snapshot_output_to_response,
};
use crate::usecases::{AccessibilitySnapshotUseCase, SnapshotUseCase};

pub fn handle_snapshot_uc<U: SnapshotUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "snapshot").entered();
    let input = parse_snapshot_input(&request);
    let strip_ansi = input.strip_ansi;

    match usecase.execute(input) {
        Ok(output) => snapshot_output_to_response(request.id, output, strip_ansi),
        Err(e) => session_error_response(request.id, e),
    }
}

pub fn handle_accessibility_snapshot_uc<U: AccessibilitySnapshotUseCase>(
    usecase: &U,
    request: RpcRequest,
) -> RpcResponse {
    let _span = common::handler_span(&request, "accessibility_snapshot").entered();
    let input = parse_accessibility_snapshot_input(&request);

    match usecase.execute(input) {
        Ok(output) => accessibility_snapshot_output_to_response(request.id, output),
        Err(e) => session_error_response(request.id, e),
    }
}
