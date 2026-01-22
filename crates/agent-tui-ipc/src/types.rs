use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;

use crate::error_messages::ai_friendly_error;

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

    pub fn action_success(id: u64) -> Self {
        Self::success(id, json!({ "success": true }))
    }

    pub fn action_failed(id: u64, element_ref: Option<&str>, error: &str) -> Self {
        Self::success(
            id,
            json!({
                "success": false,
                "message": ai_friendly_error(error, element_ref)
            }),
        )
    }

    pub fn element_not_found(id: u64, element_ref: &str) -> Self {
        Self::success(
            id,
            json!({
                "success": false,
                "message": ai_friendly_error("Element not found", Some(element_ref))
            }),
        )
    }

    pub fn wrong_element_type(id: u64, element_ref: &str, actual: &str, expected: &str) -> Self {
        let suggestion = suggest_command_for_type(actual, element_ref);
        let hint = if suggestion.is_empty() {
            "Run 'snapshot -i' to see element types.".to_string()
        } else {
            suggestion
        };
        Self::success(
            id,
            json!({
                "success": false,
                "message": format!(
                    "Element {} is a {} not a {}. {}",
                    element_ref, actual, expected, hint
                )
            }),
        )
    }
}

fn suggest_command_for_type(element_type: &str, element_ref: &str) -> String {
    match element_type {
        "button" | "menuitem" | "listitem" => format!("Try: click {}", element_ref),
        "checkbox" | "radio" => format!("Try: toggle {} or click {}", element_ref, element_ref),
        "input" => format!("Try: fill {} <value>", element_ref),
        "select" => format!("Try: select {} <option>", element_ref),
        _ => String::new(),
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
    fn test_suggest_command_for_button() {
        let suggestion = suggest_command_for_type("button", "@btn1");
        assert_eq!(suggestion, "Try: click @btn1");
    }

    #[test]
    fn test_suggest_command_for_input() {
        let suggestion = suggest_command_for_type("input", "@inp1");
        assert_eq!(suggestion, "Try: fill @inp1 <value>");
    }

    #[test]
    fn test_suggest_command_for_unknown_type() {
        let suggestion = suggest_command_for_type("unknown", "@el1");
        assert_eq!(suggestion, "");
    }
}
