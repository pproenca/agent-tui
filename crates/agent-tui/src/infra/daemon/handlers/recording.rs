use crate::infra::ipc::{RpcRequest, RpcResponse};

use super::common::session_error_response;
use crate::adapters::{
    parse_record_start_input, parse_record_status_input, parse_record_stop_input,
    record_start_output_to_response, record_status_output_to_response,
    record_stop_output_to_response,
};
use crate::usecases::{RecordStartUseCase, RecordStatusUseCase, RecordStopUseCase};

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
