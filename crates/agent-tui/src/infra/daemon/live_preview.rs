use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use axum::{
    Json, Router,
    extract::{Query, State, ws},
    http::{StatusCode, header},
    response::{Html, IntoResponse},
    routing::get,
};
use futures_util::{StreamExt, sink, stream};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::{broadcast, watch};
use tokio::time::MissedTickBehavior;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{info, warn};

use crate::domain::session_types::SessionId;
use crate::domain::{LivePreviewStartOutput, LivePreviewStatusOutput, LivePreviewStopOutput};
use crate::usecases::ports::{
    LivePreviewError, LivePreviewOptions, LivePreviewService, SessionHandle, SessionRepository,
};

const ASCIINEMA_PLAYER_JS: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/live/asciinema-player.min.js"
));
const ASCIINEMA_PLAYER_CSS: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/live/asciinema-player.css"
));
const INDEX_HTML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/live/index.html"
));

const POLL_INTERVAL: Duration = Duration::from_millis(100);
const START_TIMEOUT: Duration = Duration::from_secs(2);

pub struct LivePreviewManager {
    repository: Arc<dyn SessionRepository>,
    state: Mutex<LivePreviewState>,
}

impl LivePreviewManager {
    pub fn new(repository: Arc<dyn SessionRepository>) -> Self {
        Self {
            repository,
            state: Mutex::new(LivePreviewState { server: None }),
        }
    }
}

impl LivePreviewService for LivePreviewManager {
    fn start(
        &self,
        session: SessionHandle,
        options: LivePreviewOptions,
    ) -> Result<LivePreviewStartOutput, LivePreviewError> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());

        if let Some(server) = state.server.as_ref() {
            if !server.join.is_finished() {
                return Err(LivePreviewError::AlreadyRunning);
            }
        }

        let session_id = session.session_id();
        let handle = spawn_preview_server(
            Arc::clone(&self.repository),
            session_id,
            options.listen_addr,
        )?;
        let output = LivePreviewStartOutput {
            session_id: handle.session_id.clone(),
            listen_addr: handle.listen_addr.to_string(),
        };
        state.server = Some(handle);
        Ok(output)
    }

    fn stop(&self) -> Result<LivePreviewStopOutput, LivePreviewError> {
        let handle = {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            state.server.take()
        };

        let Some(handle) = handle else {
            return Err(LivePreviewError::NotRunning);
        };

        let _ = handle.shutdown.send(true);
        let _ = handle.join.join();

        Ok(LivePreviewStopOutput {
            stopped: true,
            session_id: Some(handle.session_id),
        })
    }

    fn status(&self) -> LivePreviewStatusOutput {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());

        if let Some(server) = state.server.as_ref() {
            if server.join.is_finished() {
                let finished = state.server.take();
                if let Some(finished) = finished {
                    let _ = finished.join.join();
                }
            }
        }

        match state.server.as_ref() {
            Some(server) => LivePreviewStatusOutput {
                running: true,
                session_id: Some(server.session_id.clone()),
                listen_addr: Some(server.listen_addr.to_string()),
            },
            None => LivePreviewStatusOutput {
                running: false,
                session_id: None,
                listen_addr: None,
            },
        }
    }
}

struct LivePreviewState {
    server: Option<LivePreviewServerHandle>,
}

struct LivePreviewServerHandle {
    session_id: SessionId,
    listen_addr: SocketAddr,
    shutdown: watch::Sender<bool>,
    join: thread::JoinHandle<()>,
}

