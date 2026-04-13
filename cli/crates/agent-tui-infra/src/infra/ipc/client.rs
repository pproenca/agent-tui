#![expect(
    clippy::print_stderr,
    reason = "CLI status messages during daemon autostart"
)]

//! IPC client implementation.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tracing::debug;
use tracing::trace;

use crate::common::Colors;
use crate::common::error_codes;
use crate::infra::ipc::error::ClientError;
use crate::infra::ipc::process::ProcessIdentity;
use crate::infra::ipc::socket::socket_path;
use crate::infra::ipc::transport::ClientConnection;
use crate::infra::ipc::transport::IpcTransport;
use crate::infra::ipc::transport::UnixSocketTransport;
use crate::infra::ipc::transport::default_transport;

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);
const STREAM_POLL_TIMEOUT: Duration = Duration::from_millis(250);

#[derive(Debug, Clone)]
pub struct DaemonClientConfig {
    read_timeout: Duration,
    write_timeout: Duration,
    max_retries: u32,
    initial_retry_delay: Duration,
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
    pub fn read_timeout(&self) -> Duration {
        self.read_timeout
    }

    pub fn write_timeout(&self) -> Duration {
        self.write_timeout
    }

    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }

    pub fn initial_retry_delay(&self) -> Duration {
        self.initial_retry_delay
    }

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

    pub fn with_initial_retry_delay(mut self, delay: Duration) -> Self {
        self.initial_retry_delay = delay;
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
    #[serde(rename = "jsonrpc")]
    _jsonrpc: String,
    #[serde(rename = "id")]
    _id: u64,
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

impl UnixSocketClient {
    pub fn connect() -> Result<Self, ClientError> {
        Self::connect_with_transport(default_transport())
    }

    pub fn connect_local() -> Result<Self, ClientError> {
        Self::connect_with_transport(std::sync::Arc::new(UnixSocketTransport))
    }

    pub(crate) fn connect_with_transport(
        transport: std::sync::Arc<dyn IpcTransport>,
    ) -> Result<Self, ClientError> {
        let connection = transport.connect_connection()?;
        drop(connection);

        Ok(Self { transport })
    }

    pub fn is_daemon_running() -> bool {
        default_transport().is_daemon_running()
    }
}

pub struct StreamAbortHandle {
    aborted: Arc<AtomicBool>,
}

impl StreamAbortHandle {
    pub fn abort(&self) {
        self.aborted.store(true, Ordering::Relaxed);
    }
}

pub struct StreamResponse {
    connection: ClientConnection,
    aborted: Arc<AtomicBool>,
}

impl StreamResponse {
    pub fn next_result(&mut self) -> Result<Option<Value>, ClientError> {
        loop {
            if self.aborted.load(Ordering::Relaxed) {
                let _ = self.connection.shutdown();
                return Ok(None);
            }

            let response_line = match self.connection.read_message() {
                Ok(Some(line)) => line,
                Ok(None) => return Ok(None),
                Err(err) if is_timeout_error(&err) => continue,
                Err(err) => return Err(err),
            };

            let response: Response = serde_json::from_str(&response_line)?;
            return response_to_result(response).map(Some);
        }
    }

    pub fn abort_handle(&self) -> Option<StreamAbortHandle> {
        Some(StreamAbortHandle {
            aborted: Arc::clone(&self.aborted),
        })
    }
}

impl Drop for StreamResponse {
    fn drop(&mut self) {
        let _ = self.connection.shutdown();
    }
}

fn is_timeout_error(error: &ClientError) -> bool {
    match error {
        ClientError::ConnectionFailed(io_err) => matches!(
            io_err.kind(),
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
        ),
        _ => false,
    }
}

fn is_retryable_call_error(error: &ClientError) -> bool {
    match error {
        ClientError::ConnectionFailed(_) => true,
        ClientError::RpcError { retryable, .. } => *retryable,
        _ => false,
    }
}

fn retry_delay_ms_from_data(data: Option<&Value>) -> Option<u64> {
    let data = data?;
    ["retry_delay_ms", "retry_after_ms"]
        .into_iter()
        .find_map(|key| data.get(key).and_then(Value::as_u64))
}

fn next_retry_delay(
    error: &ClientError,
    default_delay: Duration,
    current_delay: Duration,
) -> Duration {
    error
        .retry_delay_ms()
        .map(Duration::from_millis)
        .unwrap_or_else(|| {
            if current_delay.is_zero() {
                default_delay
            } else {
                current_delay
            }
        })
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
                .and_then(serde_json::Value::as_bool)
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
        let retry_delay_ms = retry_delay_ms_from_data(rpc_error.data.as_ref());

        return Err(ClientError::RpcError {
            code: rpc_error.code,
            message: rpc_error.message,
            category,
            retryable,
            retry_delay_ms,
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
        let request = Request {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            method: method.to_string(),
            params,
        };
        let request_json = serde_json::to_string(&request)?;
        let start = Instant::now();
        let mut attempt = 0;
        let mut retry_delay = config.initial_retry_delay();

        loop {
            debug!(
                request_id,
                method = %method,
                attempt,
                read_timeout_ms = config.read_timeout().as_millis(),
                write_timeout_ms = config.write_timeout().as_millis(),
                "RPC call started"
            );

            let result = (|| {
                let mut connection = self.transport.connect_connection()?;
                connection.set_read_timeout(Some(config.read_timeout()))?;
                connection.set_write_timeout(Some(config.write_timeout()))?;

                trace!(
                    request_id,
                    attempt,
                    bytes = request_json.len(),
                    "RPC request serialized"
                );
                connection.send_message(&request_json)?;

                let response_line = connection
                    .read_message()?
                    .ok_or(ClientError::InvalidResponse)?;
                trace!(
                    request_id,
                    attempt,
                    bytes = response_line.len(),
                    "RPC response received"
                );

                let response: Response = serde_json::from_str(&response_line)?;
                response_to_result(response)
            })();

            match result {
                Ok(value) => {
                    debug!(
                        request_id,
                        method = %method,
                        attempt,
                        elapsed_ms = start.elapsed().as_millis(),
                        "RPC call finished"
                    );
                    return Ok(value);
                }
                Err(err) if attempt < config.max_retries() && is_retryable_call_error(&err) => {
                    let delay = next_retry_delay(&err, config.initial_retry_delay(), retry_delay);
                    debug!(
                        request_id,
                        method = %method,
                        attempt,
                        retry_in_ms = delay.as_millis(),
                        error = %err,
                        "RPC call failed; retrying"
                    );
                    std::thread::park_timeout(delay);
                    retry_delay = delay.saturating_mul(2);
                    attempt += 1;
                }
                Err(err) => {
                    debug!(
                        request_id,
                        method = %method,
                        attempt,
                        elapsed_ms = start.elapsed().as_millis(),
                        error = %err,
                        "RPC call finished with error"
                    );
                    return Err(err);
                }
            }
        }
    }

    fn call_stream(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<StreamResponse, ClientError> {
        let request_id = REQUEST_ID.fetch_add(1, Ordering::SeqCst);
        let mut connection = self.transport.connect_connection()?;
        let config = DaemonClientConfig::default();

        connection.set_read_timeout(Some(config.read_timeout()))?;
        connection.set_write_timeout(Some(config.write_timeout()))?;

        let request = Request {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            method: method.to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)?;
        connection.send_message(&request_json)?;
        let response_line = connection
            .read_message()?
            .ok_or(ClientError::InvalidResponse)?;
        let response: Response = serde_json::from_str(&response_line)?;
        let _ = response_to_result(response)?;

        connection.set_read_timeout(Some(STREAM_POLL_TIMEOUT))?;

        Ok(StreamResponse {
            connection,
            aborted: Arc::new(AtomicBool::new(false)),
        })
    }
}

pub fn ensure_daemon() -> Result<UnixSocketClient, ClientError> {
    ensure_daemon_with_transport(default_transport())
}

pub(crate) fn ensure_daemon_with_transport(
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonProcessLookupResult {
    Found(ProcessIdentity),
    NotRunning,
    InvalidState { path: PathBuf, message: String },
}

#[derive(Debug, Deserialize)]
struct DaemonLockFile {
    pid: u32,
    #[serde(default)]
    process_started_at: Option<u64>,
}

pub fn get_daemon_process_identity() -> DaemonProcessLookupResult {
    let lock_path = socket_path().with_extension("lock");
    if !lock_path.exists() {
        return DaemonProcessLookupResult::NotRunning;
    }

    let content = match std::fs::read_to_string(&lock_path) {
        Ok(content) => content,
        Err(err) => {
            return DaemonProcessLookupResult::InvalidState {
                path: lock_path,
                message: format!("failed to read lock file: {err}"),
            };
        }
    };

    parse_daemon_lock_file(&content).map_or_else(
        |message| DaemonProcessLookupResult::InvalidState {
            path: lock_path,
            message,
        },
        DaemonProcessLookupResult::Found,
    )
}

pub fn get_daemon_pid() -> PidLookupResult {
    match get_daemon_process_identity() {
        DaemonProcessLookupResult::Found(identity) => PidLookupResult::Found(identity.pid),
        DaemonProcessLookupResult::NotRunning => PidLookupResult::NotRunning,
        DaemonProcessLookupResult::InvalidState { path, message } => PidLookupResult::Error(
            format!("Invalid daemon lock state {}: {message}", path.display()),
        ),
    }
}

fn parse_daemon_lock_file(content: &str) -> Result<ProcessIdentity, String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("lock file is empty".to_string());
    }

    if let Ok(pid) = trimmed.parse::<u32>() {
        return Ok(ProcessIdentity {
            pid,
            started_at: None,
        });
    }

    let payload: DaemonLockFile = serde_json::from_str(trimmed)
        .map_err(|err| format!("lock file is not a valid daemon identity payload: {err}"))?;
    Ok(ProcessIdentity {
        pid: payload.pid,
        started_at: payload.process_started_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::mutex_lock_or_recover;
    use std::io::ErrorKind;
    use std::sync::Mutex;

    #[test]
    fn test_request_serializes_to_jsonrpc_2_0() {
        let request = Request {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "version".to_string(),
            params: None,
        };
        let json = serde_json::to_string(&request).expect("request should serialize");
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"version\""));
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
        let json = serde_json::to_string(&request).expect("request should serialize");
        assert!(json.contains("\"params\""));
        assert!(json.contains("\"command\":\"bash\""));
    }

    #[test]
    fn test_response_deserializes_success_result() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"status":"ok"}}"#;
        let response: Response = serde_json::from_str(json).expect("response should parse");
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_response_deserializes_error() {
        let json =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
        let response: Response = serde_json::from_str(json).expect("response should parse");
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        let error = response.error.expect("error payload should be present");
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
            retry_delay_ms: None,
            context: None,
            suggestion: None,
        };
        assert_eq!(err.to_string(), "RPC error (-32601): Method not found");
    }

    #[test]
    fn test_config_default_values() {
        let config = DaemonClientConfig::default();
        assert_eq!(config.read_timeout(), Duration::from_secs(60));
        assert_eq!(config.write_timeout(), Duration::from_secs(10));
        assert_eq!(config.max_retries(), 3);
        assert_eq!(config.initial_retry_delay(), Duration::from_millis(100));
    }

    #[test]
    fn test_config_builder_pattern() {
        let config = DaemonClientConfig::default()
            .with_read_timeout(Duration::from_secs(30))
            .with_write_timeout(Duration::from_secs(5))
            .with_max_retries(5)
            .with_initial_retry_delay(Duration::from_millis(25));
        assert_eq!(config.read_timeout(), Duration::from_secs(30));
        assert_eq!(config.write_timeout(), Duration::from_secs(5));
        assert_eq!(config.max_retries(), 5);
        assert_eq!(config.initial_retry_delay(), Duration::from_millis(25));
    }

    #[test]
    fn test_parse_daemon_lock_file_supports_legacy_pid_only_format() {
        let identity = parse_daemon_lock_file("1234").expect("legacy lock file should parse");
        assert_eq!(
            identity,
            ProcessIdentity {
                pid: 1234,
                started_at: None,
            }
        );
    }

    #[test]
    fn test_parse_daemon_lock_file_supports_identity_payload() {
        let identity = parse_daemon_lock_file(r#"{"pid":1234,"process_started_at":42}"#)
            .expect("identity lock file should parse");
        assert_eq!(
            identity,
            ProcessIdentity {
                pid: 1234,
                started_at: Some(42),
            }
        );
    }

    #[test]
    fn test_parse_daemon_lock_file_rejects_invalid_payload() {
        let err = parse_daemon_lock_file("not-a-pid").expect_err("invalid payload should fail");
        assert!(err.contains("not a valid daemon identity payload"));
    }

    static ENV_MUTEX: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();

    #[test]
    fn test_ensure_daemon_starts_when_not_running() {
        let _guard = mutex_lock_or_recover(ENV_MUTEX.get_or_init(|| Mutex::new(())));

        let temp_dir = tempfile::Builder::new()
            .prefix("agent-tui-test-")
            .tempdir_in("/tmp")
            .expect("temp dir should be created");
        let socket_path = temp_dir.path().join("agent-tui.sock");
        let _ = std::fs::remove_file(&socket_path);

        // SAFETY: Test-only environment override to isolate socket path.
        unsafe {
            std::env::set_var("AGENT_TUI_SOCKET", &socket_path);
        }
        crate::infra::ipc::transport::USE_DAEMON_START_STUB
            .store(true, std::sync::atomic::Ordering::SeqCst);

        let result = ensure_daemon();
        match &result {
            Ok(_) => {
                assert!(UnixSocketClient::is_daemon_running());
            }
            Err(ClientError::ConnectionFailed(io_err))
                if io_err.kind() == ErrorKind::PermissionDenied =>
            {
                eprintln!("Skipping ensure_daemon test on restricted socket access: {io_err}");
            }
            Err(err) => {
                panic!(
                    "ensure_daemon failed for socket {}: {}",
                    socket_path.display(),
                    err
                );
            }
        }
        crate::infra::ipc::transport::clear_test_listener();
        let _ = std::fs::remove_file(&socket_path);
        crate::infra::ipc::transport::USE_DAEMON_START_STUB
            .store(false, std::sync::atomic::Ordering::SeqCst);
        // SAFETY: Test-only cleanup of the environment override.
        unsafe {
            std::env::remove_var("AGENT_TUI_SOCKET");
        }
    }

    #[test]
    fn test_in_memory_transport_round_trip() {
        let transport = std::sync::Arc::new(crate::infra::ipc::transport::InMemoryTransport::new(
            |request| {
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
            },
        ));

        let mut client = UnixSocketClient::connect_with_transport(transport)
            .expect("transport-backed client should connect");
        let result = client
            .call("version", None)
            .expect("transport-backed call should succeed");
        assert_eq!(result["ok"], true);
    }

    #[test]
    fn test_call_with_config_retries_retryable_rpc_error() {
        let attempts = Arc::new(Mutex::new(0u32));
        let transport =
            std::sync::Arc::new(crate::infra::ipc::transport::InMemoryTransport::new({
                let attempts = Arc::clone(&attempts);
                move |request| {
                    let value: serde_json::Value =
                        serde_json::from_str(request.trim()).expect("request json");
                    let id = value
                        .get("id")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!(1));
                    let mut count = mutex_lock_or_recover(&attempts);
                    *count += 1;
                    if *count == 1 {
                        serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {
                                "code": -32000,
                                "message": "busy",
                                "data": {
                                    "retryable": true,
                                    "retry_delay_ms": 0
                                }
                            }
                        })
                        .to_string()
                    } else {
                        serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": { "ok": true }
                        })
                        .to_string()
                    }
                }
            }));

        let mut client = UnixSocketClient::connect_with_transport(transport)
            .expect("transport-backed client should connect");
        let config = DaemonClientConfig::default()
            .with_max_retries(1)
            .with_initial_retry_delay(Duration::ZERO);

        let result = client
            .call_with_config("version", None, &config)
            .expect("retryable RPC error should be retried");

        assert_eq!(result["ok"], true);
        assert_eq!(*mutex_lock_or_recover(&attempts), 2);
    }

    #[test]
    fn test_call_with_config_does_not_retry_non_retryable_rpc_error() {
        let attempts = Arc::new(Mutex::new(0u32));
        let transport =
            std::sync::Arc::new(crate::infra::ipc::transport::InMemoryTransport::new({
                let attempts = Arc::clone(&attempts);
                move |request| {
                    let value: serde_json::Value =
                        serde_json::from_str(request.trim()).expect("request json");
                    let id = value
                        .get("id")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!(1));
                    let mut count = mutex_lock_or_recover(&attempts);
                    *count += 1;
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32001,
                            "message": "fatal",
                            "data": {
                                "retryable": false
                            }
                        }
                    })
                    .to_string()
                }
            }));

        let mut client = UnixSocketClient::connect_with_transport(transport)
            .expect("transport-backed client should connect");
        let config = DaemonClientConfig::default()
            .with_max_retries(3)
            .with_initial_retry_delay(Duration::ZERO);

        let err = client
            .call_with_config("version", None, &config)
            .expect_err("non-retryable RPC error should be returned");

        assert!(matches!(
            err,
            ClientError::RpcError {
                message,
                retry_delay_ms: None,
                ..
            } if message == "fatal"
        ));
        assert_eq!(*mutex_lock_or_recover(&attempts), 1);
    }

    #[test]
    fn test_client_error_to_json_includes_retry_delay() {
        let err = ClientError::RpcError {
            code: -32000,
            message: "busy".to_string(),
            category: None,
            retryable: true,
            retry_delay_ms: Some(250),
            context: None,
            suggestion: None,
        };

        let json = err.to_json();
        assert_eq!(json["retry_delay_ms"], 250);
    }

    #[test]
    fn test_unix_socket_client_round_trip_over_real_socket() {
        use std::io::BufRead;
        use std::io::BufReader;
        use std::io::Write;
        use std::os::unix::net::UnixListener;

        let _guard = mutex_lock_or_recover(ENV_MUTEX.get_or_init(|| Mutex::new(())));
        let temp_dir = tempfile::Builder::new()
            .prefix("agent-tui-ipc-")
            .tempdir_in("/tmp")
            .expect("temp dir should be created");
        let socket_path = temp_dir.path().join("daemon.sock");

        let listener = UnixListener::bind(&socket_path).expect("listener should bind");
        // SAFETY: Test-only environment override to isolate the Unix socket path.
        unsafe {
            std::env::set_var("AGENT_TUI_SOCKET", &socket_path);
        }

        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("server should accept");
            let reader_stream = stream.try_clone().expect("server stream should clone");
            let mut reader = BufReader::new(reader_stream);
            let mut writer = stream;

            let mut line = String::new();
            reader.read_line(&mut line).expect("request should read");
            let request: serde_json::Value =
                serde_json::from_str(line.trim()).expect("request JSON should parse");
            let id = request
                .get("id")
                .cloned()
                .expect("request should include id");
            assert_eq!(request["method"], "version");

            let response = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "ok": true }
            });
            writeln!(writer, "{response}").expect("response should write");
            writer.flush().expect("response should flush");
        });

        let mut client = UnixSocketClient {
            transport: std::sync::Arc::new(crate::infra::ipc::transport::UnixSocketTransport),
        };
        let result = client
            .call("version", None)
            .expect("real socket call should succeed");

        assert_eq!(result["ok"], true);
        server.join().expect("server thread should join");

        // SAFETY: Test-only cleanup of the Unix socket override.
        unsafe {
            std::env::remove_var("AGENT_TUI_SOCKET");
        }
    }
}
