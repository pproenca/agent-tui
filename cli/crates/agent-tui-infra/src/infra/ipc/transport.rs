//! IPC transport implementations.

use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::net::Shutdown;
use std::net::TcpStream;
use std::net::ToSocketAddrs;
use std::os::unix::net::UnixStream;
use std::time::Duration;

use crate::infra::ipc::error::ClientError;
use crate::infra::ipc::polling;
use crate::infra::ipc::socket::socket_path;
use serde::Deserialize;
use tracing::debug;
use tracing::error;
use tracing::warn;
use tungstenite::Message;
use tungstenite::WebSocket;
use tungstenite::client::IntoClientRequest;
use url::Url;

const DEFAULT_TRANSPORT: &str = "unix";
const DEFAULT_WS_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(60);
const DEFAULT_WRITE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    Unix,
    Ws,
}

fn transport_kind() -> TransportKind {
    let raw = std::env::var("AGENT_TUI_TRANSPORT")
        .unwrap_or_else(|_| DEFAULT_TRANSPORT.to_string())
        .to_ascii_lowercase();
    let kind = match raw.as_str() {
        "unix" => TransportKind::Unix,
        "ws" | "websocket" => TransportKind::Ws,
        other => {
            warn!(
                transport = %other,
                "Unknown AGENT_TUI_TRANSPORT value; defaulting to unix"
            );
            TransportKind::Unix
        }
    };
    debug!(transport = ?kind, "IPC transport selected");
    kind
}

#[derive(Debug, Deserialize)]
struct WsStateFile {
    ws_url: String,
}

fn ws_state_path_from_env() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("AGENT_TUI_WS_STATE") {
        return std::path::PathBuf::from(path);
    }
    if let Ok(path) = std::env::var("AGENT_TUI_API_STATE") {
        return std::path::PathBuf::from(path);
    }
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"));
    home.join(".agent-tui").join("api.json")
}

fn ws_addr_from_state() -> Option<Url> {
    let path = ws_state_path_from_env();
    let contents = std::fs::read_to_string(path).ok()?;
    let state: WsStateFile = serde_json::from_str(&contents).ok()?;
    Url::parse(state.ws_url.trim()).ok()
}

