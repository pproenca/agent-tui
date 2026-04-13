//! Daemon WebSocket server (UI assets + RPC over WebSocket).

use axum::extract::Query;
use axum::extract::State;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::ws::close_code;
use axum::http::HeaderMap;
use axum::http::header::ORIGIN;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::response::Response;
use axum::routing::get;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::path::Path as StdPath;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tracing::error;
use tracing::info;
use tracing::warn;
use uuid::Uuid;

use crate::adapters::rpc::RpcRequest;
use crate::adapters::rpc::RpcResponse;
use crate::app::daemon::rpc_core::RpcCore;
use crate::app::daemon::rpc_core::RpcCoreError;
use crate::app::daemon::rpc_core::RpcResponseWriter;
use crate::infra::ipc::current_process_identity;

const DEFAULT_WS_LISTEN: &str = "127.0.0.1:0";
const DEFAULT_MAX_CONNECTIONS: usize = 32;
const DEFAULT_WS_QUEUE_CAPACITY: usize = 128;
const WS_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
const WS_RECV_TIMEOUT: Duration = Duration::from_secs(60);
const WS_SEND_TIMEOUT: Duration = Duration::from_secs(15);
const WS_STREAM_TASK_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
const WS_MAX_PARSE_ERRORS: u8 = 3;
const WS_JOIN_POLL_INTERVAL: Duration = Duration::from_millis(20);
const UI_INDEX_HTML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/web/index.html"
));
const UI_APP_JS: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/web/app.js"));
const UI_STYLES_CSS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/web/styles.css"
));
const UI_XTERM_CSS: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/web/xterm.css"));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThreadJoinOutcome {
    Joined,
    ReapingInBackground,
}

#[derive(Debug, Clone)]
pub(crate) struct WsConfig {
    enabled: bool,
    listen: String,
    allow_remote: bool,
    state_path: PathBuf,
    max_connections: usize,
    ws_queue_capacity: usize,
}

