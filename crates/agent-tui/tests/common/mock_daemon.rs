//! Mock daemon for testing CLI behavior without a real daemon
//!
//! The MockDaemon listens on a Unix socket (or TCP for testing) and
//! responds to JSON-RPC requests with predefined responses.
//!
//! ## Anti-Gaming Design
//!
//! This mock is designed to catch real regressions:
//! 1. Validates exact JSON field names (catches serialization bugs)
//! 2. Returns realistic responses (catches parsing bugs)
//! 3. Can simulate failures and edge cases
//! 4. Records all requests for verification

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::oneshot;

/// JSON-RPC request structure (mirrors protocol.rs)
#[derive(Debug, Deserialize)]
struct Request {
    jsonrpc: String,
    id: u64,
    method: String,
    params: Option<Value>,
}

/// JSON-RPC response structure
#[derive(Debug, Serialize)]
struct Response {
    jsonrpc: String,
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
struct RpcError {
    code: i32,
    message: String,
}

/// Recorded request for test verification
#[derive(Debug, Clone)]
pub struct RecordedRequest {
    pub method: String,
    pub params: Option<Value>,
}

/// Configuration for how the mock should respond
#[derive(Debug, Clone)]
pub enum MockResponse {
    /// Return a successful result
    Success(Value),
    /// Return an error
    Error { code: i32, message: String },
    /// Return malformed JSON (for error handling tests)
    Malformed(String),
    /// Hang forever (for timeout tests) - use with small timeout
    Hang,
    /// Close connection immediately
    Disconnect,
}

/// Mock daemon for testing
pub struct MockDaemon {
    /// Temporary directory containing the socket
    _temp_dir: TempDir,
    /// Path to the socket file
    socket_path: PathBuf,
    /// Path to the PID file
    pid_path: PathBuf,
    /// Shutdown signal sender
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// Recorded requests for verification
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    /// Custom response handlers by method
    handlers: Arc<Mutex<HashMap<String, MockResponse>>>,
}

impl MockDaemon {
    /// Create and start a new mock daemon
    pub async fn start() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let socket_path = temp_dir.path().join("agent-tui.sock");
        let pid_path = temp_dir.path().join("agent-tui.pid");

        // Create PID file (required by CLI to detect running daemon)
        std::fs::write(&pid_path, format!("{}", std::process::id()))
            .expect("Failed to create PID file");

        let requests = Arc::new(Mutex::new(Vec::new()));
        let handlers = Arc::new(Mutex::new(HashMap::new()));

