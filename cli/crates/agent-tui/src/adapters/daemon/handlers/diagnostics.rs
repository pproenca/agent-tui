use crate::adapters::rpc::RpcRequest;
use crate::adapters::rpc::RpcResponse;

use super::common;
use super::common::session_error_response;
use crate::adapters::health_output_to_response;
use crate::adapters::metrics_output_to_response;
use crate::adapters::parse_terminal_read_input;
use crate::adapters::parse_terminal_write_input;
use crate::adapters::shutdown_output_to_response;
use crate::adapters::terminal_read_output_to_response;
use crate::adapters::terminal_write_output_to_response;
use crate::domain::HealthInput;
use crate::domain::MetricsInput;
use crate::domain::ShutdownInput;
use crate::usecases::HealthUseCase;
use crate::usecases::MetricsUseCase;
use crate::usecases::ShutdownUseCase;
use crate::usecases::TerminalReadUseCase;
use crate::usecases::TerminalWriteUseCase;

pub fn handle_health_uc<U: HealthUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "health").entered();
    let req_id = request.id;
    let input = HealthInput;

    match usecase.execute(input) {
        Ok(output) => health_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_metrics_uc<U: MetricsUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "metrics").entered();
    let req_id = request.id;
    let input = MetricsInput;

    match usecase.execute(input) {
        Ok(output) => metrics_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_terminal_read_uc<U: TerminalReadUseCase>(
    usecase: &U,
    request: RpcRequest,
) -> RpcResponse {
    let _span = common::handler_span(&request, "pty_read").entered();
    let req_id = request.id;
    let input = parse_terminal_read_input(&request);

    match usecase.execute(input) {
        Ok(output) => terminal_read_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_terminal_write_uc<U: TerminalWriteUseCase>(
    usecase: &U,
    request: RpcRequest,
) -> RpcResponse {
    let _span = common::handler_span(&request, "pty_write").entered();
    let req_id = request.id;
    let input = match parse_terminal_write_input(&request) {
        Ok(i) => i,
        Err(resp) => return resp,
    };

    match usecase.execute(input) {
        Ok(output) => terminal_write_output_to_response(req_id, output),
        Err(e) => session_error_response(req_id, e),
    }
}

pub fn handle_shutdown_uc<U: ShutdownUseCase>(usecase: &U, request: RpcRequest) -> RpcResponse {
    let _span = common::handler_span(&request, "shutdown").entered();
    let req_id = request.id;
    let output = usecase.execute(ShutdownInput);
    shutdown_output_to_response(req_id, output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::rpc::RpcRequest;
    use crate::domain::ShutdownOutput;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;

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
