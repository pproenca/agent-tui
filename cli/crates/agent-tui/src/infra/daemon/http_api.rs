use axum::Json;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::get;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use serde_json::json;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::{Path as StdPath, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime};
use tokio::net::TcpListener;
use tokio::sync::{Semaphore, mpsc, watch};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, warn};

use crate::domain::session_types::SessionInfo;
use crate::infra::daemon::SessionManager;
use crate::usecases::ports::{SessionHandle, SessionRepository, StreamCursor};

const API_VERSION: &str = "1";
const DEFAULT_API_LISTEN: &str = "127.0.0.1:0";
const DEFAULT_MAX_WS_CONNECTIONS: usize = 32;
const DEFAULT_WS_QUEUE_CAPACITY: usize = 128;
const LIVE_PREVIEW_STREAM_MAX_CHUNK_BYTES: usize = 64 * 1024;
const LIVE_PREVIEW_STREAM_MAX_TICK_BYTES: usize = 256 * 1024;
const LIVE_PREVIEW_STREAM_HEARTBEAT: Duration = Duration::from_secs(5);
const OUTPUT_FRAME_DATA: u8 = 0x01;
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

#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub enabled: bool,
    pub listen: String,
    pub allow_remote: bool,
    pub token: Option<String>,
    pub state_path: PathBuf,
    pub max_ws_connections: usize,
    pub ws_queue_capacity: usize,
}

impl ApiConfig {
    pub fn from_env() -> Self {
        let enabled = env_bool("AGENT_TUI_API_DISABLED")
            .map(|v| !v)
            .unwrap_or(true);
        let listen = std::env::var("AGENT_TUI_API_LISTEN")
            .unwrap_or_else(|_| DEFAULT_API_LISTEN.to_string());
        let allow_remote = env_bool("AGENT_TUI_API_ALLOW_REMOTE").unwrap_or(false);
        let token = match std::env::var("AGENT_TUI_API_TOKEN") {
            Ok(value) => {
                let trimmed = value.trim();
                if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
            Err(_) => Some(generate_token()),
        };
        let state_path = std::env::var("AGENT_TUI_API_STATE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_state_path());
        let max_ws_connections = std::env::var("AGENT_TUI_API_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_MAX_WS_CONNECTIONS);
        let ws_queue_capacity = std::env::var("AGENT_TUI_API_WS_QUEUE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_WS_QUEUE_CAPACITY);

        Self {
            enabled,
            listen,
            allow_remote,
            token,
            state_path,
            max_ws_connections,
            ws_queue_capacity,
        }
    }
}

pub struct ApiServerHandle {
    shutdown_tx: Option<watch::Sender<bool>>,
    join: Option<thread::JoinHandle<()>>,
    state_path: PathBuf,
}

impl ApiServerHandle {
    pub fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
        if !self.state_path.as_os_str().is_empty() {
            let _ = std::fs::remove_file(&self.state_path);
        }
    }
}

#[derive(Debug)]
pub enum ApiServerError {
    Disabled,
    Bind(String),
    InvalidListen(String),
}

#[derive(Clone)]
struct ApiState {
    session_manager: Arc<SessionManager>,
    token: Option<String>,
    api_version: &'static str,
    daemon_version: String,
    daemon_commit: String,
    start_time: Instant,
    http_url: String,
    ws_url: String,
    ws_limits: Arc<Semaphore>,
    ws_queue_capacity: usize,
}

#[derive(Deserialize)]
struct TokenQuery {
    token: Option<String>,
}

#[derive(Deserialize)]
struct StreamQuery {
    token: Option<String>,
    session: Option<String>,
    encoding: Option<String>,
}

#[derive(Deserialize)]
struct UiQuery {
    api: Option<String>,
    ws: Option<String>,
    auto: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputEncoding {
    Base64,
    Binary,
}

enum WsPayload {
    Text(String),
    Binary(Vec<u8>),
}

pub fn start_api_server(
    session_manager: Arc<SessionManager>,
    shutdown_flag: Arc<AtomicBool>,
    config: ApiConfig,
) -> Result<ApiServerHandle, ApiServerError> {
    if !config.enabled {
        return Err(ApiServerError::Disabled);
    }

    let (listener, local_addr) = bind_listener(&config)?;

    let http_url = format_http_url(&local_addr);
    let ws_url = format_ws_url(&local_addr);
    let listen_addr = local_addr.to_string();
    if let Err(err) = write_state_file(
        &config.state_path,
        &http_url,
        &ws_url,
        &listen_addr,
        config.token.as_deref(),
    ) {
        warn!(error = %err, "Failed to write API state file");
    }

    let state = Arc::new(ApiState {
        session_manager,
        token: config.token.clone(),
        api_version: API_VERSION,
        daemon_version: env!("AGENT_TUI_VERSION").to_string(),
        daemon_commit: env!("AGENT_TUI_GIT_SHA").to_string(),
        start_time: Instant::now(),
        http_url: http_url.clone(),
        ws_url: ws_url.clone(),
        ws_limits: Arc::new(Semaphore::new(config.max_ws_connections)),
        ws_queue_capacity: config.ws_queue_capacity,
    });

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let state_path = config.state_path.clone();

    let join = thread::Builder::new()
        .name("agent-tui-api".to_string())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build();
            let runtime = match runtime {
                Ok(rt) => rt,
                Err(err) => {
                    error!(error = %err, "Failed to build API runtime");
                    let _ = std::fs::remove_file(&state_path);
                    return;
                }
            };

            runtime.block_on(async move {
                let app = build_router(state.clone());
                let listener = match TcpListener::from_std(listener) {
                    Ok(l) => l,
                    Err(err) => {
                        error!(error = %err, "Failed to create async listener");
                        return;
                    }
                };
                info!(url = %http_url, "API server listening");
                let server = axum::serve(listener, app).with_graceful_shutdown(async move {
                    let _ = shutdown_rx.changed().await;
                });
                if let Err(err) = server.await {
                    error!(error = %err, "API server failed");
                }
                let _ = std::fs::remove_file(state_path);
            });
        })
        .map_err(|e| ApiServerError::Bind(e.to_string()))?;

    let shutdown_watcher = shutdown_flag;
    let shutdown_tx_clone = shutdown_tx.clone();
    thread::spawn(move || {
        while !shutdown_watcher.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(200));
        }
        let _ = shutdown_tx_clone.send(true);
    });

