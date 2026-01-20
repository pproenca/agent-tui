//! Client for connecting to the daemon
//!
//! Provides a simple interface to send JSON-RPC requests to the daemon.

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

/// Client for communicating with the daemon
pub struct DaemonClient;

impl DaemonClient {
    /// Connect to the daemon
    pub fn connect() -> Result<Self, ClientError> {
        let path = socket_path();
        if !path.exists() {
            return Err(ClientError::DaemonNotRunning);
        }

        // Verify the daemon is responding
        let stream = UnixStream::connect(&path)?;
        drop(stream);

        Ok(Self)
    }

    /// Check if daemon is running
    pub fn is_daemon_running() -> bool {
        let path = socket_path();
        if !path.exists() {
            return false;
        }

        // Try to connect to verify it's actually running
        UnixStream::connect(path).is_ok()
    }

    /// Send a request and get the response (uses new connection per request)
    pub fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value, ClientError> {
        let path = socket_path();
        let mut stream = UnixStream::connect(&path)?;

        // Set timeouts
        stream.set_read_timeout(Some(Duration::from_secs(60)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;

        let request = Request {
            jsonrpc: "2.0".to_string(),
            id: REQUEST_ID.fetch_add(1, Ordering::SeqCst),
            method: method.to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)?;

        // Write request
        writeln!(stream, "{}", request_json)?;
        stream.flush()?;

        // Read response
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

    /// Convenience method for ping
    #[allow(dead_code)]
    pub fn ping(&mut self) -> Result<bool, ClientError> {
        let result = self.call("ping", None)?;
        Ok(result
            .get("pong")
            .and_then(|v| v.as_bool())
            .unwrap_or(false))
    }
}

/// Start the daemon in the background
pub fn start_daemon_background() -> Result<(), ClientError> {
    use std::process::{Command, Stdio};

    let exe = std::env::current_exe()?;

    // Fork a daemon process
    Command::new(exe)
        .arg("daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    // Wait for daemon to start
    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if DaemonClient::is_daemon_running() {
            return Ok(());
        }
    }

    Err(ClientError::DaemonNotRunning)
}

/// Ensure daemon is running, starting it if necessary
pub fn ensure_daemon() -> Result<DaemonClient, ClientError> {
    if !DaemonClient::is_daemon_running() {
        start_daemon_background()?;
    }

    DaemonClient::connect()
}
