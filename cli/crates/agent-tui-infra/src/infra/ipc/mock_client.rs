//! Mock IPC client for tests.
#![allow(clippy::expect_used)]

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use serde_json::Value;

use crate::common::mutex_lock_or_recover;
use crate::infra::ipc::client::DaemonClient;
use crate::infra::ipc::client::DaemonClientConfig;
use crate::infra::ipc::error::ClientError;

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
        mutex_lock_or_recover(&self.responses).insert(method.to_string(), response);
    }

    pub fn set_default_response(&mut self, response: Value) {
        self.default_response = response;
    }

    pub fn get_calls(&self) -> Vec<(String, Option<Value>)> {
        mutex_lock_or_recover(&self.calls).clone()
    }

    pub fn call_count(&self, method: &str) -> usize {
        mutex_lock_or_recover(&self.calls)
            .iter()
            .filter(|(m, _)| m == method)
            .count()
    }

    pub fn last_call(&self, method: &str) -> Option<(String, Option<Value>)> {
        mutex_lock_or_recover(&self.calls)
            .iter()
            .rev()
            .find(|(m, _)| m == method)
            .cloned()
    }

    pub fn params_for(&self, method: &str) -> Vec<Option<Value>> {
        mutex_lock_or_recover(&self.calls)
            .iter()
            .filter(|(m, _)| m == method)
            .map(|(_, p)| p.clone())
            .collect()
    }

    pub fn clear_calls(&mut self) {
        mutex_lock_or_recover(&self.calls).clear();
    }

    pub fn clear_responses(&mut self) {
        mutex_lock_or_recover(&self.responses).clear();
    }

    pub fn reset(&mut self) {
        self.clear_calls();
        self.clear_responses();
    }
}

impl DaemonClient for MockClient {
    fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value, ClientError> {
        mutex_lock_or_recover(&self.calls).push((method.to_string(), params));

        let responses = mutex_lock_or_recover(&self.responses);
        if let Some(response) = responses.get(method) {
            Ok(response.clone())
        } else if self.error_on_missing {
            Err(ClientError::RpcError {
                code: -32601,
                message: format!("Method not found: {method}"),
                category: None,
                retryable: false,
                retry_delay_ms: None,
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
}

#[cfg(test)]
#[path = "mock_client_tests.rs"]
mod tests;
