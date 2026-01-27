use std::io::BufRead;
use std::io::BufReader;
use std::io::ErrorKind;
use std::io::Write;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tracing::{debug, trace};

use crate::adapters::ipc::error::ClientError;
use crate::adapters::ipc::error_codes;
use crate::adapters::ipc::socket::socket_path;
use crate::adapters::ipc::transport::ClientStream;
use crate::adapters::ipc::transport::IpcTransport;
use crate::adapters::ipc::transport::default_transport;
use crate::common::Colors;

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct DaemonClientConfig {
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub max_retries: u32,
    pub initial_retry_delay: Duration,
}

impl Default for DaemonClientConfig {
    fn default() -> Self {
        Self {
            read_timeout: Duration::from_secs(60),
            write_timeout: Duration::from_secs(10),
            max_retries: 3,
            initial_retry_delay: Duration::from_millis(100),
        }
    }
}

impl DaemonClientConfig {
    pub fn with_read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = timeout;
        self
    }

    pub fn with_write_timeout(mut self, timeout: Duration) -> Self {
        self.write_timeout = timeout;
        self
    }

    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }
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
    #[serde(default)]
    data: Option<Value>,
}

pub trait DaemonClient: Send + Sync {
    fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value, ClientError>;

    fn call_with_config(
        &mut self,
        method: &str,
        params: Option<Value>,
        config: &DaemonClientConfig,
    ) -> Result<Value, ClientError>;

    fn call_with_retry(
        &mut self,
        method: &str,
        params: Option<Value>,
        max_retries: u32,
    ) -> Result<Value, ClientError>;

    fn call_stream(
        &mut self,
        _method: &str,
        _params: Option<Value>,
    ) -> Result<StreamResponse, ClientError> {
        Err(ClientError::UnexpectedResponse {
            message: "Streaming RPC not supported by this client".to_string(),
        })
    }
}

pub struct UnixSocketClient {
    transport: std::sync::Arc<dyn IpcTransport>,
}

fn is_retriable_error(error: &ClientError) -> bool {
    match error {
        ClientError::ConnectionFailed(io_err) => matches!(
            io_err.kind(),
            ErrorKind::ConnectionRefused | ErrorKind::WouldBlock | ErrorKind::TimedOut
        ),
        ClientError::RpcError { retryable, .. } => *retryable,
        _ => false,
    }
}

impl UnixSocketClient {
    pub fn connect() -> Result<Self, ClientError> {
        Self::connect_with_transport(default_transport())
    }

    pub fn connect_with_transport(
        transport: std::sync::Arc<dyn IpcTransport>,
    ) -> Result<Self, ClientError> {
        let stream = transport.connect_stream()?;
        drop(stream);

        Ok(Self { transport })
    }

    pub fn is_daemon_running() -> bool {
        default_transport().is_daemon_running()
    }
}

pub struct StreamAbortHandle {
    stream: ClientStream,
}

impl StreamAbortHandle {
    pub fn abort(&self) {
        let _ = self.stream.shutdown();
    }
}

pub struct StreamResponse {
    reader: BufReader<ClientStream>,
}

impl StreamResponse {
    pub fn next_result(&mut self) -> Result<Option<Value>, ClientError> {
        let mut response_line = String::new();
        let bytes = self.reader.read_line(&mut response_line)?;
        if bytes == 0 {
            return Ok(None);
        }
        let response: Response = serde_json::from_str(&response_line)?;
        response_to_result(response).map(Some)
    }

    pub fn abort_handle(&self) -> Option<StreamAbortHandle> {
        self.reader
            .get_ref()
            .try_clone()
            .ok()
            .map(|stream| StreamAbortHandle { stream })
    }
}

fn response_to_result(response: Response) -> Result<Value, ClientError> {
    if let Some(rpc_error) = response.error {
        let (category, retryable, context, suggestion) = if let Some(data) = rpc_error.data.as_ref()
        {
            let cat = data
                .get("category")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<error_codes::ErrorCategory>().ok());
            let retry = data
                .get("retryable")
                .and_then(|v| v.as_bool())
                .unwrap_or_else(|| error_codes::is_retryable(rpc_error.code));
            let ctx = data.get("context").cloned();
            let sug = data
                .get("suggestion")
                .and_then(|v| v.as_str())
                .map(String::from);
            (cat, retry, ctx, sug)
        } else {
            (
                Some(error_codes::category_for_code(rpc_error.code)),
                error_codes::is_retryable(rpc_error.code),
                None,
                None,
            )
        };

        return Err(ClientError::RpcError {
            code: rpc_error.code,
            message: rpc_error.message,
            category,
            retryable,
            context,
            suggestion,
        });
    }

    response.result.ok_or(ClientError::InvalidResponse)
}