        // Set up default handlers
        {
            let mut h = handlers.lock().unwrap();
            h.insert(
                "ping".to_string(),
                MockResponse::Success(serde_json::json!({
                    "status": "ok"
                })),
            );
            h.insert(
                "health".to_string(),
                MockResponse::Success(serde_json::json!({
                    "status": "healthy",
                    "pid": super::TEST_PID,
                    "uptime_ms": 60000,
                    "session_count": 0,
                    "version": "1.0.0-test"
                })),
            );
            h.insert(
                "spawn".to_string(),
                MockResponse::Success(serde_json::json!({
                    "session_id": super::TEST_SESSION_ID,
                    "pid": super::TEST_PID
                })),
            );
            h.insert(
                "sessions".to_string(),
                MockResponse::Success(serde_json::json!({
                    "sessions": [],
                    "active_session": null
                })),
            );
            h.insert(
                "snapshot".to_string(),
                MockResponse::Success(serde_json::json!({
                    "session_id": super::TEST_SESSION_ID,
                    "screen": "Test screen content\n",
                    "elements": [],
                    "cursor": { "row": 0, "col": 0, "visible": true },
                    "size": { "cols": super::TEST_COLS, "rows": super::TEST_ROWS }
                })),
            );
            // Deprecated method returns error
            h.insert(
                "screen".to_string(),
                MockResponse::Error {
                    code: -32601,
                    message: "Method 'screen' is deprecated. Use 'snapshot' with strip_ansi=true instead.".to_string(),
                },
            );
            h.insert(
                "click".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true,
                    "message": null
                })),
            );
            h.insert(
                "fill".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true,
                    "message": null
                })),
            );
            h.insert(
                "keystroke".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true
                })),
            );
            h.insert(
                "type".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true
                })),
            );
            h.insert(
                "kill".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true,
                    "session_id": super::TEST_SESSION_ID
                })),
            );
            h.insert(
                "wait".to_string(),
                MockResponse::Success(serde_json::json!({
                    "found": true,
                    "elapsed_ms": 100
                })),
            );
            h.insert(
                "get_value".to_string(),
                MockResponse::Success(serde_json::json!({
                    "ref": "@inp1",
                    "value": "test-value",
                    "found": true
                })),
            );
            h.insert(
                "get_text".to_string(),
                MockResponse::Success(serde_json::json!({
                    "ref": "@btn1",
                    "text": "Test Text",
                    "found": true
                })),
            );
            h.insert(
                "is_visible".to_string(),
                MockResponse::Success(serde_json::json!({
                    "visible": true,
                    "ref": "@btn1"
                })),
            );
            h.insert(
                "is_focused".to_string(),
                MockResponse::Success(serde_json::json!({
                    "ref": "@inp1",
                    "focused": true,
                    "found": true
                })),
            );
            h.insert(
                "is_enabled".to_string(),
                MockResponse::Success(serde_json::json!({
                    "ref": "@btn1",
                    "enabled": true,
                    "found": true
                })),
            );
            h.insert(
                "is_checked".to_string(),
                MockResponse::Success(serde_json::json!({
                    "ref": "@cb1",
                    "checked": true,
                    "found": true
                })),
            );
            h.insert(
                "count".to_string(),
                MockResponse::Success(serde_json::json!({
                    "count": 5
                })),
            );
            h.insert(
                "focus".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true,
                    "ref": "@inp1"
                })),
            );
            h.insert(
                "clear".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true,
                    "ref": "@inp1"
                })),
            );
            h.insert(
                "select".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true,
                    "ref": "@sel1",
                    "option": "option1"
                })),
            );
            h.insert(
                "scroll".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true,
                    "direction": "down",
                    "amount": 5
                })),
            );
            h.insert(
                "toggle".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true,
                    "message": null,
                    "checked": true
                })),
            );
            h.insert(
                "resize".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true,
                    "session_id": super::TEST_SESSION_ID,
                    "size": { "cols": super::TEST_COLS, "rows": super::TEST_ROWS }
                })),
            );
            h.insert(
                "attach".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true,
                    "session": super::TEST_SESSION_ID
                })),
            );
            h.insert(
                "record_start".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true,
                    "session_id": super::TEST_SESSION_ID,
                    "message": null
                })),
            );
            h.insert(
                "record_stop".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true,
                    "session_id": super::TEST_SESSION_ID,
                    "frame_count": 100,
                    "duration_ms": 5000,
                    "output_file": null
                })),
            );
            h.insert(
                "record_status".to_string(),
                MockResponse::Success(serde_json::json!({
                    "session_id": super::TEST_SESSION_ID,
                    "is_recording": false,
                    "frame_count": null,
                    "duration_ms": null
                })),
            );
            h.insert(
                "trace".to_string(),
                MockResponse::Success(serde_json::json!({
                    "session_id": super::TEST_SESSION_ID,
                    "is_tracing": false,
                    "entries": [],
                    "formatted": null
                })),
            );
            h.insert(
                "console".to_string(),
                MockResponse::Success(serde_json::json!({
                    "session_id": super::TEST_SESSION_ID,
                    "lines": ["line 1", "line 2"],
                    "total_lines": 2
                })),
            );
            h.insert(
                "find".to_string(),
                MockResponse::Success(serde_json::json!({
                    "elements": [],
                    "count": 0
                })),
            );
            h.insert(
                "keydown".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true
                })),
            );
            h.insert(
                "keyup".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true
                })),
            );
            h.insert(
                "scroll_into_view".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true,
                    "message": null
                })),
            );
            h.insert(
                "multiselect".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true,
                    "message": null,
                    "selected_options": ["option1", "option2"]
                })),
            );
            h.insert(
                "dbl_click".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true,
                    "message": null
                })),
            );
            h.insert(
                "restart".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true,
                    "old_session_id": super::TEST_SESSION_ID,
                    "new_session_id": "new-session-xyz789",
                    "command": "bash",
                    "pid": super::TEST_PID
                })),
            );
            h.insert(
                "select_all".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true,
                    "ref": "@inp1"
                })),
            );
            h.insert(
                "get_focused".to_string(),
                MockResponse::Success(serde_json::json!({
                    "ref": "@inp1",
                    "type": "input",
                    "label": "Name",
                    "found": true
                })),
            );
            h.insert(
                "get_title".to_string(),
                MockResponse::Success(serde_json::json!({
                    "title": "bash",
                    "session_id": super::TEST_SESSION_ID
                })),
            );
            h.insert(
                "errors".to_string(),
                MockResponse::Success(serde_json::json!({
                    "session_id": super::TEST_SESSION_ID,
                    "errors": [],
                    "total_count": 0
                })),
            );
        }

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Start the listener
        let listener = UnixListener::bind(&socket_path).expect("Failed to bind socket");
        let requests_clone = requests.clone();
        let handlers_clone = handlers.clone();

        tokio::spawn(async move {
            Self::run_server(listener, requests_clone, handlers_clone, shutdown_rx).await;
        });

        // Give the server a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        Self {
            _temp_dir: temp_dir,
            socket_path,
            pid_path,
            shutdown_tx: Some(shutdown_tx),
            requests,
            handlers,
        }
    }

    /// Get the socket path for environment configuration
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }

    /// Set a custom response for a method
    pub fn set_response(&self, method: &str, response: MockResponse) {
        let mut handlers = self.handlers.lock().unwrap();
        handlers.insert(method.to_string(), response);
    }

    /// Get all recorded requests
    pub fn get_requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().unwrap().clone()
    }

    /// Clear recorded requests
    pub fn clear_requests(&self) {
        self.requests.lock().unwrap().clear();
    }

    /// Get the last request for a specific method
    pub fn last_request_for(&self, method: &str) -> Option<RecordedRequest> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|r| r.method == method)
            .cloned()
    }

    /// Get environment variables to point CLI at this mock daemon
    pub fn env_vars(&self) -> Vec<(&'static str, String)> {
        vec![
            (
                "XDG_RUNTIME_DIR",
                self.socket_path
                    .parent()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            ),
            (
                "TMPDIR",
                self.socket_path
                    .parent()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            ),
        ]
    }

    async fn run_server(
        listener: UnixListener,
        requests: Arc<Mutex<Vec<RecordedRequest>>>,
        handlers: Arc<Mutex<HashMap<String, MockResponse>>>,
        mut shutdown_rx: oneshot::Receiver<()>,
    ) {
        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, _)) => {
                            let requests = requests.clone();
                            let handlers = handlers.clone();
                            tokio::spawn(async move {
                                Self::handle_connection(stream, requests, handlers).await;
                            });
                        }
                        Err(e) => {
                            eprintln!("Mock daemon accept error: {}", e);
                            break;
                        }
                    }
                }
                _ = &mut shutdown_rx => {
                    break;
                }
            }
        }
    }

    async fn handle_connection(
        stream: tokio::net::UnixStream,
        requests: Arc<Mutex<Vec<RecordedRequest>>>,
        handlers: Arc<Mutex<HashMap<String, MockResponse>>>,
    ) {
        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();

        while buf_reader.read_line(&mut line).await.is_ok() {
            if line.is_empty() {
                break;
            }

            // Parse the request
            let request: Request = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Mock daemon parse error: {} for line: {}", e, line);
                    line.clear();
                    continue;
                }
            };

            // Record the request
            {
                let mut reqs = requests.lock().unwrap();
                reqs.push(RecordedRequest {
                    method: request.method.clone(),
                    params: request.params.clone(),
                });
            }

            // Get the handler for this method
            let handler = {
                let h = handlers.lock().unwrap();
                h.get(&request.method).cloned()
            };

            let response_str = match handler {
                Some(MockResponse::Success(result)) => {
                    let response = Response {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(result),
                        error: None,
                    };
                    serde_json::to_string(&response).unwrap()
                }
                Some(MockResponse::Error { code, message }) => {
                    let response = Response {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: None,
                        error: Some(RpcError { code, message }),
                    };
                    serde_json::to_string(&response).unwrap()
                }
                Some(MockResponse::Malformed(s)) => s,
                Some(MockResponse::Hang) => {
                    // Sleep forever (will be killed when test times out)
                    tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
                    return;
                }
                Some(MockResponse::Disconnect) => {
                    return;
                }
                None => {
                    // Unknown method error
                    let response = Response {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: None,
                        error: Some(RpcError {
                            code: -32601,
                            message: format!("Method not found: {}", request.method),
                        }),
                    };
                    serde_json::to_string(&response).unwrap()
                }
            };

            // Send response
            if writer.write_all(response_str.as_bytes()).await.is_err() {
                break;
            }
            if writer.write_all(b"\n").await.is_err() {
                break;
            }
            if writer.flush().await.is_err() {
                break;
            }

            line.clear();
        }
    }
}

