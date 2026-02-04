use serde_json::Value;
use thiserror::Error;

use crate::common::error_codes::ErrorCategory;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Failed to connect to daemon: {0}")]
    ConnectionFailed(#[from] std::io::Error),

    #[error("Failed to serialize request: {0}")]
    SerializationFailed(#[from] serde_json::Error),

    #[error("RPC error ({code}): {message}")]
    RpcError {
        code: i32,
        message: String,
        category: Option<ErrorCategory>,
        retryable: bool,
        context: Option<Value>,
        suggestion: Option<String>,
    },

    #[error("Daemon not running")]
    DaemonNotRunning,

    #[error("Invalid response from daemon")]
    InvalidResponse,

    #[error("Failed to send signal to process {pid}: {message}")]
    SignalFailed {
        pid: u32,
        message: String,
        #[source]
        source: Option<std::io::Error>,
    },

    #[error("Unexpected response: {message}")]
    UnexpectedResponse { message: String },
}

impl ClientError {
    pub fn is_retryable(&self) -> bool {
        match self {
            ClientError::RpcError { retryable, .. } => *retryable,
            _ => false,
        }
    }

    pub fn category(&self) -> Option<ErrorCategory> {
        match self {
            ClientError::RpcError { category, .. } => *category,
            _ => None,
        }
    }

    pub fn suggestion(&self) -> Option<&str> {
        match self {
            ClientError::RpcError { suggestion, .. } => suggestion.as_deref(),
            ClientError::DaemonNotRunning => Some("Start daemon with: agent-tui daemon"),
            _ => None,
        }
    }

    pub fn context(&self) -> Option<&Value> {
        match self {
            ClientError::RpcError { context, .. } => context.as_ref(),
            _ => None,
        }
    }

    pub fn to_json(&self) -> Value {
        match self {
            ClientError::RpcError {
                code,
                message,
                category,
                retryable,
                context,
                suggestion,
            } => {
                let mut obj = serde_json::json!({
                    "code": code,
                    "message": message,
                    "retryable": retryable,
                });
                if let Some(cat) = category {
                    obj["category"] = serde_json::json!(cat.as_str());
                }
                if let Some(ctx) = context {
                    obj["context"] = ctx.clone();
                }
                if let Some(sug) = suggestion {
                    obj["suggestion"] = serde_json::json!(sug);
                }
                obj
            }
            ClientError::ConnectionFailed(e) => serde_json::json!({
                "code": -32000,
                "message": format!("Connection failed: {}", e),
                "category": "external",
                "retryable": true,
            }),
            ClientError::DaemonNotRunning => serde_json::json!({
                "code": -32000,
                "message": "Daemon not running",
                "category": "external",
                "retryable": false,
                "suggestion": "Start daemon with: agent-tui daemon",
            }),
            ClientError::InvalidResponse => serde_json::json!({
                "code": -32000,
                "message": "Invalid response from daemon",
                "category": "internal",
                "retryable": false,
            }),
            ClientError::SerializationFailed(e) => serde_json::json!({
                "code": -32000,
                "message": format!("Serialization failed: {}", e),
                "category": "internal",
                "retryable": false,
            }),
            ClientError::SignalFailed { pid, message, .. } => serde_json::json!({
                "code": -32000,
                "message": format!("Failed to send signal to process {}: {}", pid, message),
                "category": "external",
                "retryable": false,
                "exit_code": 74,
            }),
            ClientError::UnexpectedResponse { message } => serde_json::json!({
                "code": -32000,
                "message": format!("Unexpected response: {}", message),
                "category": "internal",
                "retryable": false,
            }),
        }
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string_pretty(&self.to_json()).unwrap_or_default()
    }
}
