use crate::protocol::{Request, Response};
use interprocess::local_socket::{
    tokio::{prelude::*, Stream as UnixStream},
    GenericFilePath, ToFsName,
};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::time::timeout;
use rand::Rng;

// ============================================================================
// Lock Poison Recovery Helpers
// ============================================================================

/// Recover from a poisoned Mutex, logging a warning
fn mutex_lock_or_recover<T>(lock: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(|poisoned| {
        eprintln!("Warning: recovering from poisoned mutex");
        poisoned.into_inner()
    })
}

/// Transport type for daemon connection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransportType {
    Unix,
    Tcp,
}

impl TransportType {
    pub fn from_env() -> Self {
        match std::env::var("AGENT_TUI_TRANSPORT").as_deref() {
            Ok("tcp") => TransportType::Tcp,
            _ => TransportType::Unix,
        }
    }
}

/// Default TCP port for daemon
const DEFAULT_TCP_PORT: u16 = 19847;

/// Get TCP port from environment or use default
pub fn get_tcp_port() -> u16 {
    std::env::var("AGENT_TUI_TCP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_TCP_PORT)
}

/// Get TCP address for daemon connection
pub fn get_tcp_address() -> String {
    let port = get_tcp_port();
    format!("127.0.0.1:{}", port)
}

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

// Retry configuration
const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 500;

// Default request timeout (30 seconds)
const DEFAULT_TIMEOUT_MS: u64 = 30000;

// Circuit breaker configuration
const CIRCUIT_FAILURE_THRESHOLD: u32 = 5;
const CIRCUIT_RESET_TIMEOUT_MS: u64 = 30000;
const CIRCUIT_HALF_OPEN_REQUESTS: u32 = 3;

// Daemon restart limits
const MAX_DAEMON_RESTARTS: u32 = 3;
const DAEMON_RESTART_WINDOW_MS: u64 = 60000;

// Jitter for exponential backoff (0-30% of delay)
const JITTER_PERCENT: f64 = 0.30;

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Internal state for circuit breaker - all fields protected by single mutex
/// to prevent TOCTOU races between state reads and writes
struct CircuitBreakerInner {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
    half_open_requests: u32,
}

/// Circuit breaker for preventing cascading failures
/// Uses a single mutex to ensure atomic state transitions
pub struct CircuitBreaker {
    inner: Mutex<CircuitBreakerInner>,
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(CircuitBreakerInner {
                state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure_time: None,
                half_open_requests: 0,
            }),
        }
    }

    /// Check if the circuit allows requests
    /// All state checks and transitions happen atomically under single lock
    pub fn can_execute(&self) -> bool {
        let mut inner = mutex_lock_or_recover(&self.inner);
        match inner.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if we should transition to half-open
                if let Some(last_failure) = inner.last_failure_time {
                    if last_failure.elapsed() > Duration::from_millis(CIRCUIT_RESET_TIMEOUT_MS) {
                        inner.state = CircuitState::HalfOpen;
                        inner.half_open_requests = 0;
                        inner.success_count = 0;
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => {
                // Allow limited requests in half-open state
                if inner.half_open_requests < CIRCUIT_HALF_OPEN_REQUESTS {
                    inner.half_open_requests += 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Record a successful request
    /// Atomically updates counts and potentially transitions state
    pub fn record_success(&self) {
        let mut inner = mutex_lock_or_recover(&self.inner);
        match inner.state {
            CircuitState::HalfOpen => {
                inner.success_count += 1;
                if inner.success_count >= CIRCUIT_HALF_OPEN_REQUESTS {
                    // Transition back to closed
                    inner.state = CircuitState::Closed;
                    inner.failure_count = 0;
                    inner.success_count = 0;
                    inner.half_open_requests = 0;
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success
                inner.failure_count = 0;
            }
            CircuitState::Open => {}
        }
    }

    /// Record a failed request
    /// Atomically updates counts and potentially transitions state
    pub fn record_failure(&self) {
        let mut inner = mutex_lock_or_recover(&self.inner);
        inner.failure_count += 1;
        inner.last_failure_time = Some(Instant::now());

        match inner.state {
            CircuitState::Closed => {
                if inner.failure_count >= CIRCUIT_FAILURE_THRESHOLD {
                    inner.state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open goes back to open
                inner.state = CircuitState::Open;
                inner.success_count = 0;
                inner.half_open_requests = 0;
            }
            CircuitState::Open => {}
        }
    }

    /// Get current circuit state
    pub fn get_state(&self) -> CircuitState {
        mutex_lock_or_recover(&self.inner).state
    }

    /// Reset the circuit breaker
    pub fn reset(&self) {
        let mut inner = mutex_lock_or_recover(&self.inner);
        inner.state = CircuitState::Closed;
        inner.failure_count = 0;
        inner.success_count = 0;
        inner.half_open_requests = 0;
        inner.last_failure_time = None;
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal state for restart tracker - single mutex to prevent TOCTOU
struct DaemonRestartTrackerInner {
    restart_count: u32,
    window_start: Instant,
}

/// Daemon restart tracker
/// Uses single mutex for atomic check-and-update operations
struct DaemonRestartTracker {
    inner: Mutex<DaemonRestartTrackerInner>,
}

impl DaemonRestartTracker {
    fn new() -> Self {
        Self {
            inner: Mutex::new(DaemonRestartTrackerInner {
                restart_count: 0,
                window_start: Instant::now(),
            }),
        }
    }

    /// Check if we can restart the daemon
    /// Atomically checks and potentially resets the window
    fn can_restart(&self) -> bool {
        let mut inner = mutex_lock_or_recover(&self.inner);
        if inner.window_start.elapsed() > Duration::from_millis(DAEMON_RESTART_WINDOW_MS) {
            // Reset window atomically
            inner.window_start = Instant::now();
            inner.restart_count = 0;
            return true;
        }
        inner.restart_count < MAX_DAEMON_RESTARTS
    }

    /// Record a restart attempt
    fn record_restart(&self) {
        let mut inner = mutex_lock_or_recover(&self.inner);
        inner.restart_count += 1;
    }
}

// Global instances
lazy_static::lazy_static! {
    static ref CIRCUIT_BREAKER: CircuitBreaker = CircuitBreaker::new();
    static ref RESTART_TRACKER: DaemonRestartTracker = DaemonRestartTracker::new();
}

/// Calculate delay with jitter for exponential backoff
fn delay_with_jitter(base_delay_ms: u64, attempt: u32) -> u64 {
    let exponential_delay = base_delay_ms * (1 << attempt.min(4));
    let jitter_range = (exponential_delay as f64 * JITTER_PERCENT) as u64;
    let jitter = if jitter_range > 0 {
        rand::thread_rng().gen_range(0..jitter_range)
    } else {
        0
    };
    exponential_delay + jitter
}

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("Failed to connect to daemon: {0}")]
    ConnectionFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Daemon error: {code} - {message}")]
    DaemonError { code: i32, message: String },
    #[error("Timeout waiting for daemon to start")]
    DaemonStartTimeout,
    #[error("Request timed out after {0}ms")]
    RequestTimeout(u64),
    #[error("All {0} retry attempts failed")]
    RetryExhausted(u32),
    #[error("Circuit breaker open: daemon is unresponsive (will retry in {0}s)")]
    CircuitOpen(u64),
    #[error("Daemon restart limit exceeded ({0} restarts in the last minute)")]
    RestartLimitExceeded(u32),
}

pub fn get_socket_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .or_else(|_| std::env::var("TMPDIR"))
        .unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(runtime_dir).join("agent-tui.sock")
}

pub fn get_pid_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .or_else(|_| std::env::var("TMPDIR"))
        .unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(runtime_dir).join("agent-tui.pid")
}

pub fn get_daemon_path() -> PathBuf {
    // The native Rust daemon is embedded - just return our own binary path
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("agent-tui"))
}

/// Check if daemon process is alive by reading PID file and verifying process exists
fn is_process_alive(pid: i32) -> bool {
    #[cfg(unix)]
    {
        // kill with signal 0 checks if process exists without sending a signal
        unsafe { libc::kill(pid, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        // On non-Unix platforms, just assume process exists if we got a PID
        true
    }
}

/// Clean up stale socket and PID files if they belong to the expected (dead) PID.
/// Re-validates PID before cleanup to prevent TOCTOU race where a new daemon
/// starts between our check and cleanup.
fn cleanup_stale_files_for_pid(expected_pid: Option<i32>) {
    let socket_path = get_socket_path();
    let pid_path = get_pid_path();

    // Re-read PID file to verify it still contains the dead PID
    // This prevents race condition where new daemon started between check and cleanup
    if let Some(expected) = expected_pid {
        if let Ok(current_pid_str) = std::fs::read_to_string(&pid_path) {
            if let Ok(current_pid) = current_pid_str.trim().parse::<i32>() {
                if current_pid != expected {
                    // PID changed - new daemon started, don't cleanup
                    return;
                }
                // Double-check the process is still dead before removing
                if is_process_alive(current_pid) {
                    return;
                }
            }
        }
    }

    // Safe to cleanup - either no expected PID or PID confirmed dead
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }
    if pid_path.exists() {
        let _ = std::fs::remove_file(&pid_path);
    }
}

/// Legacy cleanup function for cases where we don't have a PID
fn cleanup_stale_files() {
    cleanup_stale_files_for_pid(None);
}

pub fn is_daemon_running() -> bool {
    let socket_path = get_socket_path();
    let pid_path = get_pid_path();

    // Check if PID file exists
    if !pid_path.exists() {
        // No PID file - if socket exists, it's stale (no PID to validate against)
        if socket_path.exists() {
            cleanup_stale_files();
        }
        return false;
    }

    // Read and verify PID
    match std::fs::read_to_string(&pid_path) {
        Ok(pid_str) => {
            match pid_str.trim().parse::<i32>() {
                Ok(pid) => {
                    if is_process_alive(pid) {
                        // Process is alive, verify socket exists
                        if socket_path.exists() {
                            return true;
                        }
                        // Socket missing but process alive - unusual state
                        // Let the daemon recreate it
                        return false;
                    }
                    // Process is dead, cleanup stale files with PID validation
                    cleanup_stale_files_for_pid(Some(pid));
                    false
                }
                Err(_) => {
                    // Invalid PID file, cleanup (no valid PID to check)
                    cleanup_stale_files();
                    false
                }
            }
        }
        Err(_) => {
            // Can't read PID file, cleanup (no PID to validate against)
            cleanup_stale_files();
            false
        }
    }
}

/// Verify daemon is actually responsive by sending a quick ping
pub async fn verify_daemon_responsive() -> bool {
    let socket_path = get_socket_path();
    if !socket_path.exists() {
        return false;
    }

    // Try a quick ping with short timeout
    send_request_once(crate::protocol::METHOD_PING, None, 2000).await.is_ok()
}

pub fn start_daemon() -> Result<(), ConnectionError> {
    let daemon_path = get_daemon_path();

    // Start the native Rust daemon
    let (program, args): (&str, Vec<&str>) = (daemon_path.to_str().unwrap(), vec!["daemon"]);

    // Start daemon in background
    let mut cmd = Command::new(program);
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    // On Unix, we can properly daemonize
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                // Create new session to detach from terminal
                libc::setsid();
                Ok(())
            });
        }
    }

    cmd.spawn()
        .map_err(|e| ConnectionError::ConnectionFailed(format!("Failed to start daemon: {}", e)))?;

    // Wait for socket to be created
    let socket_path = get_socket_path();
    for _ in 0..50 {
        if socket_path.exists() {
            std::thread::sleep(std::time::Duration::from_millis(100));
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    Err(ConnectionError::DaemonStartTimeout)
}

pub async fn ensure_daemon_running() -> Result<(), ConnectionError> {
    if !is_daemon_running() {
        // Check restart limits before attempting to start
        if !RESTART_TRACKER.can_restart() {
            return Err(ConnectionError::RestartLimitExceeded(MAX_DAEMON_RESTARTS));
        }

        eprintln!("Starting daemon...");
        start_daemon()?;
        RESTART_TRACKER.record_restart();

        // Reset circuit breaker on fresh start
        CIRCUIT_BREAKER.reset();
    }
    Ok(())
}

/// Attempt to recover daemon by restarting if it's unresponsive
pub async fn try_recover_daemon() -> Result<(), ConnectionError> {
    // First check if process exists but is unresponsive
    if is_daemon_running() {
        // Process exists but might be stuck - try to verify responsiveness
        if !verify_daemon_responsive().await {
            eprintln!("Daemon is unresponsive, attempting recovery...");

            // Kill the existing process
            if let Ok(pid_str) = std::fs::read_to_string(get_pid_path()) {
                if let Ok(pid) = pid_str.trim().parse::<i32>() {
                    #[cfg(unix)]
                    unsafe {
                        libc::kill(pid, libc::SIGKILL);
                    }
                    // Give it time to cleanup
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }

            // Cleanup stale files
            cleanup_stale_files();
        } else {
            // Daemon is actually responsive, nothing to do
            return Ok(());
        }
    }

    // Now try to start fresh
    ensure_daemon_running().await
}

/// Helper to perform the actual request over a generic stream
async fn perform_request<S: AsyncRead + AsyncWrite + Unpin>(
    stream: S,
    method: &str,
    params: Option<serde_json::Value>,
    timeout_ms: u64,
) -> Result<serde_json::Value, ConnectionError> {
    let id = REQUEST_ID.fetch_add(1, Ordering::SeqCst);
    let request = Request::new(id, method, params);
    let request_json = serde_json::to_string(&request)?;

    let (reader, mut writer) = tokio::io::split(stream);

    // Send request with newline delimiter
    writer.write_all(request_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    // Read response with timeout
    let mut buf_reader = BufReader::new(reader);
    let mut response_line = String::new();

    let read_result = timeout(
        Duration::from_millis(timeout_ms),
        buf_reader.read_line(&mut response_line)
    ).await;

    match read_result {
        Ok(Ok(_)) => {},
        Ok(Err(e)) => return Err(ConnectionError::Io(e)),
        Err(_) => return Err(ConnectionError::RequestTimeout(timeout_ms)),
    }

    let response: Response = serde_json::from_str(&response_line)?;

    if let Some(error) = response.error {
        return Err(ConnectionError::DaemonError {
            code: error.code,
            message: error.message,
        });
    }

    Ok(response.result.unwrap_or(serde_json::Value::Null))
}

/// Connect via Unix socket
async fn connect_unix() -> Result<UnixStream, ConnectionError> {
    let socket_path = get_socket_path();
    let socket_name = socket_path.to_fs_name::<GenericFilePath>()
        .map_err(|e| ConnectionError::ConnectionFailed(e.to_string()))?;

    UnixStream::connect(socket_name)
        .await
        .map_err(|e| ConnectionError::ConnectionFailed(e.to_string()))
}

/// Connect via TCP
async fn connect_tcp() -> Result<TcpStream, ConnectionError> {
    let address = get_tcp_address();
    TcpStream::connect(&address)
        .await
        .map_err(|e| ConnectionError::ConnectionFailed(format!("TCP connection to {} failed: {}", address, e)))
}

/// Send a request to the daemon (internal, single attempt)
async fn send_request_once(
    method: &str,
    params: Option<serde_json::Value>,
    timeout_ms: u64,
) -> Result<serde_json::Value, ConnectionError> {
    let transport = TransportType::from_env();

    match transport {
        TransportType::Unix => {
            let stream = connect_unix().await?;
            perform_request(stream, method, params, timeout_ms).await
        }
        TransportType::Tcp => {
            let stream = connect_tcp().await?;
            perform_request(stream, method, params, timeout_ms).await
        }
    }
}

/// Get current transport type
pub fn get_transport_type() -> TransportType {
    TransportType::from_env()
}

/// Send a request to the daemon with retry logic
pub async fn send_request(
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, ConnectionError> {
    send_request_with_timeout(method, params, DEFAULT_TIMEOUT_MS).await
}

/// Send a request to the daemon with custom timeout
pub async fn send_request_with_timeout(
    method: &str,
    params: Option<serde_json::Value>,
    timeout_ms: u64,
) -> Result<serde_json::Value, ConnectionError> {
    // Check circuit breaker before attempting
    if !CIRCUIT_BREAKER.can_execute() {
        let remaining_secs = CIRCUIT_RESET_TIMEOUT_MS / 1000;
        return Err(ConnectionError::CircuitOpen(remaining_secs));
    }

    ensure_daemon_running().await?;

    let mut last_error: Option<ConnectionError> = None;
    let mut consecutive_failures = 0;

    for attempt in 0..MAX_RETRIES {
        match send_request_once(method, params.clone(), timeout_ms).await {
            Ok(result) => {
                CIRCUIT_BREAKER.record_success();
                return Ok(result);
            }
            Err(e) => {
                consecutive_failures += 1;

                // Don't retry on daemon errors (those are application-level errors)
                if matches!(e, ConnectionError::DaemonError { .. }) {
                    // Application errors don't affect circuit breaker
                    return Err(e);
                }

                // Don't retry on timeout (that's a user-visible error)
                if matches!(e, ConnectionError::RequestTimeout(_)) {
                    CIRCUIT_BREAKER.record_failure();
                    return Err(e);
                }

                // Record failure for circuit breaker
                CIRCUIT_BREAKER.record_failure();
                last_error = Some(e);

                if attempt < MAX_RETRIES - 1 {
                    // Try to recover daemon on connection failures
                    if consecutive_failures >= 2 {
                        if let Err(recover_err) = try_recover_daemon().await {
                            eprintln!("Recovery failed: {}", recover_err);
                        }
                    }

                    // Use exponential backoff with jitter
                    let delay = delay_with_jitter(RETRY_DELAY_MS, attempt);
                    eprintln!("Connection attempt {} failed, retrying in {}ms...", attempt + 1, delay);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or(ConnectionError::RetryExhausted(MAX_RETRIES)))
}

/// Get current circuit breaker state
pub fn get_circuit_state() -> CircuitState {
    CIRCUIT_BREAKER.get_state()
}
