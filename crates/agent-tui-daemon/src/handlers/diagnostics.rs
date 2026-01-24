use agent_tui_ipc::{RpcRequest, RpcResponse};

use super::common::session_error_response;
use crate::adapters::{
    console_output_to_response, errors_output_to_response, health_output_to_response,
    metrics_output_to_response, parse_console_input, parse_errors_input, parse_pty_read_input,
    parse_pty_write_input, parse_trace_input, pty_read_output_to_response,
    pty_write_output_to_response, trace_output_to_response,
};
use crate::domain::{HealthInput, MetricsInput};
use crate::usecases::{
    ConsoleUseCase, ErrorsUseCase, HealthUseCase, MetricsUseCase, PtyReadUseCase, PtyWriteUseCase,
    TraceUseCase,
};

/// Handle health requests using the use case pattern.
pub fn handle_health_uc<U: HealthUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let req_id = request.id;
    let input = HealthInput;

    match usecase.execute(input) {
        Ok(output) => health_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}

/// Handle metrics requests using the use case pattern.
pub fn handle_metrics_uc<U: MetricsUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let req_id = request.id;
    let input = MetricsInput;

    match usecase.execute(input) {
        Ok(output) => metrics_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}

/// Handle trace requests using the use case pattern.
pub fn handle_trace_uc<U: TraceUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let req_id = request.id;
    let input = parse_trace_input(&request);

    match usecase.execute(input) {
        Ok(output) => trace_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}

/// Handle console requests using the use case pattern.
pub fn handle_console_uc<U: ConsoleUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let req_id = request.id;
    let input = parse_console_input(&request);

    match usecase.execute(input) {
        Ok(output) => console_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}

/// Handle errors requests using the use case pattern.
pub fn handle_errors_uc<U: ErrorsUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let req_id = request.id;
    let input = parse_errors_input(&request);

    match usecase.execute(input) {
        Ok(output) => errors_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}

/// Handle pty_read requests using the use case pattern.
pub fn handle_pty_read_uc<U: PtyReadUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let req_id = request.id;
    let input = parse_pty_read_input(&request);

    match usecase.execute(input) {
        Ok(output) => pty_read_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}

/// Handle pty_write requests using the use case pattern.
pub fn handle_pty_write_uc<U: PtyWriteUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let req_id = request.id;
    let input = match parse_pty_write_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };

    match usecase.execute(input) {
        Ok(output) => pty_write_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}
