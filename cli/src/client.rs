use crate::daemon::socket_path;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use thiserror::Error;

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Failed to connect to daemon: {0}")]
    ConnectionFailed(#[from] std::io::Error),

    #[error("Failed to serialize request: {0}")]
    SerializationFailed(#[from] serde_json::Error),

    #[error("RPC error ({code}): {message}")]
    RpcError { code: i32, message: String },

    #[error("Daemon not running")]
    DaemonNotRunning,

    #[error("Invalid response from daemon")]
    InvalidResponse,
}

#[derive(Debug, Serialize)]
struct Request {
    jsonrpc: String,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct Response {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u64,
    result: Option<Value>,
    error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    code: i32,
    message: String,
}

pub struct DaemonClient;

impl DaemonClient {
    pub fn connect() -> Result<Self, ClientError> {
        let path = socket_path();
        if !path.exists() {
            return Err(ClientError::DaemonNotRunning);
        }

        let stream = UnixStream::connect(&path)?;
        drop(stream);

        Ok(Self)
    }

    pub fn is_daemon_running() -> bool {
        let path = socket_path();
        if !path.exists() {
            return false;
        }

        UnixStream::connect(path).is_ok()
    }

    pub fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value, ClientError> {
        let path = socket_path();
        let mut stream = UnixStream::connect(&path)?;

        stream.set_read_timeout(Some(Duration::from_secs(60)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;

        let request = Request {
            jsonrpc: "2.0".to_string(),
            id: REQUEST_ID.fetch_add(1, Ordering::SeqCst),
            method: method.to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)?;

        writeln!(stream, "{}", request_json)?;
        stream.flush()?;

        let mut reader = BufReader::new(&stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line)?;

        let response: Response = serde_json::from_str(&response_line)?;

        if let Some(error) = response.error {
            return Err(ClientError::RpcError {
                code: error.code,
                message: error.message,
            });
        }

        response.result.ok_or(ClientError::InvalidResponse)
    }
}

pub fn start_daemon_background() -> Result<(), ClientError> {
    use std::process::{Command, Stdio};

    let exe = std::env::current_exe()?;

    Command::new(exe)
        .arg("daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if DaemonClient::is_daemon_running() {
            return Ok(());
        }
    }

    Err(ClientError::DaemonNotRunning)
}

pub fn ensure_daemon() -> Result<DaemonClient, ClientError> {
    if !DaemonClient::is_daemon_running() {
        start_daemon_background()?;
    }

    DaemonClient::connect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Request serialization tests
    // =========================================================================

    #[test]
    fn test_request_serializes_to_jsonrpc_2_0() {
        let request = Request {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "health".to_string(),
            params: None,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"health\""));
        assert!(!json.contains("\"params\"")); // skip_serializing_if = None
    }

    #[test]
    fn test_request_serializes_with_params() {
        let request = Request {
            jsonrpc: "2.0".to_string(),
            id: 42,
            method: "spawn".to_string(),
            params: Some(serde_json::json!({"command": "bash", "cols": 80})),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"params\""));
        assert!(json.contains("\"command\":\"bash\""));
        assert!(json.contains("\"cols\":80"));
    }

    #[test]
    fn test_request_id_increments() {
        let id1 = REQUEST_ID.load(std::sync::atomic::Ordering::SeqCst);
        let _ = REQUEST_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let id2 = REQUEST_ID.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(id2, id1 + 1);
    }

    // =========================================================================
    // Response deserialization tests
    // =========================================================================

    #[test]
    fn test_response_deserializes_success_result() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"status":"ok"}}"#;
        let response: Response = serde_json::from_str(json).unwrap();
        assert!(response.result.is_some());
        assert!(response.error.is_none());
        let result = response.result.unwrap();
        assert_eq!(result["status"], "ok");
    }

    #[test]
    fn test_response_deserializes_error() {
        let json =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
        let response: Response = serde_json::from_str(json).unwrap();
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32600);
        assert_eq!(error.message, "Invalid Request");
    }

    #[test]
    fn test_response_deserializes_with_null_result() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":null}"#;
        let response: Response = serde_json::from_str(json).unwrap();
        assert!(response.error.is_none());
    }

    // =========================================================================
    // ClientError display tests
    // =========================================================================

    #[test]
    fn test_client_error_daemon_not_running_display() {
        let err = ClientError::DaemonNotRunning;
        assert_eq!(err.to_string(), "Daemon not running");
    }

    #[test]
    fn test_client_error_invalid_response_display() {
        let err = ClientError::InvalidResponse;
        assert_eq!(err.to_string(), "Invalid response from daemon");
    }

    #[test]
    fn test_client_error_rpc_error_display() {
        let err = ClientError::RpcError {
            code: -32601,
            message: "Method not found".to_string(),
        };
        assert_eq!(err.to_string(), "RPC error (-32601): Method not found");
    }

    #[test]
    fn test_client_error_connection_failed_display() {
        let io_err =
            std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "connection refused");
        let err = ClientError::ConnectionFailed(io_err);
        assert!(err.to_string().contains("Failed to connect to daemon"));
    }

    #[test]
    fn test_client_error_serialization_failed_display() {
        // Create a JSON error by attempting to parse invalid JSON
        let json_err = serde_json::from_str::<Value>("invalid").unwrap_err();
        let err = ClientError::SerializationFailed(json_err);
        assert!(err.to_string().contains("Failed to serialize request"));
    }
}
