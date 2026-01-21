use super::error_messages::ai_friendly_error;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct Request {
    #[allow(dead_code)]
    jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

impl Request {
    pub fn param_str(&self, key: &str) -> Option<&str> {
        self.params
            .as_ref()
            .and_then(|p| p.get(key))
            .and_then(|v| v.as_str())
    }

    pub fn param_bool(&self, key: &str) -> Option<bool> {
        self.params.as_ref()?.get(key)?.as_bool()
    }

    pub fn param_array(&self, key: &str) -> Option<&Vec<Value>> {
        self.params.as_ref()?.get(key)?.as_array()
    }

    pub fn param_u64(&self, key: &str, default: u64) -> u64 {
        self.params
            .as_ref()
            .and_then(|p| p.get(key))
            .and_then(|v| v.as_u64())
            .unwrap_or(default)
    }

    pub fn param_u16(&self, key: &str, default: u16) -> u16 {
        self.param_u64(key, default as u64) as u16
    }

    pub fn param_i32(&self, key: &str, default: i32) -> i32 {
        self.params
            .as_ref()
            .and_then(|p| p.get(key))
            .and_then(|v| v.as_i64())
            .map(|n| n as i32)
            .unwrap_or(default)
    }

    #[allow(clippy::result_large_err)]
    pub fn require_str(&self, key: &str) -> Result<&str, Response> {
        self.param_str(key)
            .ok_or_else(|| Response::error(self.id, -32602, &format!("Missing '{}' param", key)))
    }

    #[allow(clippy::result_large_err)]
    pub fn require_array(&self, key: &str) -> Result<&Vec<Value>, Response> {
        self.param_array(key)
            .ok_or_else(|| Response::error(self.id, -32602, &format!("Missing '{}' param", key)))
    }
}

#[derive(Debug, Serialize)]
pub struct Response {
    jsonrpc: String,
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
pub struct RpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl Response {
    pub fn success(id: u64, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: u64, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(RpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }

    /// Shorthand for a successful action response with just `{ "success": true }`.
    pub fn action_success(id: u64) -> Self {
        Self::success(id, json!({ "success": true }))
    }

    /// Shorthand for a failed action response with AI-friendly error message.
    pub fn action_failed(id: u64, element_ref: Option<&str>, error: &str) -> Self {
        Self::success(
            id,
            json!({
                "success": false,
                "message": ai_friendly_error(error, element_ref)
            }),
        )
    }

    /// Shorthand for element not found response.
    pub fn element_not_found(id: u64, element_ref: &str) -> Self {
        Self::success(
            id,
            json!({
                "success": false,
                "message": ai_friendly_error("Element not found", Some(element_ref))
            }),
        )
    }

    /// Shorthand for wrong element type response.
    pub fn wrong_element_type(id: u64, element_ref: &str, actual: &str, expected: &str) -> Self {
        Self::success(
            id,
            json!({
                "success": false,
                "message": format!(
                    "Element {} is a {} not a {}. Run 'snapshot -i' to see element types.",
                    element_ref, actual, expected
                )
            }),
        )
    }
}
