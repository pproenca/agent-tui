use agent_tui_ipc::RpcResponse;
use agent_tui_ipc::socket_path;

use crate::config::DaemonConfig;
use crate::error::DaemonError;
use crate::handlers;
use crate::metrics::DaemonMetrics;
use crate::session::SessionManager;
use crate::transport::{
    TransportConnection, TransportError, TransportListener, UnixSocketConnection,
    UnixSocketListener,
};
use crate::usecase_container::UseCaseContainer;
use serde_json::json;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::io::AsRawFd;
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
    start_time: Instant,
    #[allow(dead_code)]
    shutdown: Arc<AtomicBool>,
    active_connections: Arc<AtomicUsize>,
    metrics: Arc<DaemonMetrics>,
}

impl Default for DaemonServer {
    fn default() -> Self {
        Self::new()
    }
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
                                        eprintln!("Worker {} receiver lock poisoned: {}", id, e);
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
                        eprintln!("Failed to spawn worker {}: {}", id, e);
                        continue;
                    }
                };

            workers.push(handle);
        }

        if workers.is_empty() {
            return Err(std::io::Error::other("Failed to spawn any worker threads"));
        }

        if workers.len() < size {
            eprintln!(
                "Warning: Only spawned {}/{} worker threads",
                workers.len(),
                size
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
    pub fn new() -> Self {
        Self::with_config(DaemonConfig::default())
    }

    pub fn with_config(config: DaemonConfig) -> Self {
        let session_manager = Arc::new(SessionManager::with_max_sessions(config.max_sessions));
        let usecases = UseCaseContainer::new(Arc::clone(&session_manager));
        Self {
            session_manager,
            usecases,
            start_time: Instant::now(),
            shutdown: Arc::new(AtomicBool::new(false)),
            active_connections: Arc::new(AtomicUsize::new(0)),
            metrics: Arc::new(DaemonMetrics::new()),
        }
    }

    pub fn with_shutdown_and_config(shutdown: Arc<AtomicBool>, config: DaemonConfig) -> Self {
        let session_manager = Arc::new(SessionManager::with_max_sessions(config.max_sessions));
        let usecases = UseCaseContainer::new(Arc::clone(&session_manager));
        Self {
            session_manager,
            usecases,
            start_time: Instant::now(),
            shutdown,
            active_connections: Arc::new(AtomicUsize::new(0)),
            metrics: Arc::new(DaemonMetrics::new()),
        }
    }

    pub fn shutdown_all_sessions(&self) {
        let sessions = self.session_manager.list();
        for info in sessions {
            if let Err(e) = self.session_manager.kill(info.id.as_str()) {
                eprintln!("Warning: Failed to kill session {}: {}", info.id, e);
            }
        }
    }

    fn handle_request(&self, request: agent_tui_ipc::RpcRequest) -> RpcResponse {
        match request.method.as_str() {
            "ping" => RpcResponse::success(request.id, json!({ "pong": true })),

            "health" => handlers::diagnostics::handle_health(
                &self.session_manager,
                &self.metrics,
                self.start_time,
                &self.active_connections,
                request,
            ),

            "metrics" => handlers::diagnostics::handle_metrics(
                &self.session_manager,
                &self.metrics,
                self.start_time,
                &self.active_connections,
                request,
            ),

            // Session handlers using use cases
            "spawn" => handlers::session::handle_spawn(&self.usecases.session.spawn, request),
            "kill" => handlers::session::handle_kill(&self.usecases.session.kill, request),
            "restart" => handlers::session::handle_restart(&self.usecases.session.restart, request),
            "sessions" => {
                handlers::session::handle_sessions(&self.usecases.session.sessions, request)
            }
            "resize" => handlers::session::handle_resize(&self.usecases.session.resize, request),
            "attach" => handlers::session::handle_attach(&self.usecases.session.attach, request),

            // Element handlers - key operations use use cases, others use session_manager
            "snapshot" => {
                handlers::elements::handle_snapshot_uc(&self.usecases.elements.snapshot, request)
            }
            "click" => handlers::elements::handle_click_uc(&self.usecases.elements.click, request),
            "dbl_click" => handlers::elements::handle_dbl_click(&self.session_manager, request),
            "fill" => handlers::elements::handle_fill(&self.session_manager, request),
            "find" => handlers::elements::handle_find(&self.session_manager, request),
            "count" => handlers::elements::handle_count(&self.session_manager, request),
            "scroll" => handlers::elements::handle_scroll(&self.session_manager, request),
            "scroll_into_view" => {
                handlers::elements::handle_scroll_into_view(&self.session_manager, request)
            }
            "get_text" => handlers::elements::handle_get_text(&self.session_manager, request),
            "get_value" => handlers::elements::handle_get_value(&self.session_manager, request),
            "is_visible" => handlers::elements::handle_is_visible(&self.session_manager, request),
            "is_focused" => handlers::elements::handle_is_focused(&self.session_manager, request),
            "is_enabled" => handlers::elements::handle_is_enabled(&self.session_manager, request),
            "is_checked" => handlers::elements::handle_is_checked(&self.session_manager, request),
            "get_focused" => handlers::elements::handle_get_focused(&self.session_manager, request),
            "get_title" => handlers::elements::handle_get_title(&self.session_manager, request),
            "focus" => handlers::elements::handle_focus(&self.session_manager, request),
            "clear" => handlers::elements::handle_clear(&self.session_manager, request),
            "select_all" => handlers::elements::handle_select_all(&self.session_manager, request),
            "toggle" => handlers::elements::handle_toggle(&self.session_manager, request),
            "select" => handlers::elements::handle_select(&self.session_manager, request),
            "multiselect" => handlers::elements::handle_multiselect(&self.session_manager, request),

            // Input handlers - keystroke and type use use cases
            "keystroke" => {
                handlers::input::handle_keystroke_uc(&self.usecases.input.keystroke, request)
            }
            "keydown" => handlers::input::handle_keydown(&self.session_manager, request),
            "keyup" => handlers::input::handle_keyup(&self.session_manager, request),
            "type" => handlers::input::handle_type_uc(&self.usecases.input.type_text, request),

            // Wait handler using use case
            "wait" => handlers::wait::handle_wait_uc(&self.usecases.wait, request),

            // Recording handlers
            "record_start" => {
                handlers::recording::handle_record_start(&self.session_manager, request)
            }
            "record_stop" => {
                handlers::recording::handle_record_stop(&self.session_manager, request)
            }
            "record_status" => {
                handlers::recording::handle_record_status(&self.session_manager, request)
            }

            // Diagnostics handlers
            "trace" => handlers::diagnostics::handle_trace(&self.session_manager, request),
            "console" => handlers::diagnostics::handle_console(&self.session_manager, request),
            "errors" => handlers::diagnostics::handle_errors(&self.session_manager, request),
            "pty_read" => handlers::diagnostics::handle_pty_read(&self.session_manager, request),
            "pty_write" => handlers::diagnostics::handle_pty_write(&self.session_manager, request),

            "screen" => RpcResponse::error(
                request.id,
                -32601,
                "Method 'screen' is deprecated. Use 'snapshot' with strip_ansi=true instead.",
            ),

            _ => RpcResponse::error(
                request.id,
                -32601,
                &format!("Method not found: {}", request.method),
            ),
        }
    }

    fn handle_client(&self, mut conn: impl TransportConnection) {
        let idle_timeout = DaemonConfig::from_env().idle_timeout;

        if let Err(e) = conn.set_read_timeout(Some(idle_timeout)) {
            eprintln!("Failed to set read timeout: {}", e);
            return;
        }

        if let Err(e) = conn.set_write_timeout(Some(Duration::from_secs(30))) {
            eprintln!("Failed to set write timeout: {}", e);
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
                    eprintln!("Client connection error: {}", e);
                    break;
                }
            };

            self.metrics.record_request();
            let response = self.handle_request(request);

            if let Err(e) = conn.write_response(&response) {
                match e {
                    TransportError::ConnectionClosed => break,
                    _ => {
                        eprintln!("Client write error: {}", e);
                        break;
                    }
                }
            }
        }
    }
}

