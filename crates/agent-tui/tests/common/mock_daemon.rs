#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::TempDir;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

#[derive(Debug, Deserialize)]
struct Request {
    jsonrpc: String,
    id: u64,
    method: String,
    params: Option<Value>,
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct RecordedRequest {
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone)]
pub enum MockResponse {
    Success(Value),
    Error {
        code: i32,
        message: String,
    },
    StructuredError {
        code: i32,
        message: String,
        category: Option<String>,
        retryable: Option<bool>,
        context: Option<Value>,
        suggestion: Option<String>,
    },
    Malformed(String),
    Hang,
    Disconnect,
    Sequence(Vec<MockResponse>),
    Delayed(Duration, Box<MockResponse>),
    // Inject arbitrary line (not JSON) to simulate protocol-level garbage before a valid frame.
    JunkThen(Box<MockResponse>, String),
}

pub struct MockDaemon {
    _temp_dir: TempDir,
    tcp_addr: std::net::SocketAddr,
    pid_path: PathBuf,
    shutdown_tx: Option<oneshot::Sender<()>>,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    handlers: Arc<Mutex<HashMap<String, MockResponse>>>,
    sequence_counters: Arc<Mutex<HashMap<String, usize>>>,
}

impl MockDaemon {
    pub async fn start() -> Self {
        let temp_dir = tokio::task::spawn_blocking(|| TempDir::new_in("/tmp"))
            .await
            .expect("Temp dir task panicked")
            .expect("Failed to create temp dir");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ =
                fs::set_permissions(temp_dir.path(), std::fs::Permissions::from_mode(0o777)).await;
        }
        let pid_path = temp_dir.path().join("agent-tui.pid");

        fs::write(&pid_path, format!("{}", std::process::id()))
            .await
            .expect("Failed to create PID file");