impl DaemonClient for UnixSocketClient {
    fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value, ClientError> {
        self.call_with_config(method, params, &DaemonClientConfig::default())
    }

    fn call_with_config(
        &mut self,
        method: &str,
        params: Option<Value>,
        config: &DaemonClientConfig,
    ) -> Result<Value, ClientError> {
        let request_id = REQUEST_ID.fetch_add(1, Ordering::SeqCst);
        let start = Instant::now();
        debug!(
            request_id,
            method = %method,
            read_timeout_ms = config.read_timeout.as_millis(),
            write_timeout_ms = config.write_timeout.as_millis(),
            "RPC call started"
        );
        let mut stream = self.transport.connect_stream()?;

        stream.set_read_timeout(Some(config.read_timeout))?;
        stream.set_write_timeout(Some(config.write_timeout))?;

        let request = Request {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            method: method.to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)?;
        trace!(
            request_id,
            bytes = request_json.len(),
            "RPC request serialized"
        );

        writeln!(stream, "{}", request_json)?;
        stream.flush()?;

        let reader_stream = stream.try_clone()?;
        let mut reader = BufReader::new(reader_stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line)?;
        trace!(
            request_id,
            bytes = response_line.len(),
            "RPC response received"
        );

        let response: Response = serde_json::from_str(&response_line)?;
        let result = response_to_result(response);
        debug!(
            request_id,
            method = %method,
            elapsed_ms = start.elapsed().as_millis(),
            "RPC call finished"
        );
        result
    }

    fn call_with_retry(
        &mut self,
        method: &str,
        params: Option<Value>,
        max_retries: u32,
    ) -> Result<Value, ClientError> {
        let config = DaemonClientConfig::default().with_max_retries(max_retries);
        let mut delay = config.initial_retry_delay;
        let mut last_error = None;

        for attempt in 0..=config.max_retries {
            debug!(
                method = %method,
                attempt,
                max_retries = config.max_retries,
                "RPC retry attempt"
            );
            let params_clone = params.clone();
            match self.call_with_config(method, params_clone, &config) {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if !is_retriable_error(&e) || attempt == config.max_retries {
                        debug!(
                            method = %method,
                            attempt,
                            retryable = is_retriable_error(&e),
                            "RPC retry aborted"
                        );
                        return Err(e);
                    }
                    last_error = Some(e);
                    std::thread::sleep(delay);
                    delay *= 2;
                }
            }
        }

        Err(last_error.unwrap_or(ClientError::DaemonNotRunning))
    }

    fn call_stream(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<StreamResponse, ClientError> {
        let request_id = REQUEST_ID.fetch_add(1, Ordering::SeqCst);
        let mut stream = self.transport.connect_stream()?;
        let config = DaemonClientConfig::default();

        stream.set_read_timeout(Some(config.read_timeout))?;
        stream.set_write_timeout(Some(config.write_timeout))?;

        let request = Request {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            method: method.to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)?;
        writeln!(stream, "{}", request_json)?;
        stream.flush()?;

        let reader_stream = stream.try_clone()?;
        let mut reader = BufReader::new(reader_stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line)?;
        let response: Response = serde_json::from_str(&response_line)?;
        let _ = response_to_result(response)?;

        stream.set_read_timeout(None)?;

        Ok(StreamResponse { reader })
    }
}

pub fn ensure_daemon() -> Result<UnixSocketClient, ClientError> {
    ensure_daemon_with_transport(default_transport())
}

