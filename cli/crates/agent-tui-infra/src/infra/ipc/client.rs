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

use crate::common::RpcId;
use crate::common::color;
use crate::common::error_codes;
use crate::infra::ipc::error::ClientError;
use crate::infra::ipc::process::ProcessIdentity;
use crate::infra::ipc::socket::socket_path;
use crate::infra::ipc::transport::ClientConnection;
use crate::infra::ipc::transport::IpcTransport;

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);
const STREAM_POLL_TIMEOUT: Duration = Duration::from_millis(250);

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

#[derive(Debug, Serialize)]
struct Request {
    jsonrpc: String,
    id: RpcId,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct Response {
    #[serde(rename = "jsonrpc")]
    _jsonrpc: String,
    #[serde(rename = "id")]
    id: Option<RpcId>,
    #[serde(default, deserialize_with = "deserialize_response_result")]
    result: ResponseResult,
    error: Option<RpcError>,
}

#[derive(Debug, Default)]
enum ResponseResult {
    #[default]
    Missing,
    Present(Value),
}

fn deserialize_response_result<'de, D>(deserializer: D) -> Result<ResponseResult, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Value::deserialize(deserializer).map(ResponseResult::Present)
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
        method: &str,
        params: Option<Value>,
    ) -> Result<StreamResponse, ClientError> {
        self.call_stream_with_config(method, params, &DaemonClientConfig::default())
    }

    fn call_stream_with_config(
        &mut self,
        _method: &str,
        _params: Option<Value>,
        _config: &DaemonClientConfig,
    ) -> Result<StreamResponse, ClientError> {
        Err(ClientError::UnexpectedResponse {
            message: "Streaming RPC not supported by this client".to_string(),
        })
    }
}

pub struct UnixSocketClient {
    transport: IpcTransport,
}

impl UnixSocketClient {
    pub fn connect() -> Result<Self, ClientError> {
        Self::connect_with_transport(IpcTransport::from_env())
    }

    pub fn connect_local() -> Result<Self, ClientError> {
        Self::connect_with_transport(IpcTransport::Unix)
    }

    pub(crate) fn connect_with_transport(transport: IpcTransport) -> Result<Self, ClientError> {
        let connection = transport.connect_connection()?;
        drop(connection);

        Ok(Self { transport })
    }

