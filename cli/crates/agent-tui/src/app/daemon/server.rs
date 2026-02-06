//! Daemon server runtime.

use crate::adapters::rpc::RpcResponse;
use crate::common::DaemonError;
use crate::common::telemetry;
use libc::POLLIN;
use libc::poll;
use libc::pollfd;
use std::collections::HashSet;
use std::io::Read;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::RawFd;
use std::os::unix::net::UnixStream;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::app::daemon::rpc_core::RpcCore;
use crate::app::daemon::rpc_core::RpcCoreError;
use crate::app::daemon::rpc_core::RpcResponseWriter;
use crate::app::daemon::transport::TransportConnection;
use crate::app::daemon::transport::TransportError;
use crate::app::daemon::transport::TransportListener;
use crate::app::daemon::transport::UnixSocketConnection;
use crate::app::daemon::transport::UnixSocketListener;
use crate::app::daemon::ws_server::WsConfig;
use crate::app::daemon::ws_server::WsServerError;
use crate::app::daemon::ws_server::WsServerHandle;
use crate::app::daemon::ws_server::start_ws_server;
use crate::infra::daemon::DaemonConfig;
use crate::infra::daemon::LockFile;
use crate::infra::daemon::SignalHandler;
use crate::infra::daemon::remove_lock_file;
use crate::infra::ipc::socket_path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::mpsc::SyncSender;
use std::thread;
use std::time::Duration;
use std::time::Instant;

const CHANNEL_CAPACITY: usize = 128;
static CONNECTION_ID: AtomicU64 = AtomicU64::new(1);

struct ShutdownWaker {
    reader: UnixStream,
    writer: Arc<std::sync::Mutex<UnixStream>>,
}

impl ShutdownWaker {
    fn new() -> std::io::Result<Self> {
        let (reader, writer) = UnixStream::pair()?;
        reader.set_nonblocking(true)?;
        Ok(Self {
            reader,
            writer: Arc::new(std::sync::Mutex::new(writer)),
        })
    }

    fn notifier(&self) -> crate::usecases::ports::ShutdownNotifierHandle {
        Arc::new(ShutdownNotify {
            writer: Arc::clone(&self.writer),
        })
    }

    fn reader_fd(&self) -> RawFd {
        self.reader.as_raw_fd()
    }

    fn drain(&mut self) {
        let mut buf = [0u8; 64];
        loop {
            match self.reader.read(&mut buf) {
                Ok(0) => break,
                Ok(_) => continue,
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
    }
}

struct ShutdownNotify {
    writer: Arc<std::sync::Mutex<UnixStream>>,
}

impl crate::usecases::ports::ShutdownNotifier for ShutdownNotify {
    fn notify(&self) {
        if let Ok(mut writer) = self.writer.lock() {
            let _ = writer.write_all(&[1]);
        }
    }
}

struct UnixRpcWriter<'a> {
    conn: &'a mut UnixSocketConnection,
}

impl RpcResponseWriter for UnixRpcWriter<'_> {
    fn write_response(&mut self, response: &RpcResponse) -> Result<(), RpcCoreError> {
        self.conn.write_response(response).map_err(|err| match err {
            TransportError::ConnectionClosed => RpcCoreError::ConnectionClosed,
            other => RpcCoreError::Other(other.to_string()),
        })
    }
}

pub struct DaemonServer {
    core: Arc<RpcCore>,
    active_connections: Arc<AtomicUsize>,
    active_fds: Arc<std::sync::Mutex<HashSet<RawFd>>>,
    connection_wait_lock: Arc<std::sync::Mutex<()>>,
    connection_cv: Arc<std::sync::Condvar>,
    stream_threads: std::sync::Mutex<Vec<thread::JoinHandle<()>>>,
}

struct ThreadPool {
    workers: Vec<thread::JoinHandle<()>>,
    sender: SyncSender<UnixSocketConnection>,
}