fn ws_addr_from_env() -> Option<Url> {
    std::env::var("AGENT_TUI_WS_ADDR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .and_then(|value| Url::parse(&value).ok())
}

pub(crate) fn default_transport() -> std::sync::Arc<dyn IpcTransport> {
    match transport_kind() {
        TransportKind::Unix => std::sync::Arc::new(UnixSocketTransport),
        TransportKind::Ws => std::sync::Arc::new(WsSocketTransport::from_env()),
    }
}

pub(crate) struct UnixMessageConnection {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl UnixMessageConnection {
    fn new(stream: UnixStream) -> Result<Self, ClientError> {
        let reader_stream = stream.try_clone()?;
        Ok(Self {
            reader: BufReader::new(reader_stream),
            writer: stream,
        })
    }
}

pub(crate) struct WsMessageConnection {
    socket: WebSocket<TcpStream>,
}

fn ws_error_to_client(err: tungstenite::Error) -> ClientError {
    match err {
        tungstenite::Error::Io(io_err) => ClientError::ConnectionFailed(io_err),
        other => ClientError::UnexpectedResponse {
            message: format!("websocket error: {other}"),
        },
    }
}

fn set_ws_timeouts(
    stream: &mut TcpStream,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
) -> Result<(), ClientError> {
    stream.set_read_timeout(read_timeout)?;
    stream.set_write_timeout(write_timeout)?;
    Ok(())
}

fn shutdown_ws_stream(stream: &mut TcpStream) -> Result<(), ClientError> {
    stream.shutdown(Shutdown::Both)?;
    Ok(())
}

fn connect_ws_socket(url: &Url) -> Result<WebSocket<TcpStream>, ClientError> {
    if url.scheme() != "ws" {
        return Err(ClientError::UnexpectedResponse {
            message: format!(
                "unsupported websocket scheme '{}'; only ws:// is supported",
                url.scheme()
            ),
        });
    }

    let host = url
        .host_str()
        .ok_or_else(|| ClientError::UnexpectedResponse {
            message: "websocket URL is missing a host".to_string(),
        })?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| ClientError::UnexpectedResponse {
            message: "websocket URL is missing a port".to_string(),
        })?;
    let mut addrs = (host, port)
        .to_socket_addrs()
        .map_err(ClientError::ConnectionFailed)?;
    let addr = addrs
        .next()
        .ok_or_else(|| ClientError::UnexpectedResponse {
            message: format!("failed to resolve websocket host '{host}:{port}'"),
        })?;

    let stream = TcpStream::connect_timeout(&addr, DEFAULT_WS_CONNECT_TIMEOUT)?;
    stream.set_nodelay(true)?;

    let request =
        url.as_str()
            .into_client_request()
            .map_err(|err| ClientError::UnexpectedResponse {
                message: format!("invalid websocket URL: {err}"),
            })?;

    let (mut socket, _response) =
        tungstenite::client::client(request, stream).map_err(|err| match err {
            tungstenite::HandshakeError::Failure(ws_err) => ws_error_to_client(ws_err),
            tungstenite::HandshakeError::Interrupted(_) => ClientError::UnexpectedResponse {
                message: "websocket handshake interrupted".to_string(),
            },
        })?;
    set_ws_timeouts(
        socket.get_mut(),
        Some(DEFAULT_READ_TIMEOUT),
        Some(DEFAULT_WRITE_TIMEOUT),
    )?;
    Ok(socket)
}

pub(crate) enum ClientConnection {
    Unix(UnixMessageConnection),
    Ws(Box<WsMessageConnection>),
}

impl ClientConnection {
    pub fn set_read_timeout(&mut self, timeout: Option<Duration>) -> Result<(), ClientError> {
        match self {
            Self::Unix(conn) => conn.writer.set_read_timeout(timeout)?,
            Self::Ws(conn) => {
                let write_timeout = conn
                    .socket
                    .get_mut()
                    .write_timeout()
                    .map_err(ClientError::ConnectionFailed)?;
                set_ws_timeouts(conn.socket.get_mut(), timeout, write_timeout)?;
            }
        }
        Ok(())
    }

    pub fn set_write_timeout(&mut self, timeout: Option<Duration>) -> Result<(), ClientError> {
        match self {
            Self::Unix(conn) => conn.writer.set_write_timeout(timeout)?,
            Self::Ws(conn) => {
                let read_timeout = conn
                    .socket
                    .get_mut()
                    .read_timeout()
                    .map_err(ClientError::ConnectionFailed)?;
                set_ws_timeouts(conn.socket.get_mut(), read_timeout, timeout)?;
            }
        }
        Ok(())
    }

    pub fn send_message(&mut self, message: &str) -> Result<(), ClientError> {
        match self {
            Self::Unix(conn) => {
                writeln!(conn.writer, "{message}")?;
                conn.writer.flush()?;
            }
            Self::Ws(conn) => {
                conn.socket
                    .send(Message::Text(message.to_string()))
                    .map_err(ws_error_to_client)?;
            }
        }
        Ok(())
    }

    pub fn read_message(&mut self) -> Result<Option<String>, ClientError> {
        match self {
            Self::Unix(conn) => {
                let mut line = String::new();
                loop {
                    line.clear();
                    let bytes = conn.reader.read_line(&mut line)?;
                    if bytes == 0 {
                        return Ok(None);
                    }
                    let trimmed = line.trim_end_matches(['\r', '\n']);
                    if trimmed.is_empty() {
                        continue;
                    }
                    return Ok(Some(trimmed.to_string()));
                }
            }
            Self::Ws(conn) => loop {
                match conn.socket.read() {
                    Ok(Message::Text(text)) => return Ok(Some(text.to_string())),
                    Ok(Message::Binary(_)) => {
                        return Err(ClientError::UnexpectedResponse {
                            message: "received binary websocket frame; expected text JSON-RPC"
                                .to_string(),
                        });
                    }
                    Ok(Message::Close(_)) => return Ok(None),
                    Ok(Message::Ping(payload)) => {
                        conn.socket
                            .send(Message::Pong(payload))
                            .map_err(ws_error_to_client)?;
                    }
                    Ok(Message::Pong(_)) => {}
                    Ok(_) => {}
                    Err(err) => return Err(ws_error_to_client(err)),
                }
            },
        }
    }

    pub fn shutdown(&mut self) -> Result<(), ClientError> {
        match self {
            Self::Unix(conn) => conn.writer.shutdown(Shutdown::Both)?,
            Self::Ws(conn) => {
                let _ = conn.socket.close(None);
                shutdown_ws_stream(conn.socket.get_mut())?;
            }
        }
        Ok(())
    }
}

pub(crate) trait IpcTransport: Send + Sync {
    fn connect_connection(&self) -> Result<ClientConnection, ClientError>;
    fn is_daemon_running(&self) -> bool;

    fn supports_autostart(&self) -> bool {
        false
    }

    fn start_daemon_background(&self) -> Result<(), ClientError> {
        Err(ClientError::DaemonNotRunning)
    }
}

pub(crate) struct UnixSocketTransport;

impl IpcTransport for UnixSocketTransport {
    fn connect_connection(&self) -> Result<ClientConnection, ClientError> {
        let path = socket_path();
        if !path.exists() {
            debug!(socket = %path.display(), "Daemon socket missing");
            return Err(ClientError::DaemonNotRunning);
        }
        debug!(socket = %path.display(), "Connecting to daemon socket");
        let stream = UnixStream::connect(&path)?;
        Ok(ClientConnection::Unix(UnixMessageConnection::new(stream)?))
    }

    fn is_daemon_running(&self) -> bool {
        let path = socket_path();
        if !path.exists() {
            return false;
        }
        UnixStream::connect(path).is_ok()
    }

    fn supports_autostart(&self) -> bool {
        true
    }

    fn start_daemon_background(&self) -> Result<(), ClientError> {
        start_daemon_background()
    }
}

pub(crate) struct WsSocketTransport {
    addr: Option<Url>,
}

impl WsSocketTransport {
    pub(crate) fn new(addr: Url) -> Self {
        Self { addr: Some(addr) }
    }

    fn from_env() -> Self {
        Self {
            addr: ws_addr_from_env().or_else(ws_addr_from_state),
        }
    }
}

impl IpcTransport for WsSocketTransport {
    fn connect_connection(&self) -> Result<ClientConnection, ClientError> {
        let Some(addr) = self.addr.as_ref() else {
            debug!("WS transport configured without AGENT_TUI_WS_ADDR and no state file");
            return Err(ClientError::DaemonNotRunning);
        };
        debug!(addr = %addr, "Connecting to daemon websocket");
        let socket = connect_ws_socket(addr)?;
        Ok(ClientConnection::Ws(Box::new(WsMessageConnection {
            socket,
        })))
    }

    fn is_daemon_running(&self) -> bool {
        let Some(addr) = self.addr.as_ref() else {
            return false;
        };
        connect_ws_socket(addr).is_ok()
    }
}

pub(crate) struct InMemoryTransport {
    handler: std::sync::Arc<dyn Fn(String) -> String + Send + Sync>,
}

impl InMemoryTransport {
    pub(crate) fn new<F>(handler: F) -> Self
    where
        F: Fn(String) -> String + Send + Sync + 'static,
    {
        Self {
            handler: std::sync::Arc::new(handler),
        }
    }
}

impl IpcTransport for InMemoryTransport {
    fn connect_connection(&self) -> Result<ClientConnection, ClientError> {
        let (client, mut server) = UnixStream::pair()?;
        let handler = self.handler.clone();

        let span = tracing::debug_span!("ipc_in_memory");
        let builder = std::thread::Builder::new().name("ipc-in-memory".to_string());
        builder
            .spawn(move || {
                let _guard = span.enter();
                let reader_stream = match server.try_clone() {
                    Ok(stream) => stream,
                    Err(_) => return,
                };
                let mut reader = BufReader::new(reader_stream);

                loop {
                    let mut line = String::new();
                    match reader.read_line(&mut line) {
                        Ok(0) => break,
                        Ok(_) => {}
                        Err(_) => break,
                    }

                    let request = line.trim_end_matches(['\r', '\n']).to_string();
                    let mut response = (handler)(request);
                    if !response.ends_with('\n') {
                        response.push('\n');
                    }

                    if server.write_all(response.as_bytes()).is_err() {
                        break;
                    }
                    let _ = server.flush();
                }
            })
            .map_err(|err| ClientError::ConnectionFailed(std::io::Error::other(err.to_string())))?;

        Ok(ClientConnection::Unix(UnixMessageConnection::new(client)?))
    }

    fn is_daemon_running(&self) -> bool {
        true
    }
}

#[cfg(any(test, feature = "test-support"))]
static TEST_LISTENER: std::sync::OnceLock<
    std::sync::Mutex<Option<std::os::unix::net::UnixListener>>,
> = std::sync::OnceLock::new();
#[cfg(any(test, feature = "test-support"))]
pub static USE_DAEMON_START_STUB: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
#[cfg(any(test, feature = "test-support"))]
static DAEMON_START_TEST_REAPED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(any(test, feature = "test-support"))]
fn start_daemon_background_stub() -> Result<(), ClientError> {
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;

    let path = socket_path();
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path)?;
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o777));
    if let Ok(accept_listener) = listener.try_clone() {
        std::thread::spawn(move || {
            let _ = accept_listener.accept();
        });
    }
    let holder = TEST_LISTENER.get_or_init(|| std::sync::Mutex::new(None));
    if let Ok(mut guard) = holder.lock() {
        *guard = Some(listener);
    }
    Ok(())
}