    Ok(ApiServerHandle {
        shutdown_tx: Some(shutdown_tx),
        join: Some(join),
        state_path: config.state_path,
    })
}

fn build_router(state: Arc<ApiState>) -> axum::Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    axum::Router::new()
        .route("/", get(ui_root_handler))
        .route("/ui", get(ui_index_handler))
        .route("/app.js", get(ui_app_js_handler))
        .route("/styles.css", get(ui_styles_handler))
        .route("/xterm.css", get(ui_xterm_handler))
        .route("/api/v1/version", get(version_handler))
        .route("/api/v1/health", get(health_handler))
        .route("/api/v1/sessions", get(sessions_handler))
        .route("/api/v1/sessions/:id/snapshot", get(snapshot_handler))
        .route("/api/v1/stream", get(ws_handler))
        .layer(cors)
        .with_state(state)
}

async fn version_handler(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Query(query): Query<TokenQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, query.token.as_deref()) {
        return *resp;
    }
    Json(json!({
        "api_version": state.api_version,
        "daemon_version": state.daemon_version,
        "daemon_commit": state.daemon_commit
    }))
    .into_response()
}

async fn ui_root_handler() -> Response {
    Redirect::temporary("/ui").into_response()
}

async fn ui_index_handler(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<UiQuery>,
) -> Response {
    let has_params = query.api.is_some() || query.ws.is_some() || query.auto.is_some();
    if !has_params {
        let mut url = String::from("/ui?api=");
        url.push_str(&state.http_url);
        url.push_str("&ws=");
        url.push_str(&state.ws_url);
        url.push_str("&session=active&encoding=binary&auto=1");
        if let Some(token) = state.token.as_deref() {
            url.push_str("&token=");
            url.push_str(token);
        }
        return Redirect::temporary(&url).into_response();
    }
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

async fn health_handler(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Query(query): Query<TokenQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, query.token.as_deref()) {
        return *resp;
    }
    let session_count = state.session_manager.session_count();
    let uptime_ms = state.start_time.elapsed().as_millis() as u64;
    Json(json!({
        "status": "healthy",
        "pid": std::process::id(),
        "uptime_ms": uptime_ms,
        "session_count": session_count,
        "api_version": state.api_version,
        "daemon_version": state.daemon_version,
        "daemon_commit": state.daemon_commit
    }))
    .into_response()
}