fn spawn_preview_server(
    repository: Arc<dyn SessionRepository>,
    default_session_id: SessionId,
    listen_addr: SocketAddr,
) -> Result<LivePreviewServerHandle, LivePreviewError> {
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<SocketAddr, LivePreviewError>>();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let session_id = default_session_id.clone();
    let server_session_id = default_session_id.clone();
    let shutdown_tx_clone = shutdown_tx.clone();
    let thread_name = format!("live-preview-{}", default_session_id.as_str());
    let join = thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = ready_tx.send(Err(LivePreviewError::BindFailed {
                        addr: listen_addr.to_string(),
                        reason: e.to_string(),
                    }));
                    return;
                }
            };

            runtime.block_on(async move {
                let listener = match tokio::net::TcpListener::bind(listen_addr).await {
                    Ok(listener) => listener,
                    Err(e) => {
                        let _ = ready_tx.send(Err(LivePreviewError::BindFailed {
                            addr: listen_addr.to_string(),
                            reason: e.to_string(),
                        }));
                        return;
                    }
                };

                let actual_addr = listener.local_addr().unwrap_or(listen_addr);
                let _ = ready_tx.send(Ok(actual_addr));

                let server_state = Arc::new(LivePreviewServerState::new(
                    repository,
                    server_session_id,
                    shutdown_tx_clone,
                ));
                let app = build_router(Arc::clone(&server_state));
                let mut shutdown = shutdown_rx.clone();
                info!(listen = %actual_addr, "Live preview server started");

                let _ = axum::serve(listener, app)
                    .with_graceful_shutdown(async move {
                        let _ = shutdown.changed().await;
                    })
                    .await;

                info!(listen = %actual_addr, "Live preview server stopped");
            });
        })
        .map_err(|e| LivePreviewError::BindFailed {
            addr: listen_addr.to_string(),
            reason: e.to_string(),
        })?;

    let actual_addr = match ready_rx.recv_timeout(START_TIMEOUT) {
        Ok(Ok(addr)) => addr,
        Ok(Err(err)) => return Err(err),
        Err(e) => {
            return Err(LivePreviewError::BindFailed {
                addr: listen_addr.to_string(),
                reason: e.to_string(),
            });
        }
    };

    Ok(LivePreviewServerHandle {
        session_id,
        listen_addr: actual_addr,
        shutdown: shutdown_tx,
        join,
    })
}

#[derive(Clone)]
struct LivePreviewServerState {
    repository: Arc<dyn SessionRepository>,
    default_session_id: SessionId,
    sessions: Arc<Mutex<HashMap<String, Arc<SessionStreamState>>>>,
    shutdown: watch::Sender<bool>,
    start_time: Instant,
}

impl LivePreviewServerState {
    fn new(
        repository: Arc<dyn SessionRepository>,
        default_session_id: SessionId,
        shutdown: watch::Sender<bool>,
    ) -> Self {
        Self {
            repository,
            default_session_id,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            shutdown,
            start_time: Instant::now(),
        }
    }

    fn resolve_session_id(&self, requested: Option<&str>) -> Result<SessionId, LivePreviewError> {
        if let Some(requested) = requested {
            return Ok(SessionId::new(requested.to_string()));
        }
        if let Some(active) = self.repository.active_session_id() {
            return Ok(active);
        }
        Ok(self.default_session_id.clone())
    }

    fn list_sessions(
        &self,
    ) -> (
        Vec<crate::domain::session_types::SessionInfo>,
        Option<SessionId>,
    ) {
        let sessions = self.repository.list();
        let active = self.repository.active_session_id();
        (sessions, active)
    }

    fn get_or_create_session(
        &self,
        session_id: &SessionId,
    ) -> Result<Arc<SessionStreamState>, LivePreviewError> {
        if let Some(state) = self
            .sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(session_id.as_str())
        {
            return Ok(Arc::clone(state));
        }

        let session = self.repository.get(session_id.as_str())?;
        if !session.is_running() {
            return Err(LivePreviewError::Session(
                crate::usecases::ports::SessionError::NotFound(format!(
                    "{} (session not running)",
                    session_id.as_str()
                )),
            ));
        }
        let (tx, _) = broadcast::channel(128);
        let (cols, rows) = session.size();
        let snapshot = SnapshotState { size: (cols, rows) };
        let session_state = Arc::new(SessionStreamState {
            session,
            broadcaster: tx,
            snapshot: Arc::new(Mutex::new(snapshot)),
        });

        self.sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(session_id.as_str().to_string(), Arc::clone(&session_state));

        let shutdown = self.shutdown.subscribe();
        let pump_state = Arc::clone(&session_state);
        let start_time = self.start_time;
        tokio::spawn(async move {
            pump_events(pump_state, start_time, shutdown).await;
        });

        Ok(session_state)
    }
}

#[derive(Clone, Copy)]
struct SnapshotState {
    size: (u16, u16),
}

#[derive(Clone)]
struct SessionStreamState {
    session: SessionHandle,
    broadcaster: broadcast::Sender<LiveEvent>,
    snapshot: Arc<Mutex<SnapshotState>>,
}