#[cfg(any(test, feature = "test-support"))]
pub fn clear_test_listener() {
    if let Some(holder) = TEST_LISTENER.get()
        && let Ok(mut guard) = holder.lock()
    {
        guard.take();
    }
}

fn handle_reaper_spawn_failure(child: std::process::Child) -> Result<(), ClientError> {
    terminate_daemon_child(child);
    Err(ClientError::UnexpectedResponse {
        message: "failed to spawn daemon reaper thread; daemon process was terminated".to_string(),
    })
}

fn spawn_daemon_reaper(child: std::process::Child) -> Result<(), ClientError> {
    let child_cell = std::sync::Arc::new(std::sync::Mutex::new(Some(child)));
    let child_for_thread = std::sync::Arc::clone(&child_cell);
    match std::thread::Builder::new()
        .name("daemon-reaper".to_string())
        .spawn(move || {
            let mut guard = child_for_thread.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(mut child) = guard.take() {
                let _ = child.wait();
            }
        }) {
        Ok(_) => Ok(()),
        Err(err) => {
            warn!(error = %err, "Failed to spawn daemon reaper thread");
            let mut guard = child_cell.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(child) = guard.take() {
                handle_reaper_spawn_failure(child)
            } else {
                Err(ClientError::UnexpectedResponse {
                    message: "failed to spawn daemon reaper thread".to_string(),
                })
            }
        }
    }
}

