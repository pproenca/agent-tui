use crate::infra::ipc::{RpcRequest, RpcResponse};

use super::common::session_error_response;
use crate::adapters::{
    health_output_to_response, metrics_output_to_response, parse_pty_read_input,
    parse_pty_write_input, pty_read_output_to_response, pty_write_output_to_response,
    shutdown_output_to_response,
};
use crate::domain::{HealthInput, MetricsInput, ShutdownInput};
use crate::usecases::{
    HealthUseCase, MetricsUseCase, PtyReadUseCase, PtyWriteUseCase, ShutdownUseCase,
};

pub fn handle_health_uc<U: HealthUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let req_id = request.id;
    let input = HealthInput;

    match usecase.execute(input) {
        Ok(output) => health_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_metrics_uc<U: MetricsUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let req_id = request.id;
    let input = MetricsInput;

    match usecase.execute(input) {
        Ok(output) => metrics_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_pty_read_uc<U: PtyReadUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let req_id = request.id;
    let input = parse_pty_read_input(&request);

    match usecase.execute(input) {
        Ok(output) => pty_read_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}

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

pub fn handle_shutdown_uc<U: ShutdownUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let req_id = request.id;
    let output = usecase.execute(ShutdownInput);
    shutdown_output_to_response(req_id, output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ShutdownOutput;
    use crate::infra::ipc::RpcRequest;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct MockShutdownUseCase {
        shutdown_flag: Arc<AtomicBool>,
    }

    impl ShutdownUseCase for MockShutdownUseCase {
        fn execute(&self, _input: ShutdownInput) -> ShutdownOutput {
            self.shutdown_flag.store(true, Ordering::SeqCst);
            ShutdownOutput { acknowledged: true }
        }
    }

    #[test]
    fn test_handle_shutdown_uc_returns_acknowledged() {
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let usecase = MockShutdownUseCase {
            shutdown_flag: Arc::clone(&shutdown_flag),
        };

        let request = RpcRequest::new(1, "shutdown".to_string(), None);
        let response = handle_shutdown_uc(&usecase, request);

        let json_str = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert!(parsed.get("error").is_none() || parsed["error"].is_null());
        assert_eq!(parsed["result"]["acknowledged"], true);
        assert!(shutdown_flag.load(Ordering::SeqCst));
    }
}
