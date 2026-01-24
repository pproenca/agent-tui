use crate::ipc::{RpcRequest, RpcResponse};

use super::common::session_error_response;
use crate::daemon::adapters::{
    parse_record_start_input, parse_record_status_input, parse_record_stop_input,
    record_start_output_to_response, record_status_output_to_response,
    record_stop_output_to_response,
};
use crate::daemon::usecases::{RecordStartUseCase, RecordStatusUseCase, RecordStopUseCase};

/// Handle record_start requests using the use case pattern.
pub fn handle_record_start_uc<U: RecordStartUseCase>(
    usecase: &U,
    request: RpcRequest,
) -> RpcResponse {
    let req_id = request.id;
    let input = parse_record_start_input(&request);

    match usecase.execute(input) {
        Ok(output) => record_start_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}

/// Handle record_stop requests using the use case pattern.
pub fn handle_record_stop_uc<U: RecordStopUseCase>(
    usecase: &U,
    request: RpcRequest,
) -> RpcResponse {
    let req_id = request.id;
    let input = parse_record_stop_input(&request);

    match usecase.execute(input) {
        Ok(output) => record_stop_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}

/// Handle record_status requests using the use case pattern.
pub fn handle_record_status_uc<U: RecordStatusUseCase>(
    usecase: &U,
    request: RpcRequest,
) -> RpcResponse {
    let req_id = request.id;
    let input = parse_record_status_input(&request);

    match usecase.execute(input) {
        Ok(status) => record_status_output_to_response(req_id, status),
        Err(e) => session_error_response(req_id, e),
    }
}