fn terminate_daemon_child(mut child: std::process::Child) {
    if let Ok(Some(_status)) = child.try_wait() {
        return;
    }
    if let Err(err) = child.kill() {
        warn!(error = %err, "Failed to terminate daemon process");
    }
    if let Err(err) = child.wait() {
        warn!(error = %err, "Failed to reap daemon process");
    }
    #[cfg(test)]
    {
        DAEMON_START_TEST_REAPED.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

fn start_daemon_background_impl() -> Result<(), ClientError> {
    // Guard: prevent recursive spawning. If AGENT_TUI_DAEMON_FOREGROUND is set,
    // we are already a daemon child process and must not spawn another.
    if std::env::var("AGENT_TUI_DAEMON_FOREGROUND")
        .ok()
        .is_some_and(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
    {
        return Err(ClientError::DaemonNotRunning);
    }

    use std::fs::OpenOptions;
    use std::process::Command;
    use std::process::Stdio;

    let log_path = socket_path().with_extension("log");

    let log_file = match OpenOptions::new().create(true).append(true).open(&log_path) {
        Ok(f) => Some(f),
        Err(e) => {
            warn!(
                error = %e,
                path = %log_path.display(),
                "Could not open daemon log file"
            );
            None
        }
    };

    let stderr = match log_file {
        Some(f) => Stdio::from(f),
        None => Stdio::null(),
    };

    #[cfg(test)]
    let test_command = std::env::var("AGENT_TUI_DAEMON_START_TEST_CMD").ok();
    #[cfg(not(test))]
    let test_command: Option<String> = None;

    let mut command = if let Some(cmd) = test_command {
        Command::new(cmd)
    } else {
        let exe = std::env::current_exe()?;
        let mut cmd = Command::new(exe);
        cmd.args(["daemon", "start"]);
        cmd.env("AGENT_TUI_DAEMON_FOREGROUND", "1");
        cmd
    };

    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(stderr)
        .spawn()?;

    let log_recent_failure = || {
        if let Ok(log_content) = std::fs::read_to_string(&log_path) {
            let last_lines: String = log_content
                .lines()
                .rev()
                .take(5)
                .collect::<Vec<_>>()
                .join("\n");
            if !last_lines.is_empty() {
                error!("Daemon failed to start. Recent log output:\n{}", last_lines);
            }
        }
    };

    let mut delay = polling::INITIAL_POLL_INTERVAL;
    let transport = UnixSocketTransport;
    for i in 0..polling::MAX_STARTUP_POLLS {
        if let Ok(Some(_status)) = child.try_wait() {
            #[cfg(test)]
            {
                DAEMON_START_TEST_REAPED.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            log_recent_failure();
            return Err(ClientError::DaemonNotRunning);
        }
        std::thread::park_timeout(delay);
        if transport.is_daemon_running() {
            return spawn_daemon_reaper(child);
        }

        delay = (delay * 2).min(polling::MAX_POLL_INTERVAL);

        if i == polling::MAX_STARTUP_POLLS - 1 {
            log_recent_failure();
        }
    }

    if let Ok(Some(_status)) = child.try_wait() {
        #[cfg(any(test, feature = "test-support"))]
        {
            DAEMON_START_TEST_REAPED.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        return Err(ClientError::DaemonNotRunning);
    }
    terminate_daemon_child(child);
    Err(ClientError::DaemonNotRunning)
}

pub fn start_daemon_background() -> Result<(), ClientError> {
    #[cfg(any(test, feature = "test-support"))]
    if USE_DAEMON_START_STUB.load(std::sync::atomic::Ordering::SeqCst) {
        return start_daemon_background_stub();
    }
    start_daemon_background_impl()
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        reason = "Test-only assertions use expect/unwrap for clarity."
    )]

    use super::*;
    use std::sync::atomic::Ordering;
    use tempfile::TempDir;

    struct EnvGuard {
        key: &'static str,
        value: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: impl Into<String>) -> Self {
            let value = value.into();
            let prev = std::env::var(key).ok();
            // SAFETY: test-only environment mutation for isolated test setup.
            unsafe {
                std::env::set_var(key, &value);
            }
            Self { key, value: prev }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: test-only environment restoration after mutation.
            unsafe {
                match self.value.take() {
                    Some(prev) => std::env::set_var(self.key, prev),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }

    #[test]
    fn start_daemon_background_reaps_early_exit() {
        let temp_dir = TempDir::new_in("/tmp").expect("Failed to create temp dir");
        let socket_path = temp_dir.path().join("daemon.sock");
        let _socket_guard = EnvGuard::set("AGENT_TUI_SOCKET", socket_path.display().to_string());
        let _cmd_guard = EnvGuard::set("AGENT_TUI_DAEMON_START_TEST_CMD", "true");

        DAEMON_START_TEST_REAPED.store(false, Ordering::SeqCst);

        let result = start_daemon_background_impl();
        assert!(matches!(result, Err(ClientError::DaemonNotRunning)));
        assert!(DAEMON_START_TEST_REAPED.load(Ordering::SeqCst));
    }

    #[test]
    fn start_daemon_background_impl_guards_against_recursive_spawn() {
        let temp_dir = TempDir::new_in("/tmp").expect("Failed to create temp dir");
        let socket_path = temp_dir.path().join("daemon.sock");
        let _socket_guard = EnvGuard::set("AGENT_TUI_SOCKET", socket_path.display().to_string());
        let _fg_guard = EnvGuard::set("AGENT_TUI_DAEMON_FOREGROUND", "1");

        let result = start_daemon_background_impl();
        assert!(
            matches!(result, Err(ClientError::DaemonNotRunning)),
            "should refuse to spawn when AGENT_TUI_DAEMON_FOREGROUND is set"
        );
    }

    #[cfg(unix)]
    #[test]
    fn reaper_failure_fallback_terminates_and_reaps_child() {
        use crate::infra::ipc::ProcessController;
        use crate::infra::ipc::ProcessStatus;
        use crate::infra::ipc::UnixProcessController;
        use std::process::Command;
        use std::time::Duration;
        use std::time::Instant;

        let child = Command::new("sh")
            .arg("-c")
            .arg("sleep 30")
            .spawn()
            .expect("failed to spawn child");
        let pid = child.id();

        let start = Instant::now();
        let result = handle_reaper_spawn_failure(child);
        assert!(matches!(
            result,
            Err(ClientError::UnexpectedResponse { .. })
        ));
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "reaper fallback should terminate child in bounded time"
        );

        let controller = UnixProcessController;
        let status = controller
            .check_process(pid)
            .expect("process check should succeed");
        assert!(
            matches!(status, ProcessStatus::NotFound),
            "child must be reaped after fallback, got {status:?}"
        );
    }
}