impl ThreadPool {
    fn new(size: usize, server: Arc<DaemonServer>) -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::sync_channel::<UnixSocketConnection>(CHANNEL_CAPACITY);
        let receiver = Arc::new(std::sync::Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            let receiver = Arc::clone(&receiver);
            let server = Arc::clone(&server);

            let handle = match thread::Builder::new()
                .name(format!("worker-{id}"))
                .spawn(move || {
                    let worker_span = tracing::info_span!("daemon_worker", worker_id = id);
                    let _worker_guard = worker_span.enter();
                    debug!(worker_id = id, "Worker thread started");
                    loop {
                        let conn = {
                            let lock = match receiver.lock() {
                                Ok(l) => l,
                                Err(e) => {
                                    error!(worker_id = id, error = %e, "Worker receiver lock poisoned");
                                    break;
                                }
                            };
                            match lock.recv() {
                                Ok(conn) => conn,
                                Err(mpsc::RecvError) => break,
                            }
                        };

                        server.active_connections.fetch_add(1, Ordering::Relaxed);
                        Arc::clone(&server).handle_client(conn);
                        let remaining = server.active_connections.fetch_sub(1, Ordering::Relaxed) - 1;
                        if remaining == 0 {
                            server.connection_cv.notify_all();
                        }
                    }
                    debug!(worker_id = id, "Worker thread stopped");
                }) {
                Ok(h) => h,
                Err(e) => {
                    error!(worker_id = id, error = %e, "Failed to spawn worker");
                    continue;
                }
            };

            workers.push(handle);
        }

        if workers.is_empty() {
            return Err(std::io::Error::other("Failed to spawn any worker threads"));
        }

        if workers.len() < size {
            warn!(
                spawned = workers.len(),
                requested = size,
                "Only spawned partial worker threads"
            );
        }

        Ok(Self { workers, sender })
    }

    fn execute(&self, conn: UnixSocketConnection) -> Result<(), UnixSocketConnection> {
        self.sender.try_send(conn).map_err(|e| match e {
            mpsc::TrySendError::Full(c) | mpsc::TrySendError::Disconnected(c) => c,
        })
    }

    fn shutdown(self) {
        drop(self.sender);
        for worker in self.workers {
            let _ = worker.join();
        }
    }
}

