//! IPC transport implementations.

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
use tracing::debug;
use tracing::error;
use tracing::warn;

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
static DAEMON_START_TEST_REAPED: std::sync::atomic::AtomicBool =
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

    fn spawn_daemon_reaper(child: std::process::Child) {
        let child = std::sync::Arc::new(std::sync::Mutex::new(child));
        let child_for_thread = std::sync::Arc::clone(&child);
        match std::thread::Builder::new()
            .name("daemon-reaper".to_string())
            .spawn(move || {
                let mut child = child_for_thread.lock().unwrap_or_else(|e| e.into_inner());
                let _ = child.wait();
            }) {
            Ok(_) => {}
            Err(err) => {
                warn!(error = %err, "Failed to spawn daemon reaper thread");
                let mut child = child.lock().unwrap_or_else(|e| e.into_inner());
                let _ = child.wait();
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
    }

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
        std::thread::sleep(delay);
        if transport.is_daemon_running() {
            spawn_daemon_reaper(child);
            return Ok(());
        }

        delay = (delay * 2).min(polling::MAX_POLL_INTERVAL);

        if i == polling::MAX_STARTUP_POLLS - 1 {
            log_recent_failure();
        }
    }

    if let Ok(Some(_status)) = child.try_wait() {
        #[cfg(test)]
        {
            DAEMON_START_TEST_REAPED.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        return Err(ClientError::DaemonNotRunning);
    }
    terminate_daemon_child(child);
    Err(ClientError::DaemonNotRunning)
}

pub fn start_daemon_background() -> Result<(), ClientError> {
    #[cfg(test)]
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
}
