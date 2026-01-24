use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use serde_json::Value;

use crate::ipc::client::{DaemonClient, DaemonClientConfig};
use crate::ipc::error::ClientError;

type CallRecord = Vec<(String, Option<Value>)>;

#[derive(Clone)]
pub struct MockClient {
    responses: Arc<Mutex<HashMap<String, Value>>>,
    calls: Arc<Mutex<CallRecord>>,
    default_response: Value,
    error_on_missing: bool,
}

impl Default for MockClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockClient {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
            calls: Arc::new(Mutex::new(Vec::new())),
            default_response: serde_json::json!({ "success": true }),
            error_on_missing: false,
        }
    }

    pub fn new_strict() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
            calls: Arc::new(Mutex::new(Vec::new())),
            default_response: serde_json::json!(null),
            error_on_missing: true,
        }
    }

    pub fn set_response(&mut self, method: &str, response: Value) {
        self.responses
            .lock()
            .unwrap()
            .insert(method.to_string(), response);
    }

    pub fn set_default_response(&mut self, response: Value) {
        self.default_response = response;
    }

    pub fn get_calls(&self) -> Vec<(String, Option<Value>)> {
        self.calls.lock().unwrap().clone()
    }

    pub fn call_count(&self, method: &str) -> usize {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(m, _)| m == method)
            .count()
    }

    pub fn last_call(&self, method: &str) -> Option<(String, Option<Value>)> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|(m, _)| m == method)
            .cloned()
    }

    pub fn params_for(&self, method: &str) -> Vec<Option<Value>> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(m, _)| m == method)
            .map(|(_, p)| p.clone())
            .collect()
    }

    pub fn clear_calls(&mut self) {
        self.calls.lock().unwrap().clear();
    }

    pub fn clear_responses(&mut self) {
        self.responses.lock().unwrap().clear();
    }

    pub fn reset(&mut self) {
        self.clear_calls();
        self.clear_responses();
    }
}

impl DaemonClient for MockClient {
    fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value, ClientError> {
        self.calls
            .lock()
            .unwrap()
            .push((method.to_string(), params.clone()));

        let responses = self.responses.lock().unwrap();
        if let Some(response) = responses.get(method) {
            Ok(response.clone())
        } else if self.error_on_missing {
            Err(ClientError::RpcError {
                code: -32601,
                message: format!("Method not found: {}", method),
                category: None,
                retryable: false,
                context: None,
                suggestion: None,
            })
        } else {
            Ok(self.default_response.clone())
        }
    }

    fn call_with_config(
        &mut self,
        method: &str,
        params: Option<Value>,
        _config: &DaemonClientConfig,
    ) -> Result<Value, ClientError> {
        self.call(method, params)
    }

    fn call_with_retry(
        &mut self,
        method: &str,
        params: Option<Value>,
        _max_retries: u32,
    ) -> Result<Value, ClientError> {
        self.call(method, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mock_client_returns_configured_response() {
        let mut mock = MockClient::new();
        mock.set_response("health", json!({ "status": "ok" }));

        let result = mock.call("health", None).unwrap();
        assert_eq!(result, json!({ "status": "ok" }));
    }

    #[test]
    fn test_mock_client_returns_default_for_unconfigured() {
        let mut mock = MockClient::new();

        let result = mock.call("unknown", None).unwrap();
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
            .unwrap();
        mock.call("method2", None).unwrap();
        mock.call("method1", Some(json!({ "key2": "value2" })))
            .unwrap();

        assert_eq!(mock.call_count("method1"), 2);
        assert_eq!(mock.call_count("method2"), 1);
        assert_eq!(mock.get_calls().len(), 3);
    }

    #[test]
    fn test_mock_client_last_call() {
        let mut mock = MockClient::new();

        mock.call("test", Some(json!({ "attempt": 1 }))).unwrap();
        mock.call("test", Some(json!({ "attempt": 2 }))).unwrap();

        let last = mock.last_call("test").unwrap();
        assert_eq!(last.1, Some(json!({ "attempt": 2 })));
    }

    #[test]
    fn test_mock_client_params_for() {
        let mut mock = MockClient::new();

        mock.call("test", Some(json!({ "a": 1 }))).unwrap();
        mock.call("other", Some(json!({ "b": 2 }))).unwrap();
        mock.call("test", Some(json!({ "c": 3 }))).unwrap();

        let params = mock.params_for("test");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], Some(json!({ "a": 1 })));
        assert_eq!(params[1], Some(json!({ "c": 3 })));
    }

    #[test]
    fn test_mock_client_reset() {
        let mut mock = MockClient::new();
        mock.set_response("test", json!({ "data": "value" }));
        mock.call("test", None).unwrap();

        mock.reset();

        assert_eq!(mock.call_count("test"), 0);
        let result = mock.call("test", None).unwrap();
        assert_eq!(result, json!({ "success": true }));
    }

    #[test]
    fn test_mock_client_custom_default() {
        let mut mock = MockClient::new();
        mock.set_default_response(json!({ "custom": "default" }));

        let result = mock.call("any_method", None).unwrap();
        assert_eq!(result, json!({ "custom": "default" }));
    }
}