impl DaemonServer {
    pub fn with_config(
        config: DaemonConfig,
        shutdown_flag: Arc<AtomicBool>,
        shutdown_notifier: crate::usecases::ports::ShutdownNotifierHandle,
    ) -> Self {
        let core = Arc::new(RpcCore::with_config(
            config,
            shutdown_flag,
            shutdown_notifier,
        ));
        Self {
            core,
            active_connections: Arc::new(AtomicUsize::new(0)),
            active_fds: Arc::new(std::sync::Mutex::new(HashSet::new())),
            connection_wait_lock: Arc::new(std::sync::Mutex::new(())),
            connection_cv: Arc::new(std::sync::Condvar::new()),
            stream_threads: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn session_repository_handle(&self) -> Arc<dyn crate::usecases::ports::SessionRepository> {
        self.core.session_repository_handle()
    }

    pub fn shutdown_all_sessions(&self) {
        self.core.shutdown_all_sessions();
    }

    fn register_connection(&self, fd: RawFd) {
        let mut fds = self.active_fds.lock().unwrap_or_else(|e| e.into_inner());
        fds.insert(fd);
    }

    fn unregister_connection(&self, fd: RawFd) {
        let mut fds = self.active_fds.lock().unwrap_or_else(|e| e.into_inner());
        fds.remove(&fd);
    }

    fn shutdown_connections(&self) {
        let fds = {
            let guard = self.active_fds.lock().unwrap_or_else(|e| e.into_inner());
            guard.iter().copied().collect::<Vec<_>>()
        };
        for fd in fds {
            // SAFETY: shutting down a socket fd is safe and idempotent for active connections.
            unsafe {
                libc::shutdown(fd, libc::SHUT_RDWR);
            }
        }
    }

    fn register_stream_thread(&self, handle: thread::JoinHandle<()>) {
        let mut guard = self
            .stream_threads
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        guard.push(handle);
    }

    fn join_stream_threads(&self, timeout: Duration) {
        let handles = {
            let mut guard = self
                .stream_threads
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            guard.drain(..).collect::<Vec<_>>()
        };

        for handle in handles {
            let (tx, rx) = mpsc::channel();
            let handle_cell = Arc::new(std::sync::Mutex::new(Some(handle)));
            let handle_for_joiner = Arc::clone(&handle_cell);
            let joiner = thread::Builder::new()
                .name("stream-joiner".to_string())
                .spawn(move || {
                    if let Some(handle) = handle_for_joiner
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .take()
                    {
                        let _ = handle.join();
                    }
                    let _ = tx.send(());
                });
            match joiner {
                Ok(_) => {
                    if rx.recv_timeout(timeout).is_err() {
                        warn!(
                            timeout_ms = timeout.as_millis(),
                            "Timed out joining stream thread"
                        );
                    }
                }
                Err(err) => {
                    warn!(error = %err, "Failed to spawn stream joiner thread; joining inline");
                    if let Some(handle) =
                        handle_cell.lock().unwrap_or_else(|e| e.into_inner()).take()
                    {
                        let _ = handle.join();
                    }
                }
            }
        }
    }

    fn handle_client(self: Arc<Self>, mut conn: UnixSocketConnection) {
        let idle_timeout = DaemonConfig::from_env().idle_timeout();
        let conn_id = CONNECTION_ID.fetch_add(1, Ordering::Relaxed);
        let conn_fd = conn.raw_fd();
        self.register_connection(conn_fd);
        let conn_span = tracing::info_span!("daemon_connection", conn_id);
        let _conn_guard = conn_span.enter();
        debug!(conn_id, "Client connected");

        if let Err(err) = conn.set_read_timeout(Some(idle_timeout)) {
            warn!(error = %err, "Failed to set read timeout");
        }
        if let Err(err) = conn.set_write_timeout(Some(Duration::from_secs(30))) {
            warn!(error = %err, "Failed to set write timeout");
        }

        loop {
            let request = match conn.read_request() {
                Ok(req) => req,
                Err(TransportError::ConnectionClosed) | Err(TransportError::Timeout) => break,
                Err(TransportError::SizeLimit { max_bytes }) => {
                    warn!(max_bytes, "Request size limit exceeded");
                    let error_response = RpcResponse::error(
                        0,
                        -32700,
                        &format!(
                            "Parse error: request size limit exceeded ({}MB max)",
                            max_bytes / 1024 / 1024
                        ),
                    );
                    let _ = conn.write_response(&error_response);
                    break;
                }
                Err(TransportError::Parse(err)) => {
                    debug!(error = %err, "Request parse error");
                    let error_response =
                        RpcResponse::error(0, -32700, &format!("Parse error: {err}"));
                    let _ = conn.write_response(&error_response);
                    continue;
                }
                Err(TransportError::Serialize(err)) => {
                    error!(error = %err, "Response serialize error");
                    break;
                }
                Err(TransportError::Io(err)) => {
                    error!(error = %err, "Client connection error");
                    break;
                }
            };

            let request_id = request.id;
            let method = request.method.clone();
            let session = request.param_str("session").map(str::to_string);
            let session_field = session.as_deref().unwrap_or("-");
            let request_span = tracing::debug_span!(
                "rpc_request",
                request_id,
                method = %method,
                session = %session_field
            );
            let _request_guard = request_span.enter();
            let start = Instant::now();

            if let Some(kind) = RpcCore::stream_kind_for_method(&method) {
                self.spawn_stream_thread(conn, request, kind, conn_id, conn_fd);
                return;
            }

            let response = self.core.route(request);
            debug!(
                request_id,
                method = %method,
                elapsed_ms = start.elapsed().as_millis(),
                "RPC request handled"
            );

            if let Err(err) = conn.write_response(&response) {
                match err {
                    TransportError::ConnectionClosed => break,
                    _ => {
                        error!(error = %err, "Client write error");
                        break;
                    }
                }
            }
        }

        self.unregister_connection(conn_fd);
        debug!(conn_id, "Client disconnected");
    }

    fn spawn_stream_thread(
        self: &Arc<Self>,
        conn: UnixSocketConnection,
        request: crate::adapters::rpc::RpcRequest,
        kind: crate::app::daemon::rpc_core::StreamKind,
        conn_id: u64,
        conn_fd: RawFd,
    ) {
        let server = Arc::clone(self);
        let method = match kind {
            crate::app::daemon::rpc_core::StreamKind::Attach => "attach_stream",
            crate::app::daemon::rpc_core::StreamKind::LivePreview => "live_preview_stream",
        };

        server.active_connections.fetch_add(1, Ordering::Relaxed);
        let payload = Arc::new(std::sync::Mutex::new(Some((conn, request))));
        let payload_for_thread = Arc::clone(&payload);
        let server_for_thread = Arc::clone(&server);
        let span = tracing::info_span!("daemon_stream", conn_id, method = %method);
        let spawn_result = thread::Builder::new()
            .name(format!("stream-{method}-{conn_id}"))
            .spawn(move || {
                let _guard = span.enter();
                let start = Instant::now();

                let Some((mut conn, request)) = payload_for_thread
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .take()
                else {
                    warn!("Stream payload missing; dropping connection");
                    let remaining = server_for_thread
                        .active_connections
                        .fetch_sub(1, Ordering::Relaxed)
                        - 1;
                    if remaining == 0 {
                        server_for_thread.connection_cv.notify_all();
                    }
                    server_for_thread.unregister_connection(conn_fd);
                    return;
                };

                let stream_result = {
                    let mut writer = UnixRpcWriter { conn: &mut conn };
                    server_for_thread
                        .core
                        .handle_stream(&mut writer, request, kind, None)
                };

                debug!(
                    conn_id,
                    method = %method,
                    elapsed_ms = start.elapsed().as_millis(),
                    "RPC stream handled"
                );
                if let Err(err) = stream_result {
                    match err {
                        RpcCoreError::ConnectionClosed => {}
                        other => error!(error = %other, "RPC stream error"),
                    }
                }
                debug!(conn_id, method = %method, "Stream connection closed");

                let remaining = server_for_thread
                    .active_connections
                    .fetch_sub(1, Ordering::Relaxed)
                    - 1;
                if remaining == 0 {
                    server_for_thread.connection_cv.notify_all();
                }
                server_for_thread.unregister_connection(conn_fd);
            });

        match spawn_result {
            Ok(handle) => server.register_stream_thread(handle),
            Err(_) => {
                let Some((mut conn, request)) =
                    payload.lock().unwrap_or_else(|e| e.into_inner()).take()
                else {
                    warn!("Stream payload missing; dropping connection");
                    let remaining = server.active_connections.fetch_sub(1, Ordering::Relaxed) - 1;
                    if remaining == 0 {
                        server.connection_cv.notify_all();
                    }
                    return;
                };

                warn!("Failed to spawn stream thread, handling on worker thread");
                let mut writer = UnixRpcWriter { conn: &mut conn };
                let _ = server.core.handle_stream(&mut writer, request, kind, None);

                let remaining = server.active_connections.fetch_sub(1, Ordering::Relaxed) - 1;
                if remaining == 0 {
                    server.connection_cv.notify_all();
                }
                server.unregister_connection(conn_fd);
            }
        }
    }
}

fn init_logging() -> telemetry::TelemetryGuard {
    telemetry::init_tracing("info")
}

fn bind_socket(
    socket_path: &std::path::Path,
    max_request_bytes: usize,
) -> Result<UnixSocketListener, DaemonError> {
    if socket_path.exists() {
        std::fs::remove_file(socket_path).map_err(|e| DaemonError::SocketBind {
            operation: "remove stale socket",
            source: Box::new(e),
        })?;
    }

    let listener = UnixSocketListener::bind(socket_path, max_request_bytes).map_err(|e| {
        DaemonError::SocketBind {
            operation: "bind socket",
            source: Box::new(e),
        }
    })?;
    listener
        .set_nonblocking(true)
        .map_err(|e| DaemonError::SocketBind {
            operation: "set non-blocking",
            source: Box::new(e),
        })?;

    Ok(listener)
}

fn run_accept_loop(
    listener: &UnixSocketListener,
    pool: &ThreadPool,
    shutdown: &AtomicBool,
    waker: &mut ShutdownWaker,
) {
    let listener_fd = listener.as_raw_fd();
    let wake_fd = waker.reader_fd();
    let mut fds = [
        pollfd {
            fd: listener_fd,
            events: POLLIN,
            revents: 0,
        },
        pollfd {
            fd: wake_fd,
            events: POLLIN,
            revents: 0,
        },
    ];

    while !shutdown.load(Ordering::Relaxed) {
        // SAFETY: `fds` is stack-allocated and length matches call argument.
        let poll_result = unsafe { poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, -1) };
        if poll_result < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            error!(error = %err, "poll failed");
            continue;
        }

        if fds[1].revents & POLLIN != 0 {
            waker.drain();
            if shutdown.load(Ordering::Relaxed) {
                break;
            }
        }

        if fds[0].revents & POLLIN != 0 {
            loop {
                match listener.accept() {
                    Ok(conn) => {
                        if let Err(conn) = pool.execute(conn) {
                            warn!("Thread pool channel closed, dropping connection");
                            drop(conn);
                        }
                    }
                    Err(TransportError::Timeout) => break,
                    Err(err) => {
                        if !shutdown.load(Ordering::Relaxed) {
                            error!(error = %err, "Error accepting connection");
                        }
                        break;
                    }
                }
            }
        }
    }
}