async fn sessions_handler(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Query(query): Query<TokenQuery>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, query.token.as_deref()) {
        return *resp;
    }
    let sessions = state.session_manager.list();
    let active = state.session_manager.active_session_id();
    let payload = json!({
        "active": active.map(|id| id.as_str().to_string()),
        "sessions": sessions.into_iter().map(|info| session_info_to_json(&info)).collect::<Vec<_>>()
    });
    Json(payload).into_response()
}

async fn snapshot_handler(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Query(query): Query<TokenQuery>,
    Path(id): Path<String>,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, query.token.as_deref()) {
        return *resp;
    }
    let session_id = if id == "active" {
        None
    } else {
        Some(id.as_str())
    };
    let session = match SessionRepository::resolve(state.session_manager.as_ref(), session_id) {
        Ok(session) => session,
        Err(err) => return error_response(StatusCode::NOT_FOUND, &err.to_string()),
    };
    if let Err(err) = session.update() {
        return error_response(StatusCode::BAD_GATEWAY, &err.to_string());
    }
    let snapshot = session.live_preview_snapshot();
    Json(json!({
        "cols": snapshot.cols,
        "rows": snapshot.rows,
        "init": snapshot.seq
    }))
    .into_response()
}

async fn ws_handler(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Query(query): Query<StreamQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    if let Err(resp) = require_auth(&state, &headers, query.token.as_deref()) {
        return *resp;
    }

    let session_param = query.session.as_deref().filter(|s| *s != "active");
    let output_encoding = match parse_output_encoding(query.encoding.as_deref()) {
        Ok(encoding) => encoding,
        Err(message) => return error_response(StatusCode::BAD_REQUEST, message),
    };
    let session = match SessionRepository::resolve(state.session_manager.as_ref(), session_param) {
        Ok(session) => session,
        Err(err) => return error_response(StatusCode::NOT_FOUND, &err.to_string()),
    };
    let permit = match state.ws_limits.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => return error_response(StatusCode::SERVICE_UNAVAILABLE, "too many connections"),
    };

    let state = state.clone();
    let session_id = session.session_id().to_string();
    let queue_capacity = state.ws_queue_capacity;
    ws.on_upgrade(move |socket| async move {
        handle_ws(
            socket,
            state,
            session,
            session_id,
            permit,
            queue_capacity,
            output_encoding,
        )
        .await;
    })
    .into_response()
}