#[derive(Clone)]
enum LiveEvent {
    Output { time: f64, seq: String },
    Resize { time: f64, cols: u16, rows: u16 },
}

async fn pump_events(
    state: Arc<SessionStreamState>,
    start_time: Instant,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut interval = tokio::time::interval(POLL_INTERVAL);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    break;
                }
            }
            _ = interval.tick() => {
                let session = Arc::clone(&state.session);
                let update_result = tokio::task::spawn_blocking(move || session.update()).await;
                match update_result {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => {
                        warn!(error = %e, "Live preview session update failed");
                        continue;
                    }
                    Err(e) => {
                        warn!(error = %e, "Live preview session update task failed");
                        continue;
                    }
                }

                let (cols, rows) = state.session.size();

                let mut snapshot = state.snapshot.lock().unwrap_or_else(|e| e.into_inner());
                let time = start_time.elapsed().as_secs_f64();

                if snapshot.size != (cols, rows) {
                    let _ = state
                        .broadcaster
                        .send(LiveEvent::Resize { time, cols, rows });
                    snapshot.size = (cols, rows);
                }

                let output = state.session.live_preview_drain_output();
                if !output.seq.is_empty() {
                    if output.dropped_bytes > 0 {
                        warn!(
                            dropped_bytes = output.dropped_bytes,
                            "Live preview output buffer overflow"
                        );
                    }
                    let _ = state
                        .broadcaster
                        .send(LiveEvent::Output { time, seq: output.seq });
                }
            }
        }
    }
}

fn build_router(state: Arc<LivePreviewServerState>) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/sessions", get(sessions_handler))
        .route("/asciinema-player.css", get(css_handler))
        .route("/asciinema-player.min.js", get(js_handler))
        .route("/ws/alis", get(alis_handler))
        .route("/ws/events", get(events_handler))
        .with_state(state)
}

async fn index_handler() -> impl IntoResponse {
    Html(INDEX_HTML)
}

async fn sessions_handler(State(state): State<Arc<LivePreviewServerState>>) -> impl IntoResponse {
    let (sessions, active) = state.list_sessions();
    let payload = json!({
        "active": active.as_ref().map(|id| id.as_str()),
        "sessions": sessions.into_iter().map(|session| {
            json!({
                "id": session.id.as_str(),
                "command": session.command,
                "pid": session.pid,
                "running": session.running,
                "created_at": session.created_at,
                "cols": session.size.0,
                "rows": session.size.1
            })
        }).collect::<Vec<_>>()
    });
    Json(payload)
}

async fn css_handler() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css")], ASCIINEMA_PLAYER_CSS)
}

async fn js_handler() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript")],
        ASCIINEMA_PLAYER_JS,
    )
}

