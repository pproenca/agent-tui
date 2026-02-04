use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::net::Shutdown;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::os::unix::net::UnixStream;
use std::time::Duration;

use crate::infra::ipc::error::ClientError;
use crate::infra::ipc::polling;
use crate::infra::ipc::socket::socket_path;
use tracing::{debug, error, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    Unix,
    Tcp,
}

fn transport_kind() -> TransportKind {
    let kind = match std::env::var("AGENT_TUI_TRANSPORT")
        .unwrap_or_else(|_| "unix".to_string())
        .to_lowercase()
        .as_str()
    {
        "tcp" => TransportKind::Tcp,
        _ => TransportKind::Unix,
    };
    debug!(transport = ?kind, "IPC transport selected");
    kind
}

fn tcp_addr_from_env() -> Option<SocketAddr> {
    std::env::var("AGENT_TUI_TCP_ADDR")
        .ok()
        .and_then(|addr| addr.parse::<SocketAddr>().ok())
}

pub fn default_transport() -> std::sync::Arc<dyn IpcTransport> {
    match transport_kind() {
        TransportKind::Unix => std::sync::Arc::new(UnixSocketTransport),
        TransportKind::Tcp => std::sync::Arc::new(TcpSocketTransport::from_env()),
    }
}

pub enum ClientStream {
    Unix(UnixStream),
    Tcp(TcpStream),
}

impl ClientStream {
    pub fn try_clone(&self) -> Result<Self, ClientError> {
        match self {
            Self::Unix(stream) => Ok(Self::Unix(stream.try_clone()?)),
            Self::Tcp(stream) => Ok(Self::Tcp(stream.try_clone()?)),
        }
    }

    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> Result<(), ClientError> {
        match self {
            Self::Unix(stream) => stream.set_read_timeout(timeout)?,
            Self::Tcp(stream) => stream.set_read_timeout(timeout)?,
        }
        Ok(())
    }

    pub fn set_write_timeout(&self, timeout: Option<Duration>) -> Result<(), ClientError> {
        match self {
            Self::Unix(stream) => stream.set_write_timeout(timeout)?,
            Self::Tcp(stream) => stream.set_write_timeout(timeout)?,
        }
        Ok(())
    }

    pub fn shutdown(&self) -> Result<(), ClientError> {
        match self {
            Self::Unix(stream) => stream.shutdown(Shutdown::Both)?,
            Self::Tcp(stream) => stream.shutdown(Shutdown::Both)?,
        }
        Ok(())
    }
}

impl Read for ClientStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Unix(stream) => stream.read(buf),
            Self::Tcp(stream) => stream.read(buf),
        }
    }
}

impl Write for ClientStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Unix(stream) => stream.write(buf),
            Self::Tcp(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Unix(stream) => stream.flush(),
            Self::Tcp(stream) => stream.flush(),
        }
    }
}

pub trait IpcTransport: Send + Sync {
    fn connect_stream(&self) -> Result<ClientStream, ClientError>;
    fn is_daemon_running(&self) -> bool;

    fn supports_autostart(&self) -> bool {
        false
    }

    fn start_daemon_background(&self) -> Result<(), ClientError> {
        Err(ClientError::DaemonNotRunning)
    }
}

pub struct UnixSocketTransport;

impl IpcTransport for UnixSocketTransport {
    fn connect_stream(&self) -> Result<ClientStream, ClientError> {
        let path = socket_path();
        if !path.exists() {
            debug!(socket = %path.display(), "Daemon socket missing");
            return Err(ClientError::DaemonNotRunning);
        }
        debug!(socket = %path.display(), "Connecting to daemon socket");
        Ok(ClientStream::Unix(UnixStream::connect(&path)?))
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

pub struct TcpSocketTransport {
    addr: Option<SocketAddr>,
}

impl TcpSocketTransport {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr: Some(addr) }
    }

    fn from_env() -> Self {
        Self {
            addr: tcp_addr_from_env(),
        }
    }
}

impl IpcTransport for TcpSocketTransport {
    fn connect_stream(&self) -> Result<ClientStream, ClientError> {
        let Some(addr) = self.addr else {
            debug!("TCP transport configured without address");
            return Err(ClientError::DaemonNotRunning);
        };
        debug!(addr = %addr, "Connecting to daemon TCP socket");
        Ok(ClientStream::Tcp(TcpStream::connect(addr)?))
    }

    fn is_daemon_running(&self) -> bool {
        let Some(addr) = self.addr else {
            return false;
        };
        TcpStream::connect(addr).is_ok()
    }
}

pub struct InMemoryTransport {
    handler: std::sync::Arc<dyn Fn(String) -> String + Send + Sync>,
}

impl InMemoryTransport {
    pub fn new<F>(handler: F) -> Self
    where
        F: Fn(String) -> String + Send + Sync + 'static,
    {
        Self {
            handler: std::sync::Arc::new(handler),
        }
    }
}

impl IpcTransport for InMemoryTransport {
    fn connect_stream(&self) -> Result<ClientStream, ClientError> {
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

        Ok(ClientStream::Unix(client))
    }

    fn is_daemon_running(&self) -> bool {
        true
    }
}

#[cfg(test)]
static TEST_LISTENER: std::sync::OnceLock<
    std::sync::Mutex<Option<std::os::unix::net::UnixListener>>,
> = std::sync::OnceLock::new();
#[cfg(test)]
pub(crate) static USE_DAEMON_START_STUB: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
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
    *holder.lock().unwrap() = Some(listener);
    Ok(())
}

#[cfg(test)]
pub(crate) fn clear_test_listener() {
    if let Some(holder) = TEST_LISTENER.get() {
        holder.lock().unwrap().take();
    }
}

fn start_daemon_background_impl() -> Result<(), ClientError> {
    use std::fs::OpenOptions;
    use std::process::Command;
    use std::process::Stdio;

    let exe = std::env::current_exe()?;
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

    Command::new(exe)
        .args(["daemon", "start", "--foreground"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(stderr)
        .spawn()?;

    let mut delay = polling::INITIAL_POLL_INTERVAL;
    let transport = UnixSocketTransport;
    for i in 0..polling::MAX_STARTUP_POLLS {
        std::thread::sleep(delay);
        if transport.is_daemon_running() {
            return Ok(());
        }

        delay = (delay * 2).min(polling::MAX_POLL_INTERVAL);

        if i == polling::MAX_STARTUP_POLLS - 1
            && let Ok(log_content) = std::fs::read_to_string(&log_path)
        {
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
    }

    Err(ClientError::DaemonNotRunning)
}

pub fn start_daemon_background() -> Result<(), ClientError> {
    #[cfg(test)]
    if USE_DAEMON_START_STUB.load(std::sync::atomic::Ordering::SeqCst) {
        return start_daemon_background_stub();
    }
    start_daemon_background_impl()
}