impl WsConfig {
    pub fn from_env() -> Self {
        let enabled = env_bool("AGENT_TUI_WS_DISABLED")
            .map(|v| !v)
            .unwrap_or(true);

        let allow_remote = std::env::var("AGENT_TUI_WS_ALLOW_REMOTE")
            .ok()
            .and_then(|v| parse_bool(&v))
            .unwrap_or(false);

        let listen = std::env::var("AGENT_TUI_WS_LISTEN")
            .ok()
            .and_then(non_empty)
            .unwrap_or_else(|| DEFAULT_WS_LISTEN.to_string());

        let state_path = std::env::var("AGENT_TUI_WS_STATE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_state_path());

        let max_connections = std::env::var("AGENT_TUI_WS_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(DEFAULT_MAX_CONNECTIONS);

        let ws_queue_capacity = std::env::var("AGENT_TUI_WS_QUEUE")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(DEFAULT_WS_QUEUE_CAPACITY);

        Self {
            enabled,
            listen,
            allow_remote,
            state_path,
            max_connections,
            ws_queue_capacity,
        }
    }
}

pub(crate) struct WsServerHandle {
    shutdown_tx: Option<watch::Sender<bool>>,
    join: Option<thread::JoinHandle<()>>,
    state_path: PathBuf,
}

impl WsServerHandle {
    pub fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }
        let mut remove_state_path = true;
        if let Some(join) = self.join.take() {
            match join_thread_with_timeout_or_reap(join, WS_SHUTDOWN_TIMEOUT, "ws server thread") {
                ThreadJoinOutcome::Joined => {}
                ThreadJoinOutcome::ReapingInBackground => {
                    warn!("WS server did not stop within shutdown timeout");
                    remove_state_path = false;
                }
            }
        }
        if remove_state_path && !self.state_path.as_os_str().is_empty() {
            let _ = std::fs::remove_file(&self.state_path);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum WsServerError {
    #[error("WS server disabled")]
    Disabled,
    #[error("Invalid listen address: {message}")]
    InvalidListen { message: String },
    #[error("WS server I/O error ({operation}): {source}")]
    Io {
        operation: &'static str,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Clone)]
struct WsState {
    core: Arc<RpcCore>,
    ws_limits: Arc<Semaphore>,
    ws_queue_capacity: usize,
    shutdown_rx: watch::Receiver<bool>,
    auth_token: String,
    ws_url: String,
}

pub(crate) fn start_ws_server(
    core: Arc<RpcCore>,
    shutdown_flag: Arc<AtomicBool>,
    config: WsConfig,
) -> Result<WsServerHandle, WsServerError> {
    if !config.enabled {
        return Err(WsServerError::Disabled);
    }

    let (listener, local_addr) = bind_listener(&config)?;
    let auth_token = generate_ws_auth_token();
    let ws_url = format_ws_url(&local_addr, &auth_token);
    let ui_url = format_ui_url(&local_addr, &ws_url);
    let listen_addr = local_addr.to_string();
    if let Err(err) = write_state_file(&config.state_path, &ws_url, &ui_url, &listen_addr) {
        warn!(error = %err, "Failed to write WS state file");
    }

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let state = Arc::new(WsState {
        core,
        ws_limits: Arc::new(Semaphore::new(config.max_connections)),
        ws_queue_capacity: config.ws_queue_capacity,
        shutdown_rx: shutdown_rx.clone(),
        auth_token,
        ws_url,
    });

    let state_path = config.state_path.clone();
    let shutdown_tx_for_thread = shutdown_tx.clone();
    let dispatch = tracing::dispatcher::get_default(std::clone::Clone::clone);

    let join = thread::Builder::new()
        .name("agent-tui-ws".to_string())
        .spawn(move || {
            tracing::dispatcher::with_default(&dispatch, || {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build();
                let runtime = match runtime {
                    Ok(rt) => rt,
                    Err(err) => {
                        error!(error = %err, "Failed to build WS runtime");
                        let _ = std::fs::remove_file(&state_path);
                        return;
                    }
                };

                let listener = {
                    let _runtime_guard = runtime.enter();
                    let listener = match TcpListener::from_std(listener) {
                        Ok(l) => l,
                        Err(err) => {
                            error!(error = %err, "Failed to create async listener");
                            return;
                        }
                    };
                    info!(listen = %listen_addr, ui_path = "/ui", ws_path = "/ws", "WS server listening");
                    listener
                };

                runtime.block_on(async move {
                    let app = build_router(state.clone());
                    let mut shutdown_rx_server = shutdown_rx.clone();
                    let mut shutdown_rx_wait = shutdown_rx.clone();

                    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
                        let _ = shutdown_rx_server.changed().await;
                    });
                    let mut server_task = tokio::spawn(async move { server.await });

                    let shutdown_task = tokio::spawn(async move {
                        while !shutdown_flag.load(Ordering::Relaxed) {
                            tokio::time::sleep(Duration::from_millis(200)).await;
                        }
                        let _ = shutdown_tx_for_thread.send(true);
                    });

                    tokio::select! {
                        join_result = &mut server_task => {
                            if let Err(err) = join_result {
                                error!(error = %err, "WS server task failed");
                            }
                        }
                        changed = shutdown_rx_wait.changed() => {
                            if changed.is_err() {
                                warn!("WS shutdown channel closed");
                            }
                            match tokio::time::timeout(WS_SHUTDOWN_TIMEOUT, &mut server_task).await {
                                Ok(join_result) => {
                                    if let Err(err) = join_result {
                                        error!(error = %err, "WS server task failed");
                                    }
                                }
                                Err(_) => {
                                    warn!(
                                        timeout_ms = WS_SHUTDOWN_TIMEOUT.as_millis(),
                                        "WS server shutdown timed out; aborting"
                                    );
                                    server_task.abort();
                                }
                            }
                        }
                    }
                    shutdown_task.abort();
                    let _ = std::fs::remove_file(state_path);
                });
            });
        })
        .map_err(|e| WsServerError::Io {
            operation: "spawn ws thread",
            source: e,
        })?;

    Ok(WsServerHandle {
        shutdown_tx: Some(shutdown_tx),
        join: Some(join),
        state_path: config.state_path,
    })
}

fn build_router(state: Arc<WsState>) -> axum::Router {
    axum::Router::new()
        .route("/", get(ui_root_handler))
        .route("/ui", get(ui_index_handler))
        .route("/app.js", get(ui_app_js_handler))
        .route("/styles.css", get(ui_styles_handler))
        .route("/xterm.css", get(ui_xterm_handler))
        .route("/ws", get(ws_handler))
        .route("/api/v1/stream", get(ws_handler))
        .with_state(state)
}

async fn ui_root_handler(State(state): State<Arc<WsState>>) -> Response {
    let ws = encode_url_query_value(&state.ws_url);
    Redirect::temporary(&format!("/ui?ws={ws}")).into_response()
}

async fn ui_index_handler() -> Response {
    Html(UI_INDEX_HTML).into_response()
}

async fn ui_app_js_handler() -> Response {
    (
        [("content-type", "application/javascript; charset=utf-8")],
        UI_APP_JS,
    )
        .into_response()
}

async fn ui_styles_handler() -> Response {
    ([("content-type", "text/css; charset=utf-8")], UI_STYLES_CSS).into_response()
}

async fn ui_xterm_handler() -> Response {
    ([("content-type", "text/css; charset=utf-8")], UI_XTERM_CSS).into_response()
}

#[derive(Debug, Default, serde::Deserialize)]
struct WsAuthQuery {
    token: Option<String>,
}

async fn ws_handler(
    State(state): State<Arc<WsState>>,
    Query(query): Query<WsAuthQuery>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if query.token.as_deref() != Some(state.auth_token.as_str()) {
        let response = RpcResponse::error(0, -32001, "unauthorized");
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            serde_json::to_string(&response)
                .unwrap_or_else(|_| "{\"error\":\"unauthorized\"}".to_string()),
        )
            .into_response();
    }

    if !origin_matches_ws_url(headers.get(ORIGIN), &state.ws_url) {
        let response = RpcResponse::error(0, -32003, "forbidden origin");
        return (
            axum::http::StatusCode::FORBIDDEN,
            serde_json::to_string(&response)
                .unwrap_or_else(|_| "{\"error\":\"forbidden origin\"}".to_string()),
        )
            .into_response();
    }

    let permit = match state.ws_limits.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            let response = RpcResponse::error(0, -32000, "too many websocket connections");
            return (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                serde_json::to_string(&response)
                    .unwrap_or_else(|_| "{\"error\":\"busy\"}".to_string()),
            )
                .into_response();
        }
    };

