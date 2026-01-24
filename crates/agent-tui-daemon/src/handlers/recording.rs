use agent_tui_ipc::{RpcRequest, RpcResponse};
use serde_json::json;

use super::common::session_error_response;
use crate::adapters::{build_asciicast, build_raw_frames};
use crate::domain::{RecordStartInput, RecordStatusInput, RecordStopInput};
use crate::usecases::{RecordStartUseCase, RecordStatusUseCase, RecordStopUseCase};

/// Handle record_start requests using the use case pattern.
pub fn handle_record_start_uc<U: RecordStartUseCase>(
    usecase: &U,
    request: RpcRequest,
) -> RpcResponse {
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = RecordStartInput { session_id };

    match usecase.execute(input) {
        Ok(output) => RpcResponse::success(
            req_id,
            json!({
                "success": output.success,
                "session_id": output.session_id.as_str(),
                "recording": true
            }),
        ),
        Err(e) => session_error_response(req_id, e),
    }
}

/// Handle record_stop requests using the use case pattern.
pub fn handle_record_stop_uc<U: RecordStopUseCase>(
    usecase: &U,
    request: RpcRequest,
) -> RpcResponse {
    let session_id = request.param_str("session").map(String::from);
    let format = request.param_str("format").map(String::from);
    let req_id = request.id;

    let input = RecordStopInput { session_id, format };

    match usecase.execute(input) {
        Ok(output) => {
            let recording_data = if output.format == "asciicast" {
                build_asciicast(
                    output.session_id.as_ref(),
                    output.cols,
                    output.rows,
                    &output.frames,
                )
            } else {
                build_raw_frames(&output.frames)
            };

            RpcResponse::success(
                req_id,
                json!({
                    "success": true,
                    "session_id": output.session_id.as_str(),
                    "frame_count": output.frame_count,
                    "recording": recording_data
                }),
            )
        }
        Err(e) => session_error_response(req_id, e),
    }
}

/// Handle record_status requests using the use case pattern.
pub fn handle_record_status_uc<U: RecordStatusUseCase>(
    usecase: &U,
    request: RpcRequest,
) -> RpcResponse {
    let session_id = request.param_str("session").map(String::from);
    let req_id = request.id;

    let input = RecordStatusInput { session_id };

    match usecase.execute(input) {
        Ok(status) => RpcResponse::success(
            req_id,
            json!({
                "recording": status.is_recording,
                "frame_count": status.frame_count,
                "duration_ms": status.duration_ms
            }),
        ),
        Err(e) => session_error_response(req_id, e),
    }
}
