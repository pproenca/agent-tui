use super::*;
use serde_json::json;

#[test]
fn test_mock_client_returns_configured_response() {
    let mut mock = MockClient::new();
    mock.set_response("version", json!({ "status": "ok" }));

    let result = mock
        .call("version", None)
        .expect("configured response should be returned");
    assert_eq!(result, json!({ "status": "ok" }));
}

#[test]
fn test_mock_client_returns_default_for_unconfigured() {
    let mut mock = MockClient::new();

    let result = mock
        .call("unknown", None)
        .expect("default response should be returned");
    assert_eq!(result, json!({ "success": true }));
}

#[test]
fn test_mock_client_strict_errors_on_unknown() {
    let mut mock = MockClient::new_strict();

    let result = mock.call("unknown", None);
    assert!(result.is_err());
}

#[test]
fn test_mock_client_tracks_calls() {
    let mut mock = MockClient::new();

    mock.call("method1", Some(json!({ "key": "value" })))
        .expect("first call should succeed");
    mock.call("method2", None)
        .expect("second call should succeed");
    mock.call("method1", Some(json!({ "key2": "value2" })))
        .expect("third call should succeed");

    assert_eq!(mock.call_count("method1"), 2);
    assert_eq!(mock.call_count("method2"), 1);
    assert_eq!(mock.get_calls().len(), 3);
}

#[test]
fn test_mock_client_last_call() {
    let mut mock = MockClient::new();

    mock.call("test", Some(json!({ "attempt": 1 })))
        .expect("first call should succeed");
    mock.call("test", Some(json!({ "attempt": 2 })))
        .expect("second call should succeed");

    let last = mock
        .last_call("test")
        .expect("last call should be recorded");
    assert_eq!(last.1, Some(json!({ "attempt": 2 })));
}

#[test]
fn test_mock_client_params_for() {
    let mut mock = MockClient::new();

    mock.call("test", Some(json!({ "a": 1 })))
        .expect("first call should succeed");
    mock.call("other", Some(json!({ "b": 2 })))
        .expect("second call should succeed");
    mock.call("test", Some(json!({ "c": 3 })))
        .expect("third call should succeed");

    let params = mock.params_for("test");
    assert_eq!(params.len(), 2);
    assert_eq!(params[0], Some(json!({ "a": 1 })));
    assert_eq!(params[1], Some(json!({ "c": 3 })));
}

#[test]
fn test_mock_client_reset() {
    let mut mock = MockClient::new();
    mock.set_response("test", json!({ "data": "value" }));
    mock.call("test", None).expect("call should succeed");

    mock.reset();

    assert_eq!(mock.call_count("test"), 0);
    let result = mock
        .call("test", None)
        .expect("default response should be returned after reset");
    assert_eq!(result, json!({ "success": true }));
}

#[test]
fn test_mock_client_custom_default() {
    let mut mock = MockClient::new();
    mock.set_default_response(json!({ "custom": "default" }));

    let result = mock
        .call("any_method", None)
        .expect("custom default response should be returned");
    assert_eq!(result, json!({ "custom": "default" }));
}