        let requests = Arc::new(Mutex::new(Vec::new()));
        let handlers = Arc::new(Mutex::new(HashMap::new()));
        let sequence_counters = Arc::new(Mutex::new(HashMap::new()));

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
                    "screenshot": "Test screen content\n",
                    "elements": [],
                    "cursor": { "row": 0, "col": 0, "visible": true },
                    "size": { "cols": super::TEST_COLS, "rows": super::TEST_ROWS }
                })),
            );
            h.insert(
                "accessibility_snapshot".to_string(),
                MockResponse::Success(serde_json::json!({
                    "session_id": super::TEST_SESSION_ID,
                    "tree": "- button \"OK\" [ref=e1]\n- textbox \"Input\" [ref=e2]",
                    "refs": {
                        "e1": { "row": 5, "col": 10, "width": 4, "height": 1 },
                        "e2": { "row": 7, "col": 10, "width": 20, "height": 1 }
                    },
                    "stats": {
                        "total_elements": 2,
                        "interactive_elements": 2,
                        "filtered_elements": 0
                    }
                })),
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
                "pty_read".to_string(),
                MockResponse::Success(serde_json::json!({
                    "session_id": super::TEST_SESSION_ID,
                    "data": "",
                    "bytes_read": 0
                })),
            );
            h.insert(
                "pty_write".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true,
                    "session_id": super::TEST_SESSION_ID
                })),
            );
        }

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind TCP listener");
        let tcp_addr = listener.local_addr().expect("Failed to get TCP addr");
        let requests_clone = requests.clone();
        let handlers_clone = handlers.clone();
        let sequence_counters_clone = sequence_counters.clone();

        tokio::spawn(async move {
            Self::run_server(
                listener,
                requests_clone,
                handlers_clone,
                sequence_counters_clone,
                shutdown_rx,
            )
            .await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        Self {
            _temp_dir: temp_dir,
            tcp_addr,
            pid_path,
            shutdown_tx: Some(shutdown_tx),
            requests,
            handlers,
            sequence_counters,
        }
    }

    pub fn tcp_addr(&self) -> std::net::SocketAddr {
        self.tcp_addr
    }

    pub fn set_response(&self, method: &str, response: MockResponse) {
        let mut handlers = self.handlers.lock().unwrap();
        handlers.insert(method.to_string(), response);
    }

    pub fn get_requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().unwrap().clone()
    }

    pub fn clear_requests(&self) {
        self.requests.lock().unwrap().clear();
    }

    pub fn last_request_for(&self, method: &str) -> Option<RecordedRequest> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|r| r.method == method)
            .cloned()
    }

    pub fn call_count_for(&self, method: &str) -> usize {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.method == method)
            .count()
    }

    pub fn nth_call_for(&self, method: &str, n: usize) -> Option<RecordedRequest> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.method == method)
            .nth(n)
            .cloned()
    }

    pub fn reset_sequence(&self, method: &str) {
        let mut counters = self.sequence_counters.lock().unwrap();
        counters.remove(method);
    }

    pub fn reset_all_sequences(&self) {
        let mut counters = self.sequence_counters.lock().unwrap();
        counters.clear();
    }

    pub fn env_vars(&self) -> Vec<(&'static str, String)> {
        vec![
            ("AGENT_TUI_TRANSPORT", "tcp".to_string()),
            ("AGENT_TUI_TCP_ADDR", self.tcp_addr.to_string()),
            (
                "TMPDIR",
                self._temp_dir.path().to_string_lossy().into_owned(),
            ),
        ]
    }

    async fn run_server(
        listener: TcpListener,
        requests: Arc<Mutex<Vec<RecordedRequest>>>,
        handlers: Arc<Mutex<HashMap<String, MockResponse>>>,
        sequence_counters: Arc<Mutex<HashMap<String, usize>>>,
        mut shutdown_rx: oneshot::Receiver<()>,
    ) {
        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, _)) => {
                            let requests = requests.clone();
                            let handlers = handlers.clone();
                            let sequence_counters = sequence_counters.clone();
                            tokio::spawn(async move {
                                Self::handle_connection(stream, requests, handlers, sequence_counters).await;
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
        stream: TcpStream,
        requests: Arc<Mutex<Vec<RecordedRequest>>>,
        handlers: Arc<Mutex<HashMap<String, MockResponse>>>,
        sequence_counters: Arc<Mutex<HashMap<String, usize>>>,
    ) {
        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();

        while buf_reader.read_line(&mut line).await.is_ok() {
            if line.is_empty() {
                break;
            }

            let request: Request = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Mock daemon parse error: {} for line: {}", e, line);
                    line.clear();
                    continue;
                }
            };

            {
                let mut reqs = requests.lock().unwrap();
                reqs.push(RecordedRequest {
                    method: request.method.clone(),
                    params: request.params.clone(),
                });
            }

            let handler = {
                let h = handlers.lock().unwrap();
                h.get(&request.method).cloned()
            };

            let resolved_handler =
                Self::resolve_handler(handler, &request.method, &sequence_counters);

            let response_str =
                Self::generate_response(resolved_handler, request.id, &request.method).await;

            let Some(response_str) = response_str else {
                return;
            };

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

    fn resolve_handler(
        handler: Option<MockResponse>,
        method: &str,
        sequence_counters: &Arc<Mutex<HashMap<String, usize>>>,
    ) -> Option<MockResponse> {
        match handler {
            Some(MockResponse::Sequence(responses)) if !responses.is_empty() => {
                let mut counters = sequence_counters.lock().unwrap();
                let index = counters.entry(method.to_string()).or_insert(0);
                let response = responses[*index % responses.len()].clone();
                *index += 1;

                drop(counters);
                Self::resolve_handler(Some(response), method, sequence_counters)
            }
            other => other,
        }
    }

    async fn generate_response(
        handler: Option<MockResponse>,
        request_id: u64,
        method: &str,
    ) -> Option<String> {
        match handler {
            Some(MockResponse::Success(result)) => {
                let response = Response {
                    jsonrpc: "2.0".to_string(),
                    id: request_id,
                    result: Some(result),
                    error: None,
                };
                Some(serde_json::to_string(&response).unwrap())
            }
            Some(MockResponse::Error { code, message }) => {
                let response = Response {
                    jsonrpc: "2.0".to_string(),
                    id: request_id,
                    result: None,
                    error: Some(RpcError {
                        code,
                        message,
                        data: None,
                    }),
                };
                Some(serde_json::to_string(&response).unwrap())
            }
            Some(MockResponse::StructuredError {
                code,
                message,
                category,
                retryable,
                context,
                suggestion,
            }) => {
                let mut data = serde_json::json!({});
                if let Some(cat) = category {
                    data["category"] = serde_json::json!(cat);
                }
                if let Some(retry) = retryable {
                    data["retryable"] = serde_json::json!(retry);
                }
                if let Some(ctx) = context {
                    data["context"] = ctx;
                }
                if let Some(sug) = suggestion {
                    data["suggestion"] = serde_json::json!(sug);
                }
                let response = Response {
                    jsonrpc: "2.0".to_string(),
                    id: request_id,
                    result: None,
                    error: Some(RpcError {
                        code,
                        message,
                        data: if data.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                            None
                        } else {
                            Some(data)
                        },
                    }),
                };
                Some(serde_json::to_string(&response).unwrap())
            }
            Some(MockResponse::Malformed(s)) => Some(s),
            Some(MockResponse::Hang) => {
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
                None
            }
            Some(MockResponse::Disconnect) => None,
            Some(MockResponse::Delayed(duration, inner)) => {
                tokio::time::sleep(duration).await;

                Box::pin(Self::generate_response(Some(*inner), request_id, method)).await
            }
            Some(MockResponse::JunkThen(next, junk)) => {
                // send junk first, then the next response
                let mut out = String::new();
                out.push_str(&junk);
                let rest = Box::pin(Self::generate_response(Some(*next), request_id, method)).await;
                if let Some(rest) = rest {
                    if !out.is_empty() {
                        out.push('\n');
                    }
                    out.push_str(&rest);
                }
                Some(out)
            }
            Some(MockResponse::Sequence(_)) => {
                unreachable!("Sequence should be resolved before generate_response")
            }
            None => {
                let response = Response {
                    jsonrpc: "2.0".to_string(),
                    id: request_id,
                    result: None,
                    error: Some(RpcError {
                        code: -32601,
                        message: format!("Method not found: {}", method),
                        data: None,
                    }),
                };
                Some(serde_json::to_string(&response).unwrap())
            }
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
        let stream = tokio::net::TcpStream::connect(daemon.tcp_addr())
            .await
            .unwrap();
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

        let requests = daemon.get_requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "ping");
    }

    #[tokio::test]
    async fn test_mock_daemon_custom_response() {
        let daemon = MockDaemon::start().await;

        daemon.set_response(
            "spawn",
            MockResponse::Error {
                code: -32000,
                message: "Test error".to_string(),
            },
        );

        let stream = tokio::net::TcpStream::connect(daemon.tcp_addr())
            .await
            .unwrap();
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
