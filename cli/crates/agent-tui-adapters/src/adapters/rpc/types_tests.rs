use super::*;
use serde::ser::Error as _;

fn make_request(params: Option<Value>) -> RpcRequest {
    RpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: RpcId::from(1),
        method: "test".to_string(),
        params,
    }
}

#[test]
fn test_param_str_extracts_string() {
    let req = make_request(Some(json!({"name": "test-value"})));
    assert_eq!(req.param_str("name"), Some("test-value"));
}

#[test]
fn test_param_str_returns_none_for_missing_key() {
    let req = make_request(Some(json!({"other": "value"})));
    assert_eq!(req.param_str("name"), None);
}

#[test]
fn test_param_bool_opt_extracts_boolean() {
    let req = make_request(Some(json!({"enabled": true, "disabled": false})));
    assert_eq!(req.param_bool_opt("enabled"), Some(true));
    assert_eq!(req.param_bool_opt("disabled"), Some(false));
}

#[test]
fn test_param_bool_with_default() {
    let req = make_request(Some(json!({"enabled": true})));
    assert!(req.param_bool("enabled", false));
    assert!(!req.param_bool("missing", false));
}

#[test]
fn test_param_u64_extracts_number() {
    let req = make_request(Some(json!({"timeout": 5000})));
    assert_eq!(req.param_u64("timeout", 0), 5000);
}

#[test]
fn test_param_u64_returns_default_for_missing() {
    let req = make_request(Some(json!({})));
    assert_eq!(req.param_u64("timeout", 30000), 30000);
}

#[test]
fn test_response_success_format() {
    let resp = RpcResponse::success(42, json!({"data": "test"}));
    let json = serde_json::to_string(&resp).expect("success response should serialize");
    assert!(json.contains("\"jsonrpc\":\"2.0\""));
    assert!(json.contains("\"id\":42"));
    assert!(json.contains("\"result\""));
    assert!(!json.contains("\"error\""));
}

#[test]
fn test_string_request_id_deserializes() {
    let req: RpcRequest =
        serde_json::from_str(r#"{"jsonrpc":"2.0","id":"abc-123","method":"ping"}"#)
            .expect("string request id should deserialize");

    assert_eq!(req.id, RpcId::from("abc-123"));
}

#[test]
fn test_string_response_id_is_echoed() {
    let resp = RpcResponse::success("abc-123", json!({"ok": true}));
    let json = serde_json::to_value(resp).expect("success response should serialize");

    assert_eq!(json["id"], "abc-123");
    assert_eq!(json["result"]["ok"], true);
}

#[test]
fn test_error_without_id_serializes_null_id() {
    let resp = RpcResponse::error_without_id(-32700, "Parse error");
    let json = serde_json::to_value(resp).expect("error response should serialize");

    assert!(json.get("id").is_some());
    assert!(json["id"].is_null());
    assert_eq!(json["error"]["code"], -32700);
}

#[test]
fn test_request_id_from_json_str_recovers_string_and_integer_ids() {
    assert_eq!(
        request_id_from_json_str(r#"{"jsonrpc":"2.0","id":"abc","method":7}"#),
        Some(RpcId::from("abc"))
    );
    assert_eq!(
        request_id_from_json_str(r#"{"jsonrpc":"2.0","id":7,"method":false}"#),
        Some(RpcId::from(7))
    );
    assert_eq!(request_id_from_json_str(r#"{"id":null}"#), None);
    assert_eq!(request_id_from_json_str("not json"), None);
}

#[test]
fn test_response_error_format() {
    let resp = RpcResponse::error(99, -32600, "Invalid Request");
    let json = serde_json::to_string(&resp).expect("error response should serialize");
    assert!(json.contains("\"error\""));
    assert!(json.contains("\"code\":-32600"));
    assert!(!json.contains("\"result\""));
}

#[test]
fn test_action_success_shorthand() {
    let resp = RpcResponse::action_success(1);
    let json = serde_json::to_string(&resp).expect("action success should serialize");
    assert!(json.contains("\"success\":true"));
}

#[test]
fn test_success_json_returns_internal_error_when_serialization_fails() {
    struct FailingSerialize;

    impl Serialize for FailingSerialize {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            Err(S::Error::custom("boom"))
        }
    }

    let resp = RpcResponse::success_json(7, &FailingSerialize);
    let json_str = serde_json::to_string(&resp).expect("internal error response should serialize");
    let parsed: Value =
        serde_json::from_str(&json_str).expect("internal error response should parse");

    assert_eq!(parsed["error"]["code"], -32603);
    assert!(
        parsed["error"]["message"]
            .as_str()
            .expect("error message")
            .contains("failed to serialize response")
    );
}

#[test]
fn test_error_with_data_includes_structured_error() {
    let error_data = ErrorData {
        category: "not_found".to_string(),
        retryable: false,
        context: Some(json!({"session_id": "sess1"})),
        suggestion: Some("Run 'sessions' to see active sessions.".to_string()),
    };
    let resp = RpcResponse::error_with_data(42, -32003, "Resource not found", error_data);
    let json_str = serde_json::to_string(&resp).expect("error-with-data response should serialize");
    let parsed: Value =
        serde_json::from_str(&json_str).expect("error-with-data response should parse");

    assert_eq!(parsed["error"]["code"], -32003);
    assert_eq!(parsed["error"]["data"]["category"], "not_found");
    assert_eq!(parsed["error"]["data"]["retryable"], false);
    assert_eq!(parsed["error"]["data"]["context"]["session_id"], "sess1");
}

#[test]
fn test_domain_error_sets_retryable_for_lock_timeout() {
    let resp = RpcResponse::domain_error(
        1,
        -32007,
        "Lock timeout",
        "busy",
        None,
        Some("Try again".to_string()),
    );
    let json_str = serde_json::to_string(&resp).expect("domain error should serialize");
    let parsed: Value = serde_json::from_str(&json_str).expect("domain error should parse");

    assert_eq!(parsed["error"]["data"]["retryable"], true);
}

#[test]
fn test_domain_error_not_retryable_for_invalid_key() {
    let resp = RpcResponse::domain_error(
        1,
        -32004,
        "Invalid key",
        "invalid_input",
        Some(json!({"key": "BadKey"})),
        None,
    );
    let json_str =
        serde_json::to_string(&resp).expect("non-retryable domain error should serialize");
    let parsed: Value =
        serde_json::from_str(&json_str).expect("non-retryable domain error should parse");

    assert_eq!(parsed["error"]["data"]["retryable"], false);
}
