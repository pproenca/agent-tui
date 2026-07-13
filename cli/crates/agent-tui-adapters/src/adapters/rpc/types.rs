//! RPC request and response types.

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;

use crate::common::RpcId;
use crate::common::error_codes;

pub fn request_id_from_json_str(input: &str) -> Option<RpcId> {
    let value: Value = serde_json::from_str(input).ok()?;
    request_id_from_json_value(&value)
}

pub fn request_id_from_json_value(value: &Value) -> Option<RpcId> {
    value
        .get("id")
        .cloned()
        .and_then(|id| serde_json::from_value(id).ok())
}

#[derive(Debug, Deserialize)]
pub struct RpcRequest {
    #[serde(rename = "jsonrpc")]
    _jsonrpc: String,
    pub id: RpcId,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

impl RpcRequest {
    pub fn new(id: impl Into<RpcId>, method: String, params: Option<Value>) -> Self {
        Self {
            _jsonrpc: "2.0".to_string(),
            id: id.into(),
            method,
            params,
        }
    }

    pub fn param_str(&self, key: &str) -> Option<&str> {
        self.params
            .as_ref()
            .and_then(|p| p.get(key))
            .and_then(|v| v.as_str())
    }

    pub fn param_bool_opt(&self, key: &str) -> Option<bool> {
        self.params.as_ref()?.get(key)?.as_bool()
    }

    pub fn param_bool(&self, key: &str, default: bool) -> bool {
        self.param_bool_opt(key).unwrap_or(default)
    }

    pub fn param_u64_opt(&self, key: &str) -> Option<u64> {
        self.params
            .as_ref()
            .and_then(|p| p.get(key))
            .and_then(serde_json::Value::as_u64)
    }

    pub fn param_u64(&self, key: &str, default: u64) -> u64 {
        self.param_u64_opt(key).unwrap_or(default)
    }

    pub fn param_u16(&self, key: &str, default: u16) -> u16 {
        self.param_u64(key, default as u64) as u16
    }

    #[allow(clippy::result_large_err)]
    pub fn require_str(&self, key: &str) -> Result<&str, RpcResponse> {
        self.param_str(key).ok_or_else(|| {
            RpcResponse::error(self.id.clone(), -32602, &format!("Missing '{key}' param"))
        })
    }
}

#[derive(Debug, Serialize)]
pub struct RpcResponse {
    #[serde(rename = "jsonrpc")]
    _jsonrpc: String,
    id: Option<RpcId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcServerError>,
}

#[derive(Debug, Serialize)]
pub struct RpcServerError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct ErrorData {
    pub category: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl RpcResponse {
    pub fn success(id: impl Into<RpcId>, result: Value) -> Self {
        Self {
            _jsonrpc: "2.0".to_string(),
            id: Some(id.into()),
            result: Some(result),
            error: None,
        }
    }

    pub fn success_json<T: Serialize>(id: impl Into<RpcId>, result: &T) -> Self {
        let id = id.into();
        match serde_json::to_value(result) {
            Ok(value) => Self::success(id, value),
            Err(err) => Self::internal_error(
                id,
                &format!("Internal error: failed to serialize response: {err}"),
            ),
        }
    }

    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    pub fn error(id: impl Into<RpcId>, code: i32, message: &str) -> Self {
        Self {
            _jsonrpc: "2.0".to_string(),
            id: Some(id.into()),
            result: None,
            error: Some(RpcServerError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }

    pub fn error_without_id(code: i32, message: &str) -> Self {
        Self {
            _jsonrpc: "2.0".to_string(),
            id: None,
            result: None,
            error: Some(RpcServerError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }

    fn internal_error(id: impl Into<RpcId>, message: &str) -> Self {
        Self::error(id, -32603, message)
    }

    pub fn error_with_context(
        id: impl Into<RpcId>,
        code: i32,
        message: &str,
        session_id: Option<&str>,
    ) -> Self {
        let data = session_id.map(|sid| json!({ "session_id": sid }));
        Self {
            _jsonrpc: "2.0".to_string(),
            id: Some(id.into()),
            result: None,
            error: Some(RpcServerError {
                code,
                message: message.to_string(),
                data,
            }),
        }
    }

    pub fn error_with_data(
        id: impl Into<RpcId>,
        code: i32,
        message: &str,
        error_data: ErrorData,
    ) -> Self {
        let id = id.into();
        match serde_json::to_value(error_data) {
            Ok(data) => Self {
                _jsonrpc: "2.0".to_string(),
                id: Some(id),
                result: None,
                error: Some(RpcServerError {
                    code,
                    message: message.to_string(),
                    data: Some(data),
                }),
            },
            Err(err) => Self::internal_error(
                id,
                &format!("Internal error: failed to serialize error payload: {err}"),
            ),
        }
    }

    pub fn domain_error(
        id: impl Into<RpcId>,
        code: i32,
        message: &str,
        category: &str,
        context: Option<Value>,
        suggestion: Option<String>,
    ) -> Self {
        Self::error_with_data(
            id,
            code,
            message,
            ErrorData {
                category: category.to_string(),
                retryable: error_codes::is_retryable(code),
                context,
                suggestion,
            },
        )
    }
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
