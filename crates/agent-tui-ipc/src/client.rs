use std::io::BufRead;
use std::io::BufReader;
use std::io::ErrorKind;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::error::ClientError;
use crate::error_codes;
use crate::socket::socket_path;

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// Polling configuration for daemon startup/shutdown.
pub mod polling {
    use std::time::Duration;

    /// Maximum number of polls during daemon startup.
    pub const MAX_STARTUP_POLLS: u32 = 50;
    /// Initial delay between polls.
    pub const INITIAL_POLL_INTERVAL: Duration = Duration::from_millis(50);
    /// Maximum delay between polls (after exponential backoff).
    pub const MAX_POLL_INTERVAL: Duration = Duration::from_millis(500);
    /// Timeout for daemon shutdown.
    pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);
}

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

/// Trait for daemon client implementations.
///
/// This trait abstracts the communication with the daemon, allowing for
/// different transport implementations (Unix socket, mock for testing, etc.).
pub trait DaemonClient: Send + Sync {
    /// Make an RPC call to the daemon.
    fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value, ClientError>;

    /// Make an RPC call with custom configuration.
    fn call_with_config(
        &mut self,
        method: &str,
        params: Option<Value>,
        config: &DaemonClientConfig,
    ) -> Result<Value, ClientError>;

    /// Make an RPC call with retry logic.
    fn call_with_retry(
        &mut self,
        method: &str,
        params: Option<Value>,
        max_retries: u32,
    ) -> Result<Value, ClientError>;
}

/// Unix socket-based daemon client implementation.
pub struct UnixSocketClient;

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
        let path = socket_path();
        let mut stream = UnixStream::connect(&path)?;

        stream.set_read_timeout(Some(config.read_timeout))?;
        stream.set_write_timeout(Some(config.write_timeout))?;

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
            let (category, retryable, context, suggestion) = if let Some(data) = error.data.as_ref()
            {
                let cat = data
                    .get("category")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<error_codes::ErrorCategory>().ok());
                let retry = data
                    .get("retryable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or_else(|| error_codes::is_retryable(error.code));
                let ctx = data.get("context").cloned();
                let sug = data
                    .get("suggestion")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                (cat, retry, ctx, sug)
            } else {
                (
                    Some(error_codes::category_for_code(error.code)),
                    error_codes::is_retryable(error.code),
                    None,
                    None,
                )
            };

            return Err(ClientError::RpcError {
                code: error.code,
                message: error.message,
                category,
                retryable,
                context,
                suggestion,
            });
        }

        response.result.ok_or(ClientError::InvalidResponse)
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
            let params_clone = params.clone();
            match self.call_with_config(method, params_clone, &config) {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if !is_retriable_error(&e) || attempt == config.max_retries {
                        return Err(e);
                    }
                    last_error = Some(e);
                    std::thread::sleep(delay);
                    delay *= 2; // exponential backoff: 100ms, 200ms, 400ms
                }
            }
        }

        Err(last_error.unwrap_or(ClientError::DaemonNotRunning))
    }
}

pub fn start_daemon_background() -> Result<(), ClientError> {
    use std::fs::OpenOptions;
    use std::process::Command;
    use std::process::Stdio;

    let exe = std::env::current_exe()?;
    let log_path = socket_path().with_extension("log");

    let log_file = match OpenOptions::new().create(true).append(true).open(&log_path) {
        Ok(f) => Some(f),
        Err(e) => {
            eprintln!(
                "Warning: Could not open daemon log file {}: {}",
                log_path.display(),
                e
            );
            None
        }
    };

    let stderr = match log_file {
        Some(f) => Stdio::from(f),
        None => Stdio::null(),
    };

    Command::new(exe)
        .args(["daemon", "start", "--foreground"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(stderr)
        .spawn()?;

    let mut delay = polling::INITIAL_POLL_INTERVAL;
    for i in 0..polling::MAX_STARTUP_POLLS {
        std::thread::sleep(delay);
        if UnixSocketClient::is_daemon_running() {
            return Ok(());
        }
        // Exponential backoff with cap
        delay = (delay * 2).min(polling::MAX_POLL_INTERVAL);

        if i == polling::MAX_STARTUP_POLLS - 1 {
            if let Ok(log_content) = std::fs::read_to_string(&log_path) {
                let last_lines: String = log_content
                    .lines()
                    .rev()
                    .take(5)
                    .collect::<Vec<_>>()
                    .join("\n");
                if !last_lines.is_empty() {
                    eprintln!("Daemon failed to start. Recent log output:\n{}", last_lines);
                }
            }
        }
    }

    Err(ClientError::DaemonNotRunning)
}

pub fn ensure_daemon() -> Result<UnixSocketClient, ClientError> {
    if !UnixSocketClient::is_daemon_running() {
        start_daemon_background()?;
    }

    UnixSocketClient::connect()
}

/// Result of PID lookup from lock file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PidLookupResult {
    /// Daemon is running with this PID.
    Found(u32),
    /// No lock file exists (daemon not running).
    NotRunning,
    /// Lock file exists but could not be read or parsed.
    Error(String),
}

/// Get the daemon PID from the lock file.
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
}