async fn alis_handler(
    ws: ws::WebSocketUpgrade,
    Query(params): Query<SessionParams>,
    State(state): State<Arc<LivePreviewServerState>>,
) -> impl IntoResponse {
    let session_id = match state.resolve_session_id(params.session.as_deref()) {
        Ok(id) => id,
        Err(e) => return (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    };
    let session_state = match state.get_or_create_session(&session_id) {
        Ok(session_state) => session_state,
        Err(e) => return (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    };
    let start_time = state.start_time;

    ws.on_upgrade(move |socket| async move {
        let _ = handle_alis_socket(socket, session_state, start_time).await;
    })
}

async fn handle_alis_socket(
    socket: ws::WebSocket,
    session_state: Arc<SessionStreamState>,
    start_time: Instant,
) -> Result<(), axum::Error> {
    let (sink, stream) = socket.split();
    let drainer = tokio::spawn(stream.map(Ok).forward(sink::drain()));

    let init_message = alis_init_message(&session_state, start_time.elapsed().as_secs_f64()).await;
    let init_stream = stream::iter(std::iter::once(Ok(init_message)));
    let events =
        BroadcastStream::new(session_state.broadcaster.subscribe()).filter_map(alis_message);

    let result = init_stream.chain(events).forward(sink).await;

    drainer.abort();
    result
}

async fn alis_init_message(state: &SessionStreamState, now: f64) -> ws::Message {
    let session = Arc::clone(&state.session);
    let update_result = tokio::task::spawn_blocking(move || session.update()).await;
    if let Ok(Err(e)) = update_result {
        warn!(error = %e, "Live preview session update failed");
    }

    let snapshot = state.session.live_preview_snapshot();
    let message = json!({
        "time": now,
        "cols": snapshot.cols,
        "rows": snapshot.rows,
        "init": snapshot.seq
    });
    ws::Message::Text(message.to_string())
}

async fn alis_message(
    event: Result<LiveEvent, tokio_stream::wrappers::errors::BroadcastStreamRecvError>,
) -> Option<Result<ws::Message, axum::Error>> {
    match event {
        Ok(LiveEvent::Output { time, seq }) => {
            Some(Ok(ws::Message::Text(json!([time, "o", seq]).to_string())))
        }
        Ok(LiveEvent::Resize { time, cols, rows }) => Some(Ok(ws::Message::Text(
            json!([time, "r", format!("{cols}x{rows}")]).to_string(),
        ))),
        Err(e) => Some(Err(axum::Error::new(e))),
    }
}

#[derive(Debug, Deserialize)]
struct SessionParams {
    session: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EventsParams {
    sub: Option<String>,
    session: Option<String>,
}

async fn events_handler(
    ws: ws::WebSocketUpgrade,
    Query(params): Query<EventsParams>,
    State(state): State<Arc<LivePreviewServerState>>,
) -> impl IntoResponse {
    let sub: Subscription = params.sub.unwrap_or_default().parse().unwrap_or_default();
    let session_id = match state.resolve_session_id(params.session.as_deref()) {
        Ok(id) => id,
        Err(e) => return (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    };
    let session_state = match state.get_or_create_session(&session_id) {
        Ok(session_state) => session_state,
        Err(e) => return (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    };

    ws.on_upgrade(move |socket| async move {
        let _ = handle_events_socket(socket, session_state, sub).await;
    })
}

async fn handle_events_socket(
    socket: ws::WebSocket,
    session_state: Arc<SessionStreamState>,
    sub: Subscription,
) -> Result<(), axum::Error> {
    let (sink, stream) = socket.split();
    let drainer = tokio::spawn(stream.map(Ok).forward(sink::drain()));

    let init = if sub.init {
        let session = Arc::clone(&session_state.session);
        let update_result = tokio::task::spawn_blocking(move || session.update()).await;
        if let Ok(Err(e)) = update_result {
            warn!(error = %e, "Live preview session update failed");
        }
        Some(event_init_message(&session_state))
    } else {
        None
    };

    let init_stream = stream::iter(init.into_iter().map(Ok));
    let events = BroadcastStream::new(session_state.broadcaster.subscribe())
        .filter_map(move |event| events_message(event, sub));

    let result = init_stream.chain(events).forward(sink).await;

    drainer.abort();
    result
}

fn event_init_message(state: &SessionStreamState) -> ws::Message {
    let snapshot = state.session.live_preview_snapshot();
    let text = state.session.screen_text();
    let message = json!({
        "type": "init",
        "data": {
            "cols": snapshot.cols,
            "rows": snapshot.rows,
            "pid": 0,
            "seq": snapshot.seq,
            "text": text
        }
    });
    ws::Message::Text(message.to_string())
}

async fn events_message(
    event: Result<LiveEvent, tokio_stream::wrappers::errors::BroadcastStreamRecvError>,
    sub: Subscription,
) -> Option<Result<ws::Message, axum::Error>> {
    match event {
        Ok(LiveEvent::Output { seq, .. }) if sub.output => Some(Ok(ws::Message::Text(
            json!({
                "type": "output",
                "data": { "seq": seq }
            })
            .to_string(),
        ))),
        Ok(LiveEvent::Resize { cols, rows, .. }) if sub.resize => Some(Ok(ws::Message::Text(
            json!({
                "type": "resize",
                "data": { "cols": cols, "rows": rows }
            })
            .to_string(),
        ))),
        Ok(_) => None,
        Err(e) => Some(Err(axum::Error::new(e))),
    }
}

#[derive(Debug, Default, Copy, Clone)]
struct Subscription {
    init: bool,
    output: bool,
    resize: bool,
}

impl std::str::FromStr for Subscription {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut sub = Subscription::default();
        for event in s.split(',').filter(|e| !e.is_empty()) {
            match event {
                "init" => sub.init = true,
                "output" => sub.output = true,
                "resize" => sub.resize = true,
                _ => return Err(format!("invalid event name: {event}")),
            }
        }
        Ok(sub)
    }
}