fn wait_for_connections(server: &DaemonServer, timeout_secs: u64) {
    info!(
        active_connections = server.active_connections.load(Ordering::Relaxed),
        "Waiting for active connections to complete"
    );
    let shutdown_deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let mut guard = server
        .connection_wait_lock
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    while server.active_connections.load(Ordering::Relaxed) > 0 {
        let now = Instant::now();
        if now >= shutdown_deadline {
            warn!("Shutdown timeout, forcing close");
            break;
        }
        let remaining = shutdown_deadline.saturating_duration_since(now);
        let (_guard, result) = server
            .connection_cv
            .wait_timeout(guard, remaining)
            .unwrap_or_else(|e| e.into_inner());
        guard = _guard;
        if result.timed_out() {
            warn!("Shutdown timeout, forcing close");
            break;
        }
    }
}

fn cleanup(
    socket_path: &std::path::Path,
    lock_path: &std::path::Path,
    server: &DaemonServer,
    pool: ThreadPool,
    ws_handle: Option<WsServerHandle>,
) {
    if let Some(handle) = ws_handle {
        handle.shutdown();
    }

    info!("Cleaning up sessions...");
    server.shutdown_all_sessions();

    info!("Stopping thread pool...");
    pool.shutdown();

    if socket_path.exists() {
        let _ = std::fs::remove_file(socket_path);
    }

    remove_lock_file(lock_path);

    info!("Daemon shutdown complete");
}