pub fn ensure_daemon_with_transport(
    transport: std::sync::Arc<dyn IpcTransport>,
) -> Result<UnixSocketClient, ClientError> {
    debug!("Ensuring daemon is running");
    if !transport.is_daemon_running() {
        debug!("Daemon not running");
        if transport.supports_autostart() {
            debug!("Attempting daemon autostart");
            eprintln!("{} Starting daemon in background...", Colors::dim("Note:"));
            transport.start_daemon_background()?;
        } else {
            return Err(ClientError::DaemonNotRunning);
        }
    }

    UnixSocketClient::connect_with_transport(transport)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PidLookupResult {
    Found(u32),
    NotRunning,
    Error(String),
}

pub fn get_daemon_pid() -> PidLookupResult {
    let lock_path = socket_path().with_extension("lock");
    if !lock_path.exists() {
        return PidLookupResult::NotRunning;
    }

    match std::fs::read_to_string(&lock_path) {
        Err(e) => PidLookupResult::Error(format!(
            "Failed to read lock file {}: {}",
            lock_path.display(),
            e
        )),
        Ok(content) => match content.trim().parse::<u32>() {
            Ok(pid) => PidLookupResult::Found(pid),
            Err(e) => PidLookupResult::Error(format!(
                "Lock file contains invalid PID '{}': {}",
                content.trim(),
                e
            )),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

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
        assert!(!json.contains("\"params\""));
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
    }

    #[test]
    fn test_response_deserializes_success_result() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"status":"ok"}}"#;
        let response: Response = serde_json::from_str(json).unwrap();
        assert!(response.result.is_some());
        assert!(response.error.is_none());
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
    }

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
            category: None,
            retryable: false,
            context: None,
            suggestion: None,
        };
        assert_eq!(err.to_string(), "RPC error (-32601): Method not found");
    }

    #[test]
    fn test_config_default_values() {
        let config = DaemonClientConfig::default();
        assert_eq!(config.read_timeout, Duration::from_secs(60));
        assert_eq!(config.write_timeout, Duration::from_secs(10));
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_retry_delay, Duration::from_millis(100));
    }

    #[test]
    fn test_config_builder_pattern() {
        let config = DaemonClientConfig::default()
            .with_read_timeout(Duration::from_secs(30))
            .with_write_timeout(Duration::from_secs(5))
            .with_max_retries(5);
        assert_eq!(config.read_timeout, Duration::from_secs(30));
        assert_eq!(config.write_timeout, Duration::from_secs(5));
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_is_retriable_error_connection_refused() {
        let io_err = std::io::Error::new(ErrorKind::ConnectionRefused, "connection refused");
        let err = ClientError::ConnectionFailed(io_err);
        assert!(is_retriable_error(&err));
    }

    #[test]
    fn test_is_retriable_error_would_block() {
        let io_err = std::io::Error::new(ErrorKind::WouldBlock, "would block");
        let err = ClientError::ConnectionFailed(io_err);
        assert!(is_retriable_error(&err));
    }

    #[test]
    fn test_is_retriable_error_timed_out() {
        let io_err = std::io::Error::new(ErrorKind::TimedOut, "timed out");
        let err = ClientError::ConnectionFailed(io_err);
        assert!(is_retriable_error(&err));
    }

    #[test]
    fn test_is_retriable_error_rpc_error_not_retriable() {
        let err = ClientError::RpcError {
            code: -32600,
            message: "Invalid request".to_string(),
            category: None,
            retryable: false,
            context: None,
            suggestion: None,
        };
        assert!(!is_retriable_error(&err));
    }

    #[test]
    fn test_is_retriable_error_rpc_lock_timeout() {
        let err = ClientError::RpcError {
            code: error_codes::LOCK_TIMEOUT,
            message: "Lock timeout".to_string(),
            category: Some(error_codes::ErrorCategory::Busy),
            retryable: true,
            context: None,
            suggestion: None,
        };
        assert!(is_retriable_error(&err));
    }

    #[test]
    fn test_is_retriable_error_daemon_not_running() {
        let err = ClientError::DaemonNotRunning;
        assert!(!is_retriable_error(&err));
    }

    static ENV_MUTEX: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();

    #[test]
    fn test_ensure_daemon_starts_when_not_running() {
        let _guard = ENV_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap();

        let temp_dir = tempfile::Builder::new()
            .prefix("agent-tui-test-")
            .tempdir_in("/tmp")
            .unwrap();
        let socket_path = temp_dir.path().join("agent-tui.sock");
        let _ = std::fs::remove_file(&socket_path);

        std::env::set_var("AGENT_TUI_SOCKET", &socket_path);
        crate::adapters::ipc::transport::USE_DAEMON_START_STUB
            .store(true, std::sync::atomic::Ordering::SeqCst);

        let result = ensure_daemon();
        match &result {
            Ok(_) => {
                assert!(UnixSocketClient::is_daemon_running());
            }
            Err(ClientError::ConnectionFailed(io_err))
                if io_err.kind() == ErrorKind::PermissionDenied =>
            {
                eprintln!(
                    "Skipping ensure_daemon test on restricted socket access: {}",
                    io_err
                );
            }
            Err(err) => {
                panic!(
                    "ensure_daemon failed for socket {}: {}",
                    socket_path.display(),
                    err
                );
            }
        }
        crate::adapters::ipc::transport::clear_test_listener();
        let _ = std::fs::remove_file(&socket_path);
        crate::adapters::ipc::transport::USE_DAEMON_START_STUB
            .store(false, std::sync::atomic::Ordering::SeqCst);
        std::env::remove_var("AGENT_TUI_SOCKET");
    }

    #[test]
    fn test_in_memory_transport_round_trip() {
        let transport = std::sync::Arc::new(
            crate::adapters::ipc::transport::InMemoryTransport::new(|request| {
                let value: serde_json::Value =
                    serde_json::from_str(request.trim()).expect("request json");
                let id = value
                    .get("id")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!(1));
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": { "ok": true }
                })
                .to_string()
            }),
        );

        let mut client = UnixSocketClient::connect_with_transport(transport).unwrap();
        let result = client.call("health", None).unwrap();
        assert_eq!(result["ok"], true);
    }
}
