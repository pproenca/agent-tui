use agent_tui_ipc::{RpcRequest, RpcResponse};
use serde_json::{Value, json};

use super::common::session_error_response;
use crate::domain::{RecordStartInput, RecordStatusInput, RecordStopInput};
use crate::session::RecordingFrame;
use crate::usecases::{RecordStartUseCase, RecordStatusUseCase, RecordStopUseCase};

fn build_asciicast(session_id: &str, cols: u16, rows: u16, frames: &[RecordingFrame]) -> Value {
    let mut output = Vec::new();

    let duration = frames
        .last()
        .map(|f| f.timestamp_ms as f64 / 1000.0)
        .unwrap_or(0.0);

    let header = json!({
        "version": 2,
        "width": cols,
        "height": rows,
        "timestamp": chrono::Utc::now().timestamp(),
        "duration": duration,
        "title": format!("agent-tui recording - {}", session_id),
        "env": {
            "TERM": "xterm-256color",
            "SHELL": std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
        }
    });

    match serde_json::to_string(&header) {
        Ok(s) => output.push(s),
        Err(e) => {
            eprintln!("Error: Failed to serialize asciicast header: {}", e);
            return json!({
                "format": "asciicast",
                "version": 2,
                "error": format!("Failed to serialize recording header: {}", e)
            });
        }
    }

    let mut prev_screen = String::new();
    for frame in frames {
        let time_secs = frame.timestamp_ms as f64 / 1000.0;
        if frame.screen != prev_screen {
            let screen_data = if prev_screen.is_empty() {
                frame.screen.clone()
            } else {
                format!("\x1b[2J\x1b[H{}", frame.screen)
            };
            let event = json!([time_secs, "o", screen_data]);
            match serde_json::to_string(&event) {
                Ok(s) => output.push(s),
                Err(e) => {
                    eprintln!("Error: Failed to serialize asciicast frame: {}", e);
                }
            }
            prev_screen = frame.screen.clone();
        }
    }

    json!({
        "format": "asciicast",
        "version": 2,
        "data": output.join("\n")
    })
}

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
                "session_id": output.session_id,
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
                let frame_data: Vec<_> = output
                    .frames
                    .iter()
                    .map(|f| {
                        json!({
                            "timestamp_ms": f.timestamp_ms,
                            "screen": f.screen
                        })
                    })
                    .collect();
                json!({ "frames": frame_data, "frame_count": output.frame_count })
            };

            RpcResponse::success(
                req_id,
                json!({
                    "success": true,
                    "session_id": output.session_id,
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