impl Drop for MockDaemon {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_daemon_responds_to_ping() {
        let daemon = MockDaemon::start().await;
        let socket_path = daemon.socket_path();

        // Connect directly and send a ping
        let stream = tokio::net::UnixStream::connect(socket_path).await.unwrap();
        let (reader, mut writer) = stream.into_split();

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "ping"
        });
        writer
            .write_all(request.to_string().as_bytes())
            .await
            .unwrap();
        writer.write_all(b"\n").await.unwrap();
        writer.flush().await.unwrap();

        let mut buf_reader = BufReader::new(reader);
        let mut response_line = String::new();
        buf_reader.read_line(&mut response_line).await.unwrap();

        let response: Value = serde_json::from_str(&response_line).unwrap();
        assert_eq!(response["result"]["status"], "ok");

        // Verify request was recorded
        let requests = daemon.get_requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "ping");
    }

    #[tokio::test]
    async fn test_mock_daemon_custom_response() {
        let daemon = MockDaemon::start().await;

        // Set custom response
        daemon.set_response(
            "spawn",
            MockResponse::Error {
                code: -32000,
                message: "Test error".to_string(),
            },
        );

        let socket_path = daemon.socket_path();
        let stream = tokio::net::UnixStream::connect(socket_path).await.unwrap();
        let (reader, mut writer) = stream.into_split();

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "spawn",
            "params": { "command": "bash" }
        });
        writer
            .write_all(request.to_string().as_bytes())
            .await
            .unwrap();
        writer.write_all(b"\n").await.unwrap();
        writer.flush().await.unwrap();

        let mut buf_reader = BufReader::new(reader);
        let mut response_line = String::new();
        buf_reader.read_line(&mut response_line).await.unwrap();

        let response: Value = serde_json::from_str(&response_line).unwrap();
        assert!(response["error"].is_object());
        assert_eq!(response["error"]["message"], "Test error");
    }
}