pub fn start_daemon() -> Result<(), DaemonError> {
    let socket_path = socket_path();
    let lock_path = socket_path.with_extension("lock");

    let lock_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| DaemonError::LockFailed(format!("failed to open lock file: {}", e)))?;

    let fd = lock_file.as_raw_fd();

    let result = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
    if result != 0 {
        return Err(DaemonError::AlreadyRunning);
    }

    lock_file
        .set_len(0)
        .map_err(|e| DaemonError::LockFailed(format!("failed to truncate lock file: {}", e)))?;
    let mut lock_file = lock_file;
    writeln!(lock_file, "{}", std::process::id())
        .map_err(|e| DaemonError::LockFailed(format!("failed to write PID to lock file: {}", e)))?;

    if socket_path.exists() {
        std::fs::remove_file(&socket_path).map_err(|e| {
            DaemonError::SocketBind(format!("failed to remove stale socket: {}", e))
        })?;
    }

    let listener = UnixSocketListener::bind(&socket_path)
        .map_err(|e| DaemonError::SocketBind(format!("failed to bind socket: {}", e)))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| DaemonError::SocketBind(format!("failed to set non-blocking: {}", e)))?;

    eprintln!("agent-tui daemon started on {}", socket_path.display());
    eprintln!("PID: {}", std::process::id());

    let shutdown = Arc::new(AtomicBool::new(false));
    let config = DaemonConfig::from_env();
    let server = Arc::new(DaemonServer::with_shutdown_and_config(
        Arc::clone(&shutdown),
        config,
    ));

    let mut signals =
        Signals::new([SIGINT, SIGTERM]).map_err(|e| DaemonError::SignalSetup(e.to_string()))?;
    let shutdown_signal = Arc::clone(&shutdown);
    thread::Builder::new()
        .name("signal-handler".to_string())
        .spawn(move || {
            if let Some(sig) = signals.forever().next() {
                eprintln!("\nReceived signal {}, initiating graceful shutdown...", sig);
                shutdown_signal.store(true, Ordering::SeqCst);
            }
        })
        .map_err(|e| DaemonError::SignalSetup(format!("failed to spawn signal handler: {}", e)))?;

    let pool = ThreadPool::new(MAX_CONNECTIONS, Arc::clone(&server), Arc::clone(&shutdown))
        .map_err(|e| DaemonError::ThreadPool(e.to_string()))?;

    while !shutdown.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok(conn) => {
                if let Err(conn) = pool.execute(conn) {
                    eprintln!("Thread pool channel closed, dropping connection");
                    drop(conn);
                }
            }
            Err(TransportError::Timeout) => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                if !shutdown.load(Ordering::Relaxed) {
                    eprintln!("Error accepting connection: {}", e);
                }
            }
        }
    }

    eprintln!("Shutting down daemon...");

    eprintln!(
        "Waiting for {} active connections to complete...",
        server.active_connections.load(Ordering::Relaxed)
    );
    let shutdown_deadline = Instant::now() + Duration::from_secs(5);
    while server.active_connections.load(Ordering::Relaxed) > 0 {
        if Instant::now() > shutdown_deadline {
            eprintln!("Shutdown timeout, forcing close");
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    eprintln!("Cleaning up sessions...");
    server.shutdown_all_sessions();

    eprintln!("Stopping thread pool...");
    pool.shutdown();

    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }

    if lock_path.exists() {
        let _ = std::fs::remove_file(&lock_path);
    }

    eprintln!("Daemon shutdown complete.");
    Ok(())
}