pub fn start_daemon() -> Result<(), DaemonError> {
    let _telemetry = init_logging();

    let socket_path = socket_path();
    let lock_path = socket_path.with_extension("lock");

    let _lock = LockFile::acquire(&lock_path)?;

    let config = DaemonConfig::from_env();
    let max_connections = config.max_connections();
    let max_request_bytes = config.max_request_bytes();
    let listener = bind_socket(&socket_path, max_request_bytes)?;
    info!(socket = %socket_path.display(), pid = std::process::id(), "Daemon started");

    let shutdown = Arc::new(AtomicBool::new(false));
    let waker = ShutdownWaker::new()
        .map_err(|e| DaemonError::SignalSetup(format!("failed to create shutdown waker: {e}")))?;
    let shutdown_notifier = waker.notifier();

    let server = Arc::new(DaemonServer::with_config(
        config,
        Arc::clone(&shutdown),
        Arc::clone(&shutdown_notifier),
    ));

    let ws_handle = match start_ws_server(
        Arc::clone(&server.core),
        Arc::clone(&shutdown),
        WsConfig::from_env(),
    ) {
        Ok(handle) => Some(handle),
        Err(WsServerError::Disabled) => None,
        Err(WsServerError::Io { operation, source }) => {
            warn!(operation = %operation, error = %source, "Failed to start WS server");
            None
        }
        Err(WsServerError::InvalidListen { message }) => {
            warn!(reason = %message, "Invalid WS listen address");
            None
        }
    };

    let _signal_handler = SignalHandler::setup(Arc::clone(&shutdown), Some(shutdown_notifier))?;

    let pool = ThreadPool::new(max_connections, Arc::clone(&server))
        .map_err(|e| DaemonError::ThreadPool(e.to_string()))?;

    let mut waker = waker;
    run_accept_loop(&listener, &pool, &shutdown, &mut waker);

    info!("Shutting down daemon...");
    server.shutdown_connections();
    wait_for_connections(&server, 5);
    server.join_stream_threads(Duration::from_secs(2));
    cleanup(&socket_path, &lock_path, &server, pool, ws_handle);

    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        reason = "Test-only assertions use expect/unwrap for clarity."
    )]

    use super::*;
    use crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier;
    use std::sync::mpsc;

    #[test]
    fn shutdown_connections_closes_idle_client() {
        let shutdown = Arc::new(AtomicBool::new(false));
        let notifier: crate::usecases::ports::ShutdownNotifierHandle =
            Arc::new(NoopShutdownNotifier);
        let server = Arc::new(DaemonServer::with_config(
            DaemonConfig::default(),
            Arc::clone(&shutdown),
            notifier,
        ));

        let (client, server_stream) = UnixStream::pair().expect("failed to create unix pair");
        let conn = UnixSocketConnection::new(server_stream).expect("failed to wrap connection");

        let (tx, rx) = mpsc::channel();
        let server_clone = Arc::clone(&server);
        std::thread::spawn(move || {
            server_clone.handle_client(conn);
            let _ = tx.send(());
        });

        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            if !server.active_fds.lock().unwrap().is_empty() {
                break;
            }
            if Instant::now() >= deadline {
                panic!("connection was not registered");
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        server.shutdown_connections();

        assert!(
            rx.recv_timeout(Duration::from_secs(1)).is_ok(),
            "client handler did not exit after shutdown"
        );

        drop(client);
    }

    #[test]
    fn join_stream_threads_drains_handles() {
        let shutdown = Arc::new(AtomicBool::new(false));
        let notifier: crate::usecases::ports::ShutdownNotifierHandle =
            Arc::new(NoopShutdownNotifier);
        let server = Arc::new(DaemonServer::with_config(
            DaemonConfig::default(),
            Arc::clone(&shutdown),
            notifier,
        ));

        let handle = std::thread::spawn(|| {});
        server.register_stream_thread(handle);
        server.join_stream_threads(Duration::from_secs(1));

        assert!(server.stream_threads.lock().unwrap().is_empty());
    }
}
