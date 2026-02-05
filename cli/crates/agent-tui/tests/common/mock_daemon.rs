#![expect(dead_code, reason = "Test harness helpers are used selectively.")]
#![expect(clippy::print_stderr, reason = "Test diagnostics for mock daemon")]

//! Mock daemon for integration tests.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
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

struct DelayState {
    now_ms: u64,
    pending: Vec<(u64, oneshot::Sender<()>)>,
}

struct DelayController {
    state: Mutex<DelayState>,
    pending_cvar: Condvar,
}

impl DelayController {
    fn new() -> Self {
        Self {
            state: Mutex::new(DelayState {
                now_ms: 0,
                pending: Vec::new(),
            }),
            pending_cvar: Condvar::new(),
        }
    }

    fn register(&self, duration: Duration) -> oneshot::Receiver<()> {
        let mut state = self.state.lock().unwrap();
        let target = state
            .now_ms
            .saturating_add(duration.as_millis().min(u64::MAX as u128) as u64);
        let (tx, rx) = oneshot::channel();
        state.pending.push((target, tx));
        self.pending_cvar.notify_all();
        rx
    }

    fn advance(&self, duration: Duration) {
        let mut ready = Vec::new();
        {
            let mut state = self.state.lock().unwrap();
            state.now_ms = state
                .now_ms
                .saturating_add(duration.as_millis().min(u64::MAX as u128) as u64);
            let now_ms = state.now_ms;
            let mut remaining = Vec::with_capacity(state.pending.len());
            for (target, tx) in state.pending.drain(..) {
                if target <= now_ms {
                    ready.push(tx);
                } else {
                    remaining.push((target, tx));
                }
            }
            state.pending = remaining;
        }
        for tx in ready {
            let _ = tx.send(());
        }
    }

    fn wait_for_pending(&self, count: usize) {
        let mut state = self.state.lock().unwrap();
        while state.pending.len() < count {
            state = self.pending_cvar.wait(state).unwrap();
        }
    }
}

struct RequestCounter {
    count: Mutex<usize>,
    cvar: Condvar,
}

impl RequestCounter {
    fn new() -> Self {
        Self {
            count: Mutex::new(0),
            cvar: Condvar::new(),
        }
    }

    fn increment(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        self.cvar.notify_all();
    }

    fn wait_for(&self, target: usize) {
        let mut count = self.count.lock().unwrap();
        while *count < target {
            count = self.cvar.wait(count).unwrap();
        }
    }

    fn reset(&self) {
        let mut count = self.count.lock().unwrap();
        *count = 0;
        self.cvar.notify_all();
    }
}

pub struct MockDaemon {
    _temp_dir: TempDir,
    tcp_addr: std::net::SocketAddr,
    pid_path: PathBuf,
    shutdown_tx: Option<oneshot::Sender<()>>,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    request_counter: Arc<RequestCounter>,
    delay_controller: Arc<DelayController>,
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
        let request_counter = Arc::new(RequestCounter::new());
        let delay_controller = Arc::new(DelayController::new());
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
                "version".to_string(),
                MockResponse::Success(serde_json::json!({
                    "daemon_version": "1.0.0-test",
                    "daemon_commit": "abc123"
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
                    "cursor": { "row": 0, "col": 0, "visible": true }
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
                "resize".to_string(),
                MockResponse::Success(serde_json::json!({
                    "success": true,
                    "session_id": super::TEST_SESSION_ID,
                    "cols": super::TEST_COLS,
                    "rows": super::TEST_ROWS
                })),
            );
            h.insert(
                "attach".to_string(),
                MockResponse::Success(serde_json::json!({
                    "session_id": super::TEST_SESSION_ID,
                    "success": true,
                    "message": null
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
                "restart".to_string(),
                MockResponse::Success(serde_json::json!({
                    "old_session_id": super::TEST_SESSION_ID,
                    "new_session_id": "new-session-xyz789",
                    "command": "bash",
                    "pid": super::TEST_PID
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
        let request_counter_clone = request_counter.clone();
        let delay_controller_clone = delay_controller.clone();
        let handlers_clone = handlers.clone();
        let sequence_counters_clone = sequence_counters.clone();

        tokio::spawn(async move {
            Self::run_server(
                listener,
                requests_clone,
                request_counter_clone,
                delay_controller_clone,
                handlers_clone,
                sequence_counters_clone,
                shutdown_rx,
            )
            .await;
        });

        Self {
            _temp_dir: temp_dir,
            tcp_addr,
            pid_path,
            shutdown_tx: Some(shutdown_tx),
            requests,
            request_counter,
            delay_controller,
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
        self.request_counter.reset();
    }

    pub fn wait_for_request_count(&self, count: usize) {
        self.request_counter.wait_for(count);
    }

    pub fn wait_for_pending_delays(&self, count: usize) {
        self.delay_controller.wait_for_pending(count);
    }

    pub fn advance_time(&self, duration: Duration) {
        self.delay_controller.advance(duration);
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
        request_counter: Arc<RequestCounter>,
        delay_controller: Arc<DelayController>,
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
                            let request_counter = request_counter.clone();
                            let delay_controller = delay_controller.clone();
                            let handlers = handlers.clone();
                            let sequence_counters = sequence_counters.clone();
                            tokio::spawn(async move {
                                Self::handle_connection(
                                    stream,
                                    requests,
                                    request_counter,
                                    delay_controller,
                                    handlers,
                                    sequence_counters,
                                )
                                .await;
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
        request_counter: Arc<RequestCounter>,
        delay_controller: Arc<DelayController>,
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
            request_counter.increment();

            let handler = {
                let h = handlers.lock().unwrap();
                h.get(&request.method).cloned()
            };

            let resolved_handler =
                Self::resolve_handler(handler, &request.method, &sequence_counters);

            let response_str = Self::generate_response(
                resolved_handler,
                request.id,
                &request.method,
                &delay_controller,
            )
            .await;

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
        delay_controller: &Arc<DelayController>,
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
                let rx = delay_controller.register(Duration::from_secs(3600));
                let _ = rx.await;
                None
            }
            Some(MockResponse::Disconnect) => None,
            Some(MockResponse::Delayed(duration, inner)) => {
                let rx = delay_controller.register(duration);
                let _ = rx.await;

                Box::pin(Self::generate_response(
                    Some(*inner),
                    request_id,
                    method,
                    delay_controller,
                ))
                .await
            }
            Some(MockResponse::JunkThen(next, junk)) => {
                // send junk first, then the next response
                let mut out = String::new();
                out.push_str(&junk);
                let rest = Box::pin(Self::generate_response(
                    Some(*next),
                    request_id,
                    method,
                    delay_controller,
                ))
                .await;
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