    let ctx = WsContext {
        state,
        _permit: permit,
    };

    ws.on_upgrade(move |socket| async move {
        handle_ws(socket, ctx).await;
    })
    .into_response()
}

struct WsContext {
    state: Arc<WsState>,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

struct ChannelWriter {
    tx: mpsc::Sender<String>,
}

impl RpcResponseWriter for ChannelWriter {
    fn write_response(&mut self, response: &RpcResponse) -> Result<(), RpcCoreError> {
        let payload = serde_json::to_string(response)
            .map_err(|err| RpcCoreError::Other(format!("failed to serialize response: {err}")))?;
        self.tx
            .blocking_send(payload)
            .map_err(|_| RpcCoreError::ConnectionClosed)
    }
}

async fn handle_ws(mut socket: WebSocket, ctx: WsContext) {
    let WsContext {
        state,
        _permit: _permit_guard,
    } = ctx;

    let mut shutdown_rx = state.shutdown_rx.clone();
    let mut parse_errors = 0u8;

    loop {
        tokio::select! {
            changed = shutdown_rx.changed() => {
                if changed.is_err() {
                    warn!("WS shutdown channel closed");
                }
                let _ = socket.send(Message::Close(None)).await;
                break;
            }
            msg = tokio::time::timeout(WS_RECV_TIMEOUT, socket.recv()) => {
                let Some(msg) = (match msg {
                    Ok(value) => value,
                    Err(_) => {
                        let _ = socket.send(Message::Close(None)).await;
                        break;
                    }
                }) else {
                    break;
                };
                let msg = match msg {
                    Ok(msg) => msg,
                    Err(_) => break,
                };

                match msg {
                    Message::Text(text) => {
                        let request: RpcRequest = match serde_json::from_str(&text) {
                            Ok(req) => req,
                            Err(err) => {
                                let response = RpcResponse::error(0, -32700, &format!("Parse error: {err}"));
                                if send_rpc_response(&mut socket, &response).await.is_err() {
                                    break;
                                }
                                parse_errors = parse_errors.saturating_add(1);
                                if parse_errors >= WS_MAX_PARSE_ERRORS {
                                    let _ = socket.send(Message::Close(Some(axum::extract::ws::CloseFrame {
                                        code: close_code::POLICY,
                                        reason: "too many parse errors".into(),
                                    }))).await;
                                    break;
                                }
                                continue;
                            }
                        };
                        parse_errors = 0;

                        if let Some(kind) = RpcCore::stream_kind_for_method(&request.method) {
                            if run_stream_connection(&state, &mut socket, request, kind).await.is_err() {
                                break;
                            }
                            break;
                        }

                        let response = state.core.route(request);
                        if send_rpc_response(&mut socket, &response).await.is_err() {
                            break;
                        }
                    }
                    Message::Binary(_) => {
                        let _ = socket.send(Message::Close(Some(axum::extract::ws::CloseFrame {
                            code: close_code::PROTOCOL,
                            reason: "binary frames are not supported".into(),
                        }))).await;
                        break;
                    }
                    Message::Close(_) => break,
                    Message::Ping(payload) => {
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Message::Pong(_) => {}
                }
            }
        }
    }
}

async fn run_stream_connection(
    state: &Arc<WsState>,
    socket: &mut WebSocket,
    request: RpcRequest,
    kind: crate::app::daemon::rpc_core::StreamKind,
) -> Result<(), ()> {
    let (tx, rx) = mpsc::channel::<String>(state.ws_queue_capacity);
    let core = Arc::clone(&state.core);
    let stream_cancelled = Arc::new(AtomicBool::new(false));
    let stream_cancelled_for_task = Arc::clone(&stream_cancelled);
    let mut rx = Some(rx);

    let mut stream_task = tokio::task::spawn_blocking(move || {
        let mut writer = ChannelWriter { tx };
        let _ = core.handle_stream(
            &mut writer,
            request,
            kind,
            Some(stream_cancelled_for_task.as_ref()),
        );
    });

    while let Some(payload) = match rx.as_mut() {
        Some(rx_ref) => rx_ref.recv().await,
        None => None,
    } {
        let send = tokio::time::timeout(WS_SEND_TIMEOUT, socket.send(Message::Text(payload))).await;
        if send.is_err() || send.ok().is_some_and(|result| result.is_err()) {
            let _ = cancel_stream_task(&stream_cancelled, &mut rx, &mut stream_task).await;
            return Err(());
        }
    }

    cancel_stream_task(&stream_cancelled, &mut rx, &mut stream_task).await
}

async fn cancel_stream_task(
    stream_cancelled: &Arc<AtomicBool>,
    rx: &mut Option<mpsc::Receiver<String>>,
    stream_task: &mut tokio::task::JoinHandle<()>,
) -> Result<(), ()> {
    stream_cancelled.store(true, Ordering::Relaxed);
    let _ = rx.take();
    wait_for_stream_task(stream_task).await
}

async fn wait_for_stream_task(stream_task: &mut tokio::task::JoinHandle<()>) -> Result<(), ()> {
    match tokio::time::timeout(WS_STREAM_TASK_SHUTDOWN_TIMEOUT, &mut *stream_task).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(err)) => {
            warn!(error = %err, "WS stream task failed");
            Err(())
        }
        Err(_) => {
            warn!(
                timeout_ms = WS_STREAM_TASK_SHUTDOWN_TIMEOUT.as_millis(),
                "WS stream task shutdown timed out; aborting"
            );
            stream_task.abort();
            Err(())
        }
    }
}

