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

    fn make_request(params: Option<Value>) -> Request {
        Request {
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
    fn test_param_str_returns_none_for_non_string() {
        let req = make_request(Some(json!({"count": 42})));
        assert_eq!(req.param_str("count"), None);
    }

    #[test]
    fn test_param_str_returns_none_for_null_params() {
        let req = make_request(None);
        assert_eq!(req.param_str("name"), None);
    }

    #[test]
    fn test_param_bool_extracts_boolean() {
        let req = make_request(Some(json!({"enabled": true, "disabled": false})));
        assert_eq!(req.param_bool("enabled"), Some(true));
        assert_eq!(req.param_bool("disabled"), Some(false));
    }

    #[test]
    fn test_param_bool_returns_none_for_missing_key() {
        let req = make_request(Some(json!({"other": "value"})));
        assert_eq!(req.param_bool("enabled"), None);
    }

    #[test]
    fn test_param_array_extracts_array() {
        let req = make_request(Some(json!({"items": ["a", "b", "c"]})));
        let arr = req.param_array("items").unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], "a");
    }

    #[test]
    fn test_param_array_returns_none_for_non_array() {
        let req = make_request(Some(json!({"items": "not-an-array"})));
        assert_eq!(req.param_array("items"), None);
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
    fn test_param_u16_extracts_and_converts() {
        let req = make_request(Some(json!({"cols": 80})));
        assert_eq!(req.param_u16("cols", 120), 80);
    }

    #[test]
    fn test_param_i32_extracts_signed() {
        let req = make_request(Some(json!({"offset": -10})));
        assert_eq!(req.param_i32("offset", 0), -10);
    }

    #[test]
    fn test_require_str_returns_value_when_present() {
        let req = make_request(Some(json!({"ref": "@btn1"})));
        assert_eq!(req.require_str("ref").unwrap(), "@btn1");
    }

    #[test]
    fn test_require_str_returns_error_when_missing() {
        let req = make_request(Some(json!({})));
        let err = req.require_str("ref").unwrap_err();
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("Missing 'ref' param"));
        assert!(json.contains("-32602"));
    }

    #[test]
    fn test_require_array_returns_value_when_present() {
        let req = make_request(Some(json!({"options": ["a", "b"]})));
        let arr = req.require_array("options").unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn test_require_array_returns_error_when_missing() {
        let req = make_request(Some(json!({})));
        let err = req.require_array("options").unwrap_err();
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("Missing 'options' param"));
    }

    #[test]
    fn test_response_success_format() {
        let resp = Response::success(42, json!({"data": "test"}));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":42"));
        assert!(json.contains("\"result\""));
        assert!(json.contains("\"data\":\"test\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_response_error_format() {
        let resp = Response::error(99, -32600, "Invalid Request");
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":99"));
        assert!(json.contains("\"error\""));
        assert!(json.contains("\"code\":-32600"));
        assert!(json.contains("\"message\":\"Invalid Request\""));
        assert!(!json.contains("\"result\""));
    }

    #[test]
    fn test_action_success_shorthand() {
        let resp = Response::action_success(1);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn test_action_failed_includes_message() {
        let resp = Response::action_failed(1, Some("@btn1"), "Click timeout");
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("message"));
    }

    #[test]
    fn test_element_not_found_response() {
        let resp = Response::element_not_found(1, "@missing");
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("@missing"));
    }

    #[test]
    fn test_wrong_element_type_response() {
        let resp = Response::wrong_element_type(1, "@btn1", "button", "input");
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("button"));
        assert!(json.contains("input"));
    }

    #[test]
    fn test_suggest_command_for_button() {
        let suggestion = suggest_command_for_type("button", "@btn1");
        assert_eq!(suggestion, "Try: click @btn1");
    }

    #[test]
    fn test_suggest_command_for_menuitem() {
        let suggestion = suggest_command_for_type("menuitem", "@menu1");
        assert_eq!(suggestion, "Try: click @menu1");
    }

    #[test]
    fn test_suggest_command_for_listitem() {
        let suggestion = suggest_command_for_type("listitem", "@item1");
        assert_eq!(suggestion, "Try: click @item1");
    }

    #[test]
    fn test_suggest_command_for_checkbox() {
        let suggestion = suggest_command_for_type("checkbox", "@cb1");
        assert_eq!(suggestion, "Try: toggle @cb1 or click @cb1");
    }

    #[test]
    fn test_suggest_command_for_radio() {
        let suggestion = suggest_command_for_type("radio", "@rad1");
        assert_eq!(suggestion, "Try: toggle @rad1 or click @rad1");
    }

    #[test]
    fn test_suggest_command_for_input() {
        let suggestion = suggest_command_for_type("input", "@inp1");
        assert_eq!(suggestion, "Try: fill @inp1 <value>");
    }

    #[test]
    fn test_suggest_command_for_select() {
        let suggestion = suggest_command_for_type("select", "@sel1");
        assert_eq!(suggestion, "Try: select @sel1 <option>");
    }

    #[test]
    fn test_suggest_command_for_unknown_type() {
        let suggestion = suggest_command_for_type("unknown", "@el1");
        assert_eq!(suggestion, "");
    }

    #[test]
    fn test_suggest_command_for_static_text() {
        let suggestion = suggest_command_for_type("static_text", "@txt1");
        assert_eq!(suggestion, "");
    }
}
