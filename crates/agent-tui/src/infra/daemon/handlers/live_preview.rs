use crate::infra::ipc::{RpcRequest, RpcResponse};

use crate::adapters::{
    live_preview_error_response, live_preview_start_output_to_response,
    live_preview_status_output_to_response, live_preview_stop_output_to_response,
    parse_live_preview_start_input,
};
use crate::usecases::{LivePreviewStartUseCase, LivePreviewStatusUseCase, LivePreviewStopUseCase};

pub fn handle_live_preview_start<U: LivePreviewStartUseCase>(
    usecase: &U,
    request: RpcRequest,
) -> RpcResponse {
    let input = match parse_live_preview_start_input(&request) {
        Ok(input) => input,
        Err(resp) => return resp,
    };

    match usecase.execute(input) {
        Ok(output) => live_preview_start_output_to_response(request.id, output),
        Err(err) => live_preview_error_response(request.id, err),
    }
}

pub fn handle_live_preview_stop<U: LivePreviewStopUseCase>(
    usecase: &U,
    request: RpcRequest,
) -> RpcResponse {
    match usecase.execute() {
        Ok(output) => live_preview_stop_output_to_response(request.id, output),
        Err(err) => live_preview_error_response(request.id, err),
    }
}

pub fn handle_live_preview_status<U: LivePreviewStatusUseCase>(
    usecase: &U,
    request: RpcRequest,
) -> RpcResponse {
    let output = usecase.execute();
    live_preview_status_output_to_response(request.id, output)
}
