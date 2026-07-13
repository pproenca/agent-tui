use super::*;
use crate::adapters::rpc::RpcRequest;
use std::io;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

struct FailingShutdownNotifier;

impl crate::usecases::ports::ShutdownNotifier for FailingShutdownNotifier {
    fn notify(&self) -> Result<(), io::Error> {
        Err(io::Error::other("wakeup pipe closed"))
    }
}

#[test]
fn test_handle_shutdown_uc_returns_null_success() {
    let shutdown_flag = AtomicBool::new(false);
    let notifier = crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier;

    let request = RpcRequest::new(1, "shutdown".to_string(), None);
    let response = handle_shutdown_uc(&shutdown_flag, &notifier, request);

    let json_str = serde_json::to_string(&response).expect("shutdown response should serialize");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("shutdown response should parse");

    assert!(parsed.get("error").is_none() || parsed["error"].is_null());
    assert!(parsed["result"].is_null());
    assert!(shutdown_flag.load(Ordering::SeqCst));
}

#[test]
fn test_handle_shutdown_uc_returns_error_when_notification_fails() {
    let shutdown_flag = AtomicBool::new(false);
    let request = RpcRequest::new(1, "shutdown".to_string(), None);

    let response = handle_shutdown_uc(&shutdown_flag, &FailingShutdownNotifier, request);
    let parsed = serde_json::to_value(response).expect("shutdown response should serialize");

    assert_eq!(parsed["error"]["code"], -32603);
    assert!(
        parsed["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("wakeup pipe closed"))
    );
    assert!(shutdown_flag.load(Ordering::SeqCst));
}