async fn send_rpc_response(socket: &mut WebSocket, response: &RpcResponse) -> Result<(), ()> {
    let payload = serde_json::to_string(response).map_err(|_| ())?;
    let send = tokio::time::timeout(WS_SEND_TIMEOUT, socket.send(Message::Text(payload))).await;
    match send {
        Ok(result) => result.map_err(|_| ()),
        Err(_) => Err(()),
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn env_bool(key: &str) -> Option<bool> {
    std::env::var(key).ok().and_then(|value| parse_bool(&value))
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn bind_listener(config: &WsConfig) -> Result<(std::net::TcpListener, SocketAddr), WsServerError> {
    let mut addrs = config
        .listen
        .to_socket_addrs()
        .map_err(|e| WsServerError::InvalidListen {
            message: e.to_string(),
        })?;
    let addr = addrs.next().ok_or_else(|| WsServerError::InvalidListen {
        message: "no resolved address".to_string(),
    })?;

    if !config.allow_remote && !addr.ip().is_loopback() {
        return Err(WsServerError::InvalidListen {
            message: "refusing to bind non-loopback address without AGENT_TUI_WS_ALLOW_REMOTE=1"
                .to_string(),
        });
    }

    let listener = std::net::TcpListener::bind(addr).map_err(|e| WsServerError::Io {
        operation: "bind",
        source: e,
    })?;
    listener
        .set_nonblocking(true)
        .map_err(|e| WsServerError::Io {
            operation: "set non-blocking",
            source: e,
        })?;
    let local_addr = listener.local_addr().map_err(|e| WsServerError::Io {
        operation: "read local address",
        source: e,
    })?;
    Ok((listener, local_addr))
}

fn format_ws_url(addr: &SocketAddr, auth_token: &str) -> String {
    let host = match addr.ip() {
        std::net::IpAddr::V4(ip) => ip.to_string(),
        std::net::IpAddr::V6(ip) => format!("[{ip}]"),
    };
    format!("ws://{}:{}/ws?token={}", host, addr.port(), auth_token)
}

fn format_ui_url(addr: &SocketAddr, ws_url: &str) -> String {
    let host = match addr.ip() {
        std::net::IpAddr::V4(ip) => ip.to_string(),
        std::net::IpAddr::V6(ip) => format!("[{ip}]"),
    };
    let ws = encode_url_query_value(ws_url);
    format!("http://{}:{}/ui?ws={}", host, addr.port(), ws)
}

fn encode_url_query_value(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

fn urls_share_origin(left: &url::Url, right: &url::Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}

fn allowed_browser_origin(ws_url: &str) -> Option<String> {
    let ws_url = url::Url::parse(ws_url).ok()?;
    let scheme = if ws_url.scheme() == "wss" {
        "https"
    } else {
        "http"
    };
    let host = ws_url.host_str()?;
    let mut origin = format!("{scheme}://{host}");
    if let Some(port) = ws_url.port() {
        origin.push(':');
        origin.push_str(&port.to_string());
    }
    Some(origin)
}

fn origin_matches_ws_url(origin: Option<&axum::http::HeaderValue>, ws_url: &str) -> bool {
    let Some(origin) = origin else {
        return true;
    };
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    let Ok(origin_url) = url::Url::parse(origin) else {
        return false;
    };
    let Some(allowed_origin) = allowed_browser_origin(ws_url) else {
        return false;
    };
    let Ok(allowed_origin) = url::Url::parse(&allowed_origin) else {
        return false;
    };
    urls_share_origin(&origin_url, &allowed_origin)
}

fn generate_ws_auth_token() -> String {
    Uuid::new_v4().simple().to_string()
}

fn join_thread_with_timeout_or_reap(
    handle: thread::JoinHandle<()>,
    timeout: Duration,
    thread_label: &'static str,
) -> ThreadJoinOutcome {
    let deadline = Instant::now() + timeout;
    while !handle.is_finished() {
        if Instant::now() >= deadline {
            warn!(
                thread = thread_label,
                timeout_ms = timeout.as_millis(),
                "Timed out joining thread; handing ownership to background reaper"
            );
            return spawn_join_reaper(handle, thread_label);
        }
        thread::park_timeout(WS_JOIN_POLL_INTERVAL);
    }
    let _ = handle.join();
    ThreadJoinOutcome::Joined
}

fn spawn_join_reaper(
    handle: thread::JoinHandle<()>,
    thread_label: &'static str,
) -> ThreadJoinOutcome {
    let handle_cell = Arc::new(std::sync::Mutex::new(Some(handle)));
    let handle_for_thread = Arc::clone(&handle_cell);
    match thread::Builder::new()
        .name("agent-tui-ws-reaper".to_string())
        .spawn(move || {
            let Some(handle) = handle_for_thread
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take()
            else {
                return;
            };

            if handle.join().is_err() {
                warn!(
                    thread = thread_label,
                    "Background reaper observed thread panic"
                );
            }
        }) {
        Ok(_) => ThreadJoinOutcome::ReapingInBackground,
        Err(err) => {
            warn!(
                thread = thread_label,
                error = %err,
                "Failed to spawn background join reaper; joining synchronously"
            );
            if let Some(handle) = handle_cell
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take()
            {
                let _ = handle.join();
            }
            ThreadJoinOutcome::Joined
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct WsStateFile<'a> {
    pid: u32,
    ws_url: &'a str,
    ui_url: &'a str,
    listen: &'a str,
    started_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    process_started_at: Option<u64>,
}

fn write_state_file(
    path: &StdPath,
    ws_url: &str,
    ui_url: &str,
    listen: &str,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let started_at = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let identity = current_process_identity();

    let payload = WsStateFile {
        pid: identity.pid,
        ws_url,
        ui_url,
        listen,
        started_at,
        process_started_at: identity.started_at,
    };

    let tmp_path = path.with_extension("tmp");
    std::fs::write(
        &tmp_path,
        serde_json::to_vec_pretty(&payload).unwrap_or_default(),
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp_path, path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

fn default_state_path() -> PathBuf {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    home.join(".agent-tui").join("api.json")
}

#[cfg(test)]
#[path = "ws_server_tests.rs"]
mod tests;
