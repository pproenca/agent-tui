use super::*;
use crate::common::error_codes;
use crate::usecases::SnapshotUseCaseImpl;
use agent_tui_usecases::usecases::ports::test_support::MockSession;
use agent_tui_usecases::usecases::ports::test_support::MockSessionRepository;
use serde_json::json;
use std::sync::Arc;

#[test]
fn handle_snapshot_uc_rejects_named_region() {
    let repository = Arc::new(
        MockSessionRepository::builder()
            .with_session_handle(Arc::new(MockSession::new("test-session")))
            .build(),
    );
    let usecase = SnapshotUseCaseImpl::new(repository);
    let request = RpcRequest::new(
        1,
        "snapshot".to_string(),
        Some(json!({ "region": "modal" })),
    );

    let response = handle_snapshot_uc(&usecase, request);
    let response_json = serde_json::to_value(response).expect("response should serialize");

    assert_eq!(response_json["error"]["code"], error_codes::INVALID_INPUT);
    assert_eq!(response_json["error"]["data"]["category"], "invalid_input");
    assert_eq!(response_json["error"]["data"]["context"]["field"], "region");
}
