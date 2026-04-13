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

    let json_str = serde_json::to_string(&response).expect("shutdown response should serialize");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("shutdown response should parse");

    assert!(parsed.get("error").is_none() || parsed["error"].is_null());
    assert_eq!(parsed["result"]["acknowledged"], true);
    assert!(shutdown_flag.load(Ordering::SeqCst));
}
