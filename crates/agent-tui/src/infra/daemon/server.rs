use crate::infra::ipc::RpcResponse;
use crate::infra::ipc::socket_path;
use tracing::{error, info, warn};

use super::config::DaemonConfig;
use super::router::Router;
use super::signal_handler::SignalHandler;
use super::usecase_container::UseCaseContainer;
use crate::infra::daemon::DaemonError;
use crate::infra::daemon::DaemonMetrics;
use crate::infra::daemon::LivePreviewManager;
use crate::infra::daemon::SessionManager;
use crate::infra::daemon::transport::{
    TransportConnection, TransportError, TransportListener, UnixSocketConnection,
    UnixSocketListener,
};
use crate::infra::daemon::{LockFile, remove_lock_file};
use crate::usecases::ports::SessionRepository;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::thread;
use std::time::{Duration, Instant};

const MAX_CONNECTIONS: usize = 64;
const CHANNEL_CAPACITY: usize = 128;

pub struct DaemonServer {
    session_manager: Arc<SessionManager>,
    usecases: UseCaseContainer<SessionManager>,
    active_connections: Arc<AtomicUsize>,
    metrics: Arc<DaemonMetrics>,
}

struct ThreadPool {
    workers: Vec<thread::JoinHandle<()>>,
    sender: SyncSender<UnixSocketConnection>,
}

impl ThreadPool {
    fn new(
        size: usize,
        server: Arc<DaemonServer>,
        shutdown: Arc<AtomicBool>,
    ) -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::sync_channel::<UnixSocketConnection>(CHANNEL_CAPACITY);
        let receiver = Arc::new(std::sync::Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            let receiver = Arc::clone(&receiver);
            let server = Arc::clone(&server);
            let shutdown = Arc::clone(&shutdown);

            let handle =
                match thread::Builder::new()
                    .name(format!("worker-{}", id))
                    .spawn(move || {
                        loop {
                            if shutdown.load(Ordering::Relaxed) {
                                break;
                            }

                            let conn = {
                                let lock = match receiver.lock() {
                                    Ok(l) => l,
                                    Err(e) => {
                                        error!(worker_id = id, error = %e, "Worker receiver lock poisoned");
                                        break;
                                    }
                                };
                                match lock.recv_timeout(Duration::from_millis(100)) {
                                    Ok(conn) => conn,
                                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                                }
                            };

                            server.active_connections.fetch_add(1, Ordering::Relaxed);
                            server.handle_client(conn);
                            server.active_connections.fetch_sub(1, Ordering::Relaxed);
                        }
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

        Ok(ThreadPool { workers, sender })
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
    pub fn with_config(config: DaemonConfig, shutdown_flag: Arc<AtomicBool>) -> Self {
        let session_manager = Arc::new(SessionManager::with_max_sessions(config.max_sessions));
        let metrics = Arc::new(DaemonMetrics::new());
        let session_repo: Arc<dyn SessionRepository> = session_manager.clone();
        let live_preview = Arc::new(LivePreviewManager::new(session_repo));
        let start_time = Instant::now();
        let active_connections = Arc::new(AtomicUsize::new(0));
        let usecases = UseCaseContainer::new(
            Arc::clone(&session_manager),
            Arc::clone(&metrics),
            start_time,
            Arc::clone(&active_connections),
            shutdown_flag,
            live_preview,
        );
        Self {
            session_manager,
            usecases,
            active_connections,
            metrics,
        }
    }

    pub fn shutdown_all_sessions(&self) {
        let sessions = self.session_manager.list();
        for info in sessions {
            if let Err(e) = self.session_manager.kill(info.id.as_str()) {
                warn!(session_id = %info.id, error = %e, "Failed to kill session during shutdown");
            }
        }
    }

    fn handle_request(&self, request: crate::infra::ipc::RpcRequest) -> RpcResponse {
        let router = Router::new(&self.usecases);
        router.route(request)
    }

    fn handle_client(&self, mut conn: impl TransportConnection) {
        let idle_timeout = DaemonConfig::from_env().idle_timeout;

        if let Err(e) = conn.set_read_timeout(Some(idle_timeout)) {
            error!(error = %e, "Failed to set read timeout");
            return;
        }

        if let Err(e) = conn.set_write_timeout(Some(Duration::from_secs(30))) {
            error!(error = %e, "Failed to set write timeout");
            return;
        }

        loop {
            let request = match conn.read_request() {
                Ok(req) => req,
                Err(TransportError::ConnectionClosed) | Err(TransportError::Timeout) => break,
                Err(TransportError::SizeLimit { max_bytes }) => {
                    self.metrics.record_error();
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
                Err(TransportError::Parse(msg)) => {
                    self.metrics.record_error();
                    let error_response =
                        RpcResponse::error(0, -32700, &format!("Parse error: {}", msg));
                    let _ = conn.write_response(&error_response);
                    continue;
                }
                Err(TransportError::Io(e)) => {
                    error!(error = %e, "Client connection error");
                    break;
                }
            };

            self.metrics.record_request();
            let response = self.handle_request(request);

            if let Err(e) = conn.write_response(&response) {
                match e {
                    TransportError::ConnectionClosed => break,
                    _ => {
                        error!(error = %e, "Client write error");
                        break;
                    }
                }
            }
        }
    }
}

fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();
}

fn bind_socket(socket_path: &std::path::Path) -> Result<UnixSocketListener, DaemonError> {
    if socket_path.exists() {
        std::fs::remove_file(socket_path).map_err(|e| {
            DaemonError::SocketBind(format!("failed to remove stale socket: {}", e))
        })?;
    }

    let listener = UnixSocketListener::bind(socket_path)
        .map_err(|e| DaemonError::SocketBind(format!("failed to bind socket: {}", e)))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| DaemonError::SocketBind(format!("failed to set non-blocking: {}", e)))?;

    Ok(listener)
}

fn run_accept_loop(listener: &UnixSocketListener, pool: &ThreadPool, shutdown: &AtomicBool) {
    while !shutdown.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok(conn) => {
                if let Err(conn) = pool.execute(conn) {
                    warn!("Thread pool channel closed, dropping connection");
                    drop(conn);
                }
            }
            Err(TransportError::Timeout) => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                if !shutdown.load(Ordering::Relaxed) {
                    error!(error = %e, "Error accepting connection");
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
    while server.active_connections.load(Ordering::Relaxed) > 0 {
        if Instant::now() > shutdown_deadline {
            warn!("Shutdown timeout, forcing close");
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn cleanup(
    socket_path: &std::path::Path,
    lock_path: &std::path::Path,
    server: &DaemonServer,
    pool: ThreadPool,
) {
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
    init_logging();

    let socket_path = socket_path();
    let lock_path = socket_path.with_extension("lock");

    let _lock = LockFile::acquire(&lock_path)?;

    let listener = bind_socket(&socket_path)?;
    info!(socket = %socket_path.display(), pid = std::process::id(), "Daemon started");

    let shutdown = Arc::new(AtomicBool::new(false));
    let config = DaemonConfig::from_env();
    let server = Arc::new(DaemonServer::with_config(config, Arc::clone(&shutdown)));

    let _signal_handler = SignalHandler::setup(Arc::clone(&shutdown))?;

    let pool = ThreadPool::new(MAX_CONNECTIONS, Arc::clone(&server), Arc::clone(&shutdown))
        .map_err(|e| DaemonError::ThreadPool(e.to_string()))?;

    run_accept_loop(&listener, &pool, &shutdown);

    info!("Shutting down daemon...");
    wait_for_connections(&server, 5);
    cleanup(&socket_path, &lock_path, &server, pool);

    Ok(())
}