    pub fn is_daemon_running() -> bool {
        IpcTransport::from_env().is_daemon_running()
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
    expected_id: RpcId,
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
            return response_to_result(response, &self.expected_id).map(Some);
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

fn validate_response_id(response: &Response, expected_id: &RpcId) -> Result<(), ClientError> {
    match &response.id {
        Some(actual_id) if actual_id == expected_id => Ok(()),
        Some(_) => Err(ClientError::InvalidResponse),
        None if response.error.is_some() => Ok(()),
        None => Err(ClientError::InvalidResponse),
    }
}

fn response_to_result(response: Response, expected_id: &RpcId) -> Result<Value, ClientError> {
    validate_response_id(&response, expected_id)?;

    match (&response.result, &response.error) {
        (ResponseResult::Present(_), None) | (ResponseResult::Missing, Some(_)) => {}
        _ => return Err(ClientError::InvalidResponse),
    }

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

    match response.result {
        ResponseResult::Present(result) => Ok(result),
        ResponseResult::Missing => Err(ClientError::InvalidResponse),
    }
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
        let rpc_id = RpcId::from(request_id);
        let request = Request {
            jsonrpc: "2.0".to_string(),
            id: rpc_id.clone(),
            method: method.to_string(),
            params,
        };
        let request_json = serde_json::to_string(&request)?;
        let start = Instant::now();
        let mut attempt = 0;
        let mut retry_delay = config.initial_retry_delay;

        loop {
            debug!(
                request_id,
                method = %method,
                attempt,
                read_timeout_ms = config.read_timeout.as_millis(),
                write_timeout_ms = config.write_timeout.as_millis(),
                "RPC call started"
            );

            let result = (|| {
                let mut connection = self.transport.connect_connection()?;
                connection.set_read_timeout(Some(config.read_timeout))?;
                connection.set_write_timeout(Some(config.write_timeout))?;

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
                response_to_result(response, &rpc_id)
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
                Err(err) if attempt < config.max_retries && is_retryable_call_error(&err) => {
                    let delay = next_retry_delay(&err, config.initial_retry_delay, retry_delay);
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
        self.call_stream_with_config(method, params, &DaemonClientConfig::default())
    }

    fn call_stream_with_config(
        &mut self,
        method: &str,
        params: Option<Value>,
        config: &DaemonClientConfig,
    ) -> Result<StreamResponse, ClientError> {
        let request_id = REQUEST_ID.fetch_add(1, Ordering::SeqCst);
        let rpc_id = RpcId::from(request_id);
        let request = Request {
            jsonrpc: "2.0".to_string(),
            id: rpc_id.clone(),
            method: method.to_string(),
            params,
        };
        let request_json = serde_json::to_string(&request)?;
        let start = Instant::now();
        let mut attempt = 0;
        let mut retry_delay = config.initial_retry_delay;

        loop {
            debug!(
                request_id,
                method = %method,
                attempt,
                read_timeout_ms = config.read_timeout.as_millis(),
                write_timeout_ms = config.write_timeout.as_millis(),
                "RPC stream call started"
            );

            let result = (|| {
                let mut connection = self.transport.connect_connection()?;
                connection.set_read_timeout(Some(config.read_timeout))?;
                connection.set_write_timeout(Some(config.write_timeout))?;

                trace!(
                    request_id,
                    attempt,
                    bytes = request_json.len(),
                    "RPC stream request serialized"
                );
                connection.send_message(&request_json)?;
                let response_line = connection
                    .read_message()?
                    .ok_or(ClientError::InvalidResponse)?;
                trace!(
                    request_id,
                    attempt,
                    bytes = response_line.len(),
                    "RPC stream handshake received"
                );
                let response: Response = serde_json::from_str(&response_line)?;
                let _ = response_to_result(response, &rpc_id)?;

                connection.set_read_timeout(Some(STREAM_POLL_TIMEOUT))?;

                Ok(StreamResponse {
                    connection,
                    expected_id: rpc_id.clone(),
                    aborted: Arc::new(AtomicBool::new(false)),
                })
            })();

            match result {
                Ok(stream) => {
                    debug!(
                        request_id,
                        method = %method,
                        attempt,
                        elapsed_ms = start.elapsed().as_millis(),
                        "RPC stream call finished"
                    );
                    return Ok(stream);
                }
                Err(err) if attempt < config.max_retries && is_retryable_call_error(&err) => {
                    let delay = next_retry_delay(&err, config.initial_retry_delay, retry_delay);
                    debug!(
                        request_id,
                        method = %method,
                        attempt,
                        retry_in_ms = delay.as_millis(),
                        error = %err,
                        "RPC stream call failed; retrying"
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
                        "RPC stream call finished with error"
                    );
                    return Err(err);
                }
            }
        }
    }
}

pub fn ensure_daemon() -> Result<UnixSocketClient, ClientError> {
    ensure_daemon_with_transport(IpcTransport::from_env())
}

pub(crate) fn ensure_daemon_with_transport(
    transport: IpcTransport,
) -> Result<UnixSocketClient, ClientError> {
    debug!("Ensuring daemon is running");
    if !transport.is_daemon_running() {
        debug!("Daemon not running");
        if matches!(&transport, IpcTransport::Unix) {
            debug!("Attempting daemon autostart");
            eprintln!("{} Starting daemon in background...", color::dim("Note:"));
            crate::infra::ipc::transport::start_daemon_background()?;
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
#[path = "client_tests.rs"]
mod tests;
