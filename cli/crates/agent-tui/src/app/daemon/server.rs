//! Daemon server runtime.

use crate::adapters::attach_output_to_response;
use crate::adapters::daemon::Router;
use crate::adapters::daemon::UseCaseContainer;
use crate::adapters::parse_attach_input;
use crate::adapters::rpc::RpcResponse;
use crate::adapters::session_error_response;
use crate::common::DaemonError;
use crate::common::telemetry;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use libc::POLLIN;
use libc::poll;
use libc::pollfd;
use serde::Serialize;
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

use crate::app::daemon::http_api::ApiConfig;
use crate::app::daemon::http_api::ApiServerError;
use crate::app::daemon::http_api::ApiServerHandle;
use crate::app::daemon::http_api::start_api_server;
use crate::app::daemon::transport::TransportConnection;
use crate::app::daemon::transport::TransportError;
use crate::app::daemon::transport::TransportListener;
use crate::app::daemon::transport::UnixSocketConnection;
use crate::app::daemon::transport::UnixSocketListener;
use crate::infra::daemon::DaemonConfig;
use crate::infra::daemon::DaemonMetrics;
use crate::infra::daemon::LockFile;
use crate::infra::daemon::SessionManager;
use crate::infra::daemon::SignalHandler;
use crate::infra::daemon::SystemClock;
use crate::infra::daemon::SystemInfo;
use crate::infra::daemon::remove_lock_file;
use crate::infra::ipc::socket_path;
use crate::usecases::AttachUseCase;
use crate::usecases::ports::MetricsProvider;
use crate::usecases::ports::SessionRepository;
use crate::usecases::ports::StreamCursor;
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
const ATTACH_STREAM_MAX_CHUNK_BYTES: usize = 64 * 1024;
const ATTACH_STREAM_MAX_TICK_BYTES: usize = 512 * 1024;
const ATTACH_STREAM_HEARTBEAT: Duration = Duration::from_secs(30);
const LIVE_PREVIEW_STREAM_MAX_CHUNK_BYTES: usize = 64 * 1024;
const LIVE_PREVIEW_STREAM_MAX_TICK_BYTES: usize = 256 * 1024;
const LIVE_PREVIEW_STREAM_HEARTBEAT: Duration = Duration::from_secs(5);
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

    fn reader_fd(&self) -> std::os::unix::io::RawFd {
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

pub struct DaemonServer {
    session_manager: Arc<SessionManager>,
    usecases: UseCaseContainer<SessionManager>,
    active_connections: Arc<AtomicUsize>,
    active_fds: Arc<std::sync::Mutex<HashSet<RawFd>>>,
    connection_wait_lock: Arc<std::sync::Mutex<()>>,
    connection_cv: Arc<std::sync::Condvar>,
    metrics: Arc<DaemonMetrics>,
    shutdown_flag: Arc<AtomicBool>,
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

            let handle =
                match thread::Builder::new()
                    .name(format!("worker-{}", id))
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
                            let remaining =
                                server.active_connections.fetch_sub(1, Ordering::Relaxed) - 1;
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
    pub fn with_config(
        config: DaemonConfig,
        shutdown_flag: Arc<AtomicBool>,
        shutdown_notifier: crate::usecases::ports::ShutdownNotifierHandle,
    ) -> Self {
        let session_manager = Arc::new(SessionManager::with_max_sessions(config.max_sessions()));
        let metrics = Arc::new(DaemonMetrics::new());
        let metrics_provider: Arc<dyn MetricsProvider> = metrics.clone();
        let system_info = Arc::new(SystemInfo::new(Instant::now()));
        let clock = Arc::new(SystemClock::new());
        let active_connections = Arc::new(AtomicUsize::new(0));
        let usecases = UseCaseContainer::new(
            Arc::clone(&session_manager),
            metrics_provider,
            system_info,
            clock,
            Arc::clone(&active_connections),
            Arc::clone(&shutdown_flag),
            shutdown_notifier,
        );
        Self {
            session_manager,
            usecases,
            active_connections,
            active_fds: Arc::new(std::sync::Mutex::new(HashSet::new())),
            connection_wait_lock: Arc::new(std::sync::Mutex::new(())),
            connection_cv: Arc::new(std::sync::Condvar::new()),
            metrics,
            shutdown_flag: Arc::clone(&shutdown_flag),
            stream_threads: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn session_repository_handle(&self) -> Arc<dyn SessionRepository> {
        let repository: Arc<dyn SessionRepository> = self.session_manager.clone();
        repository
    }

    fn should_shutdown(&self) -> bool {
        self.shutdown_flag.load(Ordering::Relaxed)
    }

    pub fn shutdown_all_sessions(&self) {
        let sessions = self.session_manager.list();
        for info in sessions {
            if let Err(e) = self.session_manager.kill(info.id.as_str()) {
                warn!(session_id = %info.id, error = %e, "Failed to kill session during shutdown");
            }
        }
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
            let joiner = thread::Builder::new()
                .name("stream-joiner".to_string())
                .spawn(move || {
                    let _ = handle.join();
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
                    warn!(error = %err, "Failed to spawn stream joiner thread");
                }
            }
        }
    }

    fn handle_request(&self, request: crate::adapters::rpc::RpcRequest) -> RpcResponse {
        let router = Router::new(&self.usecases);
        router.route(request)
    }

    fn handle_attach_stream(
        &self,
        conn: &mut impl TransportConnection,
        request: crate::adapters::rpc::RpcRequest,
    ) -> Result<(), TransportError> {
        let req_id = request.id;
        let input = match parse_attach_input(&request) {
            Ok(input) => input,
            Err(response) => {
                let _ = conn.write_response(&response);
                return Ok(());
            }
        };

        let session_id = input.session_id.to_string();
        match self.usecases.session.attach.execute(input) {
            Ok(output) => {
                let response = attach_output_to_response(req_id, output);
                conn.write_response(&response)?;
            }
            Err(err) => {
                let response = session_error_response(req_id, err);
                let _ = conn.write_response(&response);
                return Ok(());
            }
        }

        let session =
            match SessionRepository::resolve(self.session_manager.as_ref(), Some(&session_id)) {
                Ok(session) => session,
                Err(err) => {
                    let response = session_error_response(req_id, err);
                    let _ = conn.write_response(&response);
                    return Ok(());
                }
            };

        if let Err(err) = session.update() {
            let response = session_error_response(req_id, err);
            let _ = conn.write_response(&response);
            return Ok(());
        }

        #[derive(Serialize)]
        struct AttachReady<'a> {
            event: &'static str,
            session_id: &'a str,
        }

        #[derive(Serialize)]
        struct AttachDropped {
            event: &'static str,
            dropped_bytes: u64,
        }

        #[derive(Serialize)]
        struct AttachOutput<'a> {
            event: &'static str,
            data: &'a str,
            bytes: usize,
            dropped_bytes: u64,
        }

        #[derive(Serialize)]
        struct AttachEvent {
            event: &'static str,
        }

        let ready = RpcResponse::success_json(
            req_id,
            &AttachReady {
                event: "ready",
                session_id: &session_id,
            },
        );
        conn.write_response(&ready)?;

        let stream_seq = session.live_preview_snapshot().stream_seq;
        let subscription = session.stream_subscribe();
        let mut cursor = StreamCursor { seq: stream_seq };

        loop {
            if self.should_shutdown() {
                let response = RpcResponse::success_json(req_id, &AttachEvent { event: "closed" });
                let _ = conn.write_response(&response);
                return Ok(());
            }
            let mut budget = ATTACH_STREAM_MAX_TICK_BYTES;
            let mut sent_any = false;

            loop {
                if budget == 0 {
                    break;
                }

                let max_chunk = budget.min(ATTACH_STREAM_MAX_CHUNK_BYTES);
                let read = match session.stream_read(&mut cursor, max_chunk, 0) {
                    Ok(read) => read,
                    Err(err) => {
                        let response = session_error_response(req_id, err);
                        let _ = conn.write_response(&response);
                        return Ok(());
                    }
                };

                if read.dropped_bytes > 0 && read.data.is_empty() {
                    let response = RpcResponse::success_json(
                        req_id,
                        &AttachDropped {
                            event: "dropped",
                            dropped_bytes: read.dropped_bytes,
                        },
                    );
                    conn.write_response(&response)?;
                    sent_any = true;
                }

                if !read.data.is_empty() {
                    let data_b64 = STANDARD.encode(&read.data);
                    let response = RpcResponse::success_json(
                        req_id,
                        &AttachOutput {
                            event: "output",
                            data: &data_b64,
                            bytes: read.data.len(),
                            dropped_bytes: read.dropped_bytes,
                        },
                    );
                    conn.write_response(&response)?;
                    sent_any = true;
                    budget = budget.saturating_sub(read.data.len());
                    if read.closed {
                        let response =
                            RpcResponse::success_json(req_id, &AttachEvent { event: "closed" });
                        let _ = conn.write_response(&response);
                        return Ok(());
                    }
                    continue;
                }

                if read.closed {
                    let response =
                        RpcResponse::success_json(req_id, &AttachEvent { event: "closed" });
                    let _ = conn.write_response(&response);
                    return Ok(());
                }

                break;
            }

            if sent_any && budget == 0 {
                continue;
            }

            if !subscription.wait(Some(ATTACH_STREAM_HEARTBEAT)) {
                if self.should_shutdown() {
                    let response =
                        RpcResponse::success_json(req_id, &AttachEvent { event: "closed" });
                    let _ = conn.write_response(&response);
                    return Ok(());
                }
                let response =
                    RpcResponse::success_json(req_id, &AttachEvent { event: "heartbeat" });
                conn.write_response(&response)?;
            }
        }
    }

    fn handle_live_preview_stream(
        &self,
        conn: &mut impl TransportConnection,
        request: crate::adapters::rpc::RpcRequest,
    ) -> Result<(), TransportError> {
        let req_id = request.id;
        let session_param = request.param_str("session").map(str::to_string);

        let session = match SessionRepository::resolve(
            self.session_manager.as_ref(),
            session_param.as_deref(),
        ) {
            Ok(session) => session,
            Err(err) => {
                let response = session_error_response(req_id, err);
                let _ = conn.write_response(&response);
                return Ok(());
            }
        };

        if let Err(err) = session.update() {
            let response = session_error_response(req_id, err);
            let _ = conn.write_response(&response);
            return Ok(());
        }

        let snapshot = session.live_preview_snapshot();
        let session_id = session.session_id().to_string();
        #[derive(Serialize)]
        struct LivePreviewReady<'a> {
            event: &'static str,
            session_id: &'a str,
            cols: u16,
            rows: u16,
        }

        #[derive(Serialize)]
        struct LivePreviewInit<'a> {
            event: &'static str,
            time: f64,
            cols: u16,
            rows: u16,
            init: &'a str,
        }

        #[derive(Serialize)]
        struct LivePreviewDropped {
            event: &'static str,
            time: f64,
            dropped_bytes: u64,
        }

        #[derive(Serialize)]
        struct LivePreviewOutput<'a> {
            event: &'static str,
            time: f64,
            data_b64: &'a str,
        }

        #[derive(Serialize)]
        struct LivePreviewClosed {
            event: &'static str,
            time: f64,
        }

        #[derive(Serialize)]
        struct LivePreviewResize {
            event: &'static str,
            time: f64,
            cols: u16,
            rows: u16,
        }

        #[derive(Serialize)]
        struct LivePreviewHeartbeat {
            event: &'static str,
            time: f64,
        }

        let ready = RpcResponse::success_json(
            req_id,
            &LivePreviewReady {
                event: "ready",
                session_id: &session_id,
                cols: snapshot.cols,
                rows: snapshot.rows,
            },
        );
        conn.write_response(&ready)?;

        let start_time = Instant::now();
        let init = RpcResponse::success_json(
            req_id,
            &LivePreviewInit {
                event: "init",
                time: start_time.elapsed().as_secs_f64(),
                cols: snapshot.cols,
                rows: snapshot.rows,
                init: &snapshot.seq,
            },
        );
        conn.write_response(&init)?;

        let subscription = session.stream_subscribe();
        let mut cursor = StreamCursor::default();
        let mut last_size = (snapshot.cols, snapshot.rows);

        loop {
            if self.should_shutdown() {
                let response = RpcResponse::success_json(
                    req_id,
                    &LivePreviewClosed {
                        event: "closed",
                        time: start_time.elapsed().as_secs_f64(),
                    },
                );
                let _ = conn.write_response(&response);
                return Ok(());
            }
            let mut budget = LIVE_PREVIEW_STREAM_MAX_TICK_BYTES;
            let mut sent_any = false;

            loop {
                if budget == 0 {
                    break;
                }

                let max_chunk = budget.min(LIVE_PREVIEW_STREAM_MAX_CHUNK_BYTES);
                let read = match session.stream_read(&mut cursor, max_chunk, 0) {
                    Ok(read) => read,
                    Err(err) => {
                        let response = session_error_response(req_id, err);
                        let _ = conn.write_response(&response);
                        return Ok(());
                    }
                };

                if read.dropped_bytes > 0 {
                    let dropped = RpcResponse::success_json(
                        req_id,
                        &LivePreviewDropped {
                            event: "dropped",
                            time: start_time.elapsed().as_secs_f64(),
                            dropped_bytes: read.dropped_bytes,
                        },
                    );
                    conn.write_response(&dropped)?;
                    if let Err(err) = session.update() {
                        let response = session_error_response(req_id, err);
                        let _ = conn.write_response(&response);
                        return Ok(());
                    }
                    let snapshot = session.live_preview_snapshot();
                    let init = RpcResponse::success_json(
                        req_id,
                        &LivePreviewInit {
                            event: "init",
                            time: start_time.elapsed().as_secs_f64(),
                            cols: snapshot.cols,
                            rows: snapshot.rows,
                            init: &snapshot.seq,
                        },
                    );
                    conn.write_response(&init)?;
                    last_size = (snapshot.cols, snapshot.rows);
                    cursor.seq = read.latest_cursor.seq;
                    sent_any = true;
                    break;
                }

                if !read.data.is_empty() {
                    let data_b64 = STANDARD.encode(&read.data);
                    let response = RpcResponse::success_json(
                        req_id,
                        &LivePreviewOutput {
                            event: "output",
                            time: start_time.elapsed().as_secs_f64(),
                            data_b64: &data_b64,
                        },
                    );
                    conn.write_response(&response)?;
                    sent_any = true;
                    budget = budget.saturating_sub(read.data.len());
                    if read.closed {
                        let response = RpcResponse::success_json(
                            req_id,
                            &LivePreviewClosed {
                                event: "closed",
                                time: start_time.elapsed().as_secs_f64(),
                            },
                        );
                        let _ = conn.write_response(&response);
                        return Ok(());
                    }
                    continue;
                }

                if read.closed {
                    let response = RpcResponse::success_json(
                        req_id,
                        &LivePreviewClosed {
                            event: "closed",
                            time: start_time.elapsed().as_secs_f64(),
                        },
                    );
                    let _ = conn.write_response(&response);
                    return Ok(());
                }

                break;
            }

            let size = session.size();
            if size != last_size {
                let resize = RpcResponse::success_json(
                    req_id,
                    &LivePreviewResize {
                        event: "resize",
                        time: start_time.elapsed().as_secs_f64(),
                        cols: size.0,
                        rows: size.1,
                    },
                );
                conn.write_response(&resize)?;
                last_size = size;
                sent_any = true;
            }

            if sent_any && budget == 0 {
                continue;
            }

            if !subscription.wait(Some(LIVE_PREVIEW_STREAM_HEARTBEAT)) {
                if self.should_shutdown() {
                    let response = RpcResponse::success_json(
                        req_id,
                        &LivePreviewClosed {
                            event: "closed",
                            time: start_time.elapsed().as_secs_f64(),
                        },
                    );
                    let _ = conn.write_response(&response);
                    return Ok(());
                }
                let response = RpcResponse::success_json(
                    req_id,
                    &LivePreviewHeartbeat {
                        event: "heartbeat",
                        time: start_time.elapsed().as_secs_f64(),
                    },
                );
                conn.write_response(&response)?;
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

        if let Err(e) = conn.set_read_timeout(Some(idle_timeout)) {
            warn!(error = %e, "Failed to set read timeout");
        }

        if let Err(e) = conn.set_write_timeout(Some(Duration::from_secs(30))) {
            warn!(error = %e, "Failed to set write timeout");
        }

        loop {
            let request = match conn.read_request() {
                Ok(req) => req,
                Err(TransportError::ConnectionClosed) | Err(TransportError::Timeout) => break,
                Err(TransportError::SizeLimit { max_bytes }) => {
                    self.metrics.record_error();
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
                    self.metrics.record_error();
                    debug!(error = %err, "Request parse error");
                    let error_response =
                        RpcResponse::error(0, -32700, &format!("Parse error: {}", err));
                    let _ = conn.write_response(&error_response);
                    continue;
                }
                Err(TransportError::Serialize(err)) => {
                    self.metrics.record_error();
                    error!(error = %err, "Response serialize error");
                    break;
                }
                Err(TransportError::Io(e)) => {
                    error!(error = %e, "Client connection error");
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

            self.metrics.record_request();

            if method == "attach_stream" {
                self.spawn_stream_thread(conn, request, StreamKind::Attach, conn_id, conn_fd);
                return;
            }

            if method == "live_preview_stream" {
                self.spawn_stream_thread(conn, request, StreamKind::LivePreview, conn_id, conn_fd);
                return;
            }

            let response = self.handle_request(request);
            debug!(
                request_id,
                method = %method,
                elapsed_ms = start.elapsed().as_millis(),
                "RPC request handled"
            );

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
        self.unregister_connection(conn_fd);
        debug!(conn_id, "Client disconnected");
    }

    fn spawn_stream_thread(
        self: &Arc<Self>,
        conn: UnixSocketConnection,
        request: crate::adapters::rpc::RpcRequest,
        kind: StreamKind,
        conn_id: u64,
        conn_fd: RawFd,
    ) {
        let server = Arc::clone(self);
        let method = match kind {
            StreamKind::Attach => "attach_stream",
            StreamKind::LivePreview => "live_preview_stream",
        };

        server.active_connections.fetch_add(1, Ordering::Relaxed);
        let payload = Arc::new(std::sync::Mutex::new(Some((conn, request))));
        let payload_for_thread = Arc::clone(&payload);
        let server_for_thread = Arc::clone(&server);
        let span = tracing::info_span!("daemon_stream", conn_id, method = %method);
        let spawn_result = thread::Builder::new()
            .name(format!("stream-{}-{}", method, conn_id))
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
                let stream_result = match kind {
                    StreamKind::Attach => {
                        server_for_thread.handle_attach_stream(&mut conn, request)
                    }
                    StreamKind::LivePreview => {
                        server_for_thread.handle_live_preview_stream(&mut conn, request)
                    }
                };
                debug!(
                    conn_id,
                    method = %method,
                    elapsed_ms = start.elapsed().as_millis(),
                    "RPC stream handled"
                );
                if let Err(e) = stream_result {
                    match e {
                        TransportError::ConnectionClosed => {}
                        _ => error!(error = %e, "RPC stream error"),
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
            Ok(handle) => {
                server.register_stream_thread(handle);
            }
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
                let _ = match kind {
                    StreamKind::Attach => server.handle_attach_stream(&mut conn, request),
                    StreamKind::LivePreview => {
                        server.handle_live_preview_stream(&mut conn, request)
                    }
                };
                let remaining = server.active_connections.fetch_sub(1, Ordering::Relaxed) - 1;
                if remaining == 0 {
                    server.connection_cv.notify_all();
                }
                server.unregister_connection(conn_fd);
            }
        }
    }
}

#[derive(Clone, Copy)]
enum StreamKind {
    Attach,
    LivePreview,
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
        // SAFETY: `fds` is a stack-allocated array that outlives the call, and the length
        // matches the number of elements passed to `poll`.
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
                    Err(e) => {
                        if !shutdown.load(Ordering::Relaxed) {
                            error!(error = %e, "Error accepting connection");
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
    api_handle: Option<ApiServerHandle>,
) {
    if let Some(handle) = api_handle {
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
        .map_err(|e| DaemonError::SignalSetup(format!("failed to create shutdown waker: {}", e)))?;
    let shutdown_notifier = waker.notifier();
    let server = Arc::new(DaemonServer::with_config(
        config,
        Arc::clone(&shutdown),
        Arc::clone(&shutdown_notifier),
    ));

    let api_handle = match start_api_server(
        server.session_repository_handle(),
        Arc::clone(&shutdown),
        ApiConfig::from_env(),
    ) {
        Ok(handle) => Some(handle),
        Err(ApiServerError::Disabled) => None,
        Err(ApiServerError::Io { operation, source }) => {
            warn!(
                operation = %operation,
                error = %source,
                "Failed to start API server"
            );
            None
        }
        Err(ApiServerError::InvalidListen { message }) => {
            warn!(reason = %message, "Invalid API listen address");
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
    cleanup(&socket_path, &lock_path, &server, pool, api_handle);

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