async fn handle_ws(
    mut socket: WebSocket,
    state: Arc<ApiState>,
    session: SessionHandle,
    session_id: String,
    _permit: tokio::sync::OwnedSemaphorePermit,
    queue_capacity: usize,
    output_encoding: OutputEncoding,
) {
    let hello = json!({
        "event": "hello",
        "api_version": state.api_version,
        "daemon_version": state.daemon_version,
        "daemon_commit": state.daemon_commit,
        "session_id": session_id,
        "output_encoding": match output_encoding {
            OutputEncoding::Base64 => "base64",
            OutputEncoding::Binary => "binary",
        }
    });
    if socket.send(Message::Text(hello.to_string())).await.is_err() {
        return;
    }

    let (tx, mut rx) = mpsc::channel::<WsPayload>(queue_capacity);
    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_thread = Arc::clone(&stop);
    let session_for_thread = Arc::clone(&session);
    thread::Builder::new()
        .name(format!("api-stream-{session_id}"))
        .spawn(move || {
            stream_live_preview(session_for_thread, tx, stop_for_thread, output_encoding);
        })
        .ok();

    loop {
        tokio::select! {
            maybe = rx.recv() => {
                match maybe {
                    Some(WsPayload::Text(payload)) => {
                        if socket.send(Message::Text(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(WsPayload::Binary(payload)) => {
                        if socket.send(Message::Binary(payload)).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                    None => break,
                }
            }
        }
    }

    stop.store(true, Ordering::Relaxed);
}

fn stream_live_preview(
    session: SessionHandle,
    sender: mpsc::Sender<WsPayload>,
    stop: Arc<AtomicBool>,
    output_encoding: OutputEncoding,
) {
    let start_time = Instant::now();

    if let Err(err) = session.update() {
        let _ = sender.blocking_send(WsPayload::Text(error_event(&err.to_string()).to_string()));
        return;
    }

    let snapshot = session.live_preview_snapshot();
    let session_id = session.session_id().to_string();
    if sender
        .blocking_send(WsPayload::Text(
            json!({
                "event": "ready",
                "session_id": session_id,
                "cols": snapshot.cols,
                "rows": snapshot.rows
            })
            .to_string(),
        ))
        .is_err()
    {
        return;
    }

    if sender
        .blocking_send(WsPayload::Text(
            json!({
                "event": "init",
                "time": start_time.elapsed().as_secs_f64(),
                "cols": snapshot.cols,
                "rows": snapshot.rows,
                "init": snapshot.seq
            })
            .to_string(),
        ))
        .is_err()
    {
        return;
    }

    let subscription = session.stream_subscribe();
    let mut cursor = StreamCursor {
        seq: snapshot.stream_seq,
    };
    let mut last_size = (snapshot.cols, snapshot.rows);

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        let mut budget = LIVE_PREVIEW_STREAM_MAX_TICK_BYTES;
        let mut sent_any = false;

        loop {
            if budget == 0 || stop.load(Ordering::Relaxed) {
                break;
            }

            let max_chunk = budget.min(LIVE_PREVIEW_STREAM_MAX_CHUNK_BYTES);
            let read = match session.stream_read(&mut cursor, max_chunk, 0) {
                Ok(read) => read,
                Err(err) => {
                    let _ = sender
                        .blocking_send(WsPayload::Text(error_event(&err.to_string()).to_string()));
                    return;
                }
            };

            if read.dropped_bytes > 0 {
                if sender
                    .blocking_send(WsPayload::Text(
                        json!({
                            "event": "dropped",
                            "time": start_time.elapsed().as_secs_f64(),
                            "dropped_bytes": read.dropped_bytes
                        })
                        .to_string(),
                    ))
                    .is_err()
                {
                    return;
                }

                if let Err(err) = session.update() {
                    let _ = sender
                        .blocking_send(WsPayload::Text(error_event(&err.to_string()).to_string()));
                    return;
                }
                let snapshot = session.live_preview_snapshot();
                if sender
                    .blocking_send(WsPayload::Text(
                        json!({
                            "event": "init",
                            "time": start_time.elapsed().as_secs_f64(),
                            "cols": snapshot.cols,
                            "rows": snapshot.rows,
                            "init": snapshot.seq
                        })
                        .to_string(),
                    ))
                    .is_err()
                {
                    return;
                }
                last_size = (snapshot.cols, snapshot.rows);
                cursor.seq = read.latest_cursor.seq;
                sent_any = true;
                break;
            }

            if !read.data.is_empty() {
                match output_encoding {
                    OutputEncoding::Base64 => {
                        let data_b64 = STANDARD.encode(&read.data);
                        if sender
                            .blocking_send(WsPayload::Text(
                                json!({
                                    "event": "output",
                                    "time": start_time.elapsed().as_secs_f64(),
                                    "data_b64": data_b64
                                })
                                .to_string(),
                            ))
                            .is_err()
                        {
                            return;
                        }
                    }
                    OutputEncoding::Binary => {
                        let mut frame = Vec::with_capacity(1 + read.data.len());
                        frame.push(OUTPUT_FRAME_DATA);
                        frame.extend_from_slice(&read.data);
                        if sender.blocking_send(WsPayload::Binary(frame)).is_err() {
                            return;
                        }
                    }
                }
                sent_any = true;
                budget = budget.saturating_sub(read.data.len());
                if read.closed {
                    let _ = sender.blocking_send(WsPayload::Text(
                        json!({
                            "event": "closed",
                            "time": start_time.elapsed().as_secs_f64()
                        })
                        .to_string(),
                    ));
                    return;
                }
                continue;
            }

            if read.closed {
                let _ = sender.blocking_send(WsPayload::Text(
                    json!({
                        "event": "closed",
                        "time": start_time.elapsed().as_secs_f64()
                    })
                    .to_string(),
                ));
                return;
            }

            break;
        }

        let size = session.size();
        if size != last_size {
            if sender
                .blocking_send(WsPayload::Text(
                    json!({
                        "event": "resize",
                        "time": start_time.elapsed().as_secs_f64(),
                        "cols": size.0,
                        "rows": size.1
                    })
                    .to_string(),
                ))
                .is_err()
            {
                return;
            }
            last_size = size;
            sent_any = true;
        }

        if sent_any && budget == 0 {
            continue;
        }

        if !subscription.wait(Some(LIVE_PREVIEW_STREAM_HEARTBEAT))
            && sender
                .blocking_send(WsPayload::Text(
                    json!({
                        "event": "heartbeat",
                        "time": start_time.elapsed().as_secs_f64()
                    })
                    .to_string(),
                ))
                .is_err()
        {
            return;
        }
    }
}

fn require_auth(
    state: &ApiState,
    headers: &HeaderMap,
    query_token: Option<&str>,
) -> Result<(), Box<Response>> {
    let Some(expected) = state.token.as_deref() else {
        return Ok(());
    };
    let mut candidate = query_token.map(str::to_string);
    if candidate.is_none() {
        if let Some(value) = headers.get("x-agent-tui-token") {
            candidate = value.to_str().ok().map(|v| v.to_string());
        }
    }
    if candidate.is_none() {
        if let Some(value) = headers.get(axum::http::header::AUTHORIZATION) {
            if let Ok(auth) = value.to_str() {
                if let Some(token) = auth.strip_prefix("Bearer ") {
                    candidate = Some(token.trim().to_string());
                }
            }
        }
    }

    if candidate.as_deref() == Some(expected) {
        Ok(())
    } else {
        Err(Box::new(error_response(
            StatusCode::UNAUTHORIZED,
            "invalid token",
        )))
    }
}

fn error_event(message: &str) -> serde_json::Value {
    json!({ "event": "error", "message": message })
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

fn parse_output_encoding(value: Option<&str>) -> Result<OutputEncoding, &'static str> {
    match value.unwrap_or("base64").to_ascii_lowercase().as_str() {
        "base64" => Ok(OutputEncoding::Base64),
        "binary" => Ok(OutputEncoding::Binary),
        _ => Err("invalid encoding; use 'base64' or 'binary'"),
    }
}

fn bind_listener(
    config: &ApiConfig,
) -> Result<(std::net::TcpListener, SocketAddr), ApiServerError> {
    let mut addrs = config
        .listen
        .to_socket_addrs()
        .map_err(|e| ApiServerError::InvalidListen(e.to_string()))?;
    let addr = addrs
        .next()
        .ok_or_else(|| ApiServerError::InvalidListen("no resolved address".to_string()))?;

    if !config.allow_remote && !addr.ip().is_loopback() {
        return Err(ApiServerError::InvalidListen(
            "refusing to bind non-loopback address without AGENT_TUI_API_ALLOW_REMOTE=1"
                .to_string(),
        ));
    }

    let listener =
        std::net::TcpListener::bind(addr).map_err(|e| ApiServerError::Bind(e.to_string()))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| ApiServerError::Bind(e.to_string()))?;
    let local_addr = listener
        .local_addr()
        .map_err(|e| ApiServerError::Bind(e.to_string()))?;
    Ok((listener, local_addr))
}

fn format_http_url(addr: &SocketAddr) -> String {
    let host = match addr.ip() {
        std::net::IpAddr::V4(ip) => ip.to_string(),
        std::net::IpAddr::V6(ip) => format!("[{ip}]"),
    };
    format!("http://{}:{}/", host, addr.port())
}

fn format_ws_url(addr: &SocketAddr) -> String {
    let host = match addr.ip() {
        std::net::IpAddr::V4(ip) => ip.to_string(),
        std::net::IpAddr::V6(ip) => format!("[{ip}]"),
    };
    format!("ws://{}:{}/api/v1/stream", host, addr.port())
}

fn write_state_file(
    path: &StdPath,
    http_url: &str,
    ws_url: &str,
    listen: &str,
    token: Option<&str>,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let started_at = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let payload = json!({
        "pid": std::process::id(),
        "http_url": http_url,
        "ws_url": ws_url,
        "listen": listen,
        "token": token,
        "api_version": API_VERSION,
        "started_at": started_at
    });
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
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".agent-tui").join("api.json")
}

fn env_bool(key: &str) -> Option<bool> {
    std::env::var(key)
        .ok()
        .and_then(|value| match value.to_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
}

fn generate_token() -> String {
    let bytes: [u8; 16] = rand::random();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn session_info_to_json(info: &SessionInfo) -> serde_json::Value {
    json!({
        "id": info.id.as_str(),
        "command": info.command,
        "pid": info.pid,
        "running": info.running,
        "created_at": info.created_at,
        "size": { "cols": info.size.0, "rows": info.size.1 }
    })
}
