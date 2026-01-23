use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;

use crate::error_codes;

#[derive(Debug, Deserialize)]
pub struct RpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

impl RpcRequest {
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
    pub fn require_str(&self, key: &str) -> Result<&str, RpcResponse> {
        self.param_str(key)
            .ok_or_else(|| RpcResponse::error(self.id, -32602, &format!("Missing '{}' param", key)))
    }

    #[allow(clippy::result_large_err)]
    pub fn require_array(&self, key: &str) -> Result<&Vec<Value>, RpcResponse> {
        self.param_array(key)
            .ok_or_else(|| RpcResponse::error(self.id, -32602, &format!("Missing '{}' param", key)))
    }
}

#[derive(Debug, Serialize)]
pub struct RpcResponse {
    jsonrpc: String,
    id: u64,
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

/// Structured error data for programmatic handling by AI agents.
///
/// This provides rich context about errors including:
/// - Category for routing error handling logic
/// - Retryable flag for automatic retry decisions
/// - Context with error-specific details
/// - Suggestion for how to resolve the error
#[derive(Debug, Serialize)]
pub struct ErrorData {
    /// Error category (not_found, invalid_input, busy, internal, external, timeout)
    pub category: String,
    /// Whether this error might succeed on retry
    pub retryable: bool,
    /// Error-specific context (element_ref, session_id, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Value>,
    /// Human-readable suggestion for resolving the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl RpcResponse {
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
            error: Some(RpcServerError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }

    pub fn error_with_context(id: u64, code: i32, message: &str, session_id: Option<&str>) -> Self {
        let data = session_id.map(|sid| json!({ "session_id": sid }));
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(RpcServerError {
                code,
                message: message.to_string(),
                data,
            }),
        }
    }

    /// Create an error response with structured ErrorData.
    ///
    /// This is the preferred method for domain errors as it provides
    /// machine-readable context for AI agents.
    pub fn error_with_data(id: u64, code: i32, message: &str, error_data: ErrorData) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(RpcServerError {
                code,
                message: message.to_string(),
                data: Some(serde_json::to_value(error_data).unwrap_or(json!({}))),
            }),
        }
    }

    /// Create an error response from a DomainError-like interface.
    ///
    /// This helper constructs a fully structured error response with:
    /// - Semantic error code
    /// - Human-readable message
    /// - Category, retryable flag, context, and suggestion
    pub fn domain_error(
        id: u64,
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

    pub fn action_success(id: u64) -> Self {
        Self::success(id, json!({ "success": true }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(params: Option<Value>) -> RpcRequest {
        RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
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
    fn test_param_bool_extracts_boolean() {
        let req = make_request(Some(json!({"enabled": true, "disabled": false})));
        assert_eq!(req.param_bool("enabled"), Some(true));
        assert_eq!(req.param_bool("disabled"), Some(false));
    }

    #[test]
    fn test_param_array_extracts_array() {
        let req = make_request(Some(json!({"items": ["a", "b", "c"]})));
        let arr = req.param_array("items").unwrap();
        assert_eq!(arr.len(), 3);
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
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":42"));
        assert!(json.contains("\"result\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_response_error_format() {
        let resp = RpcResponse::error(99, -32600, "Invalid Request");
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"error\""));
        assert!(json.contains("\"code\":-32600"));
        assert!(!json.contains("\"result\""));
    }

    #[test]
    fn test_action_success_shorthand() {
        let resp = RpcResponse::action_success(1);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn test_error_with_data_includes_structured_error() {
        let error_data = ErrorData {
            category: "not_found".to_string(),
            retryable: false,
            context: Some(json!({"element_ref": "@btn1"})),
            suggestion: Some("Run 'snapshot -i' to see elements.".to_string()),
        };
        let resp = RpcResponse::error_with_data(42, -32003, "Element not found", error_data);
        let json_str = serde_json::to_string(&resp).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["error"]["code"], -32003);
        assert_eq!(parsed["error"]["data"]["category"], "not_found");
        assert_eq!(parsed["error"]["data"]["retryable"], false);
        assert_eq!(parsed["error"]["data"]["context"]["element_ref"], "@btn1");
    }

    #[test]
    fn test_domain_error_sets_retryable_for_lock_timeout() {
        let resp = RpcResponse::domain_error(
            1,
            -32007, // LOCK_TIMEOUT
            "Lock timeout",
            "busy",
            None,
            Some("Try again".to_string()),
        );
        let json_str = serde_json::to_string(&resp).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["error"]["data"]["retryable"], true);
    }

    #[test]
    fn test_domain_error_not_retryable_for_element_not_found() {
        let resp = RpcResponse::domain_error(
            1,
            -32003, // ELEMENT_NOT_FOUND
            "Element not found",
            "not_found",
            Some(json!({"element_ref": "@btn1"})),
            None,
        );
        let json_str = serde_json::to_string(&resp).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["error"]["data"]["retryable"], false);
    }
}
