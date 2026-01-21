use serde::{Deserialize, Serialize};
use serde_json::Value;

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
}
