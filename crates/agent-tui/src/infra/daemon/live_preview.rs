use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use axum::{
    Router,
    extract::{Query, State, ws},
    http::header,
    response::{Html, IntoResponse},
    routing::get,
};
use futures_util::{StreamExt, sink, stream};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::{broadcast, watch};
use tokio_stream::wrappers::BroadcastStream;
use tracing::{info, warn};

use crate::domain::core::CursorPosition;
use crate::domain::session_types::SessionId;
use crate::domain::{LivePreviewStartOutput, LivePreviewStatusOutput, LivePreviewStopOutput};
use crate::usecases::ports::{
    LivePreviewError, LivePreviewOptions, LivePreviewService, SessionHandle,
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
    state: Mutex<LivePreviewState>,
}

impl LivePreviewManager {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(LivePreviewState { server: None }),
        }
    }
}

impl Default for LivePreviewManager {
    fn default() -> Self {
        Self::new()
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

        let handle = spawn_preview_server(session, options.listen_addr)?;
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
    session: SessionHandle,
    listen_addr: SocketAddr,
) -> Result<LivePreviewServerHandle, LivePreviewError> {
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<SocketAddr, LivePreviewError>>();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let session_id = session.session_id();

    let thread_name = format!("live-preview-{}", session_id.as_str());
    let join = thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
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

                let server_state = Arc::new(LivePreviewServerState::new(session));
                let app = build_router(Arc::clone(&server_state));
                let mut shutdown = shutdown_rx.clone();
                let pump_shutdown = shutdown_rx.clone();

                let pump_handle =
                    tokio::spawn(pump_events(Arc::clone(&server_state), pump_shutdown));
                info!(listen = %actual_addr, "Live preview server started");

                let _ = axum::serve(listener, app)
                    .with_graceful_shutdown(async move {
                        let _ = shutdown.changed().await;
                    })
                    .await;

                pump_handle.abort();
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
    session: SessionHandle,
    broadcaster: broadcast::Sender<LiveEvent>,
    snapshot: Arc<Mutex<SnapshotState>>,
    start_time: Instant,
}

impl LivePreviewServerState {
    fn new(session: SessionHandle) -> Self {
        let _ = session.update();
        let screen = session.screen_text();
        let cursor = session.cursor();
        let size = session.size();
        let snapshot = SnapshotState {
            screen,
            cursor,
            size,
        };
        let (tx, _) = broadcast::channel(128);
        Self {
            session,
            broadcaster: tx,
            snapshot: Arc::new(Mutex::new(snapshot)),
            start_time: Instant::now(),
        }
    }

    fn snapshot(&self) -> SnapshotState {
        self.snapshot
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn now(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }
}

#[derive(Clone)]
struct SnapshotState {
    screen: String,
    cursor: CursorPosition,
    size: (u16, u16),
}

#[derive(Clone)]
enum LiveEvent {
    Output { time: f64, seq: String },
    Resize { time: f64, cols: u16, rows: u16 },
}

async fn pump_events(state: Arc<LivePreviewServerState>, mut shutdown: watch::Receiver<bool>) {
    let mut interval = tokio::time::interval(POLL_INTERVAL);

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    break;
                }
            }
            _ = interval.tick() => {
                if let Err(e) = state.session.update() {
                    warn!(error = %e, "Live preview session update failed");
                    continue;
                }

                let screen = state.session.screen_text();
                let cursor = state.session.cursor();
                let (cols, rows) = state.session.size();

                let mut snapshot = state.snapshot.lock().unwrap_or_else(|e| e.into_inner());
                let time = state.now();

                if snapshot.size != (cols, rows) {
                    let _ = state
                        .broadcaster
                        .send(LiveEvent::Resize { time, cols, rows });
                    snapshot.size = (cols, rows);
                }

                if snapshot.screen != screen || snapshot.cursor != cursor {
                    let seq = build_frame(&screen, &cursor);
                    let _ = state
                        .broadcaster
                        .send(LiveEvent::Output { time, seq });
                    snapshot.screen = screen;
                    snapshot.cursor = cursor;
                }
            }
        }
    }
}

fn build_router(state: Arc<LivePreviewServerState>) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/asciinema-player.css", get(css_handler))
        .route("/asciinema-player.min.js", get(js_handler))
        .route("/ws/alis", get(alis_handler))
        .route("/ws/events", get(events_handler))
        .with_state(state)
}

async fn index_handler() -> impl IntoResponse {
    Html(INDEX_HTML)
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
    State(state): State<Arc<LivePreviewServerState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        let _ = handle_alis_socket(socket, state).await;
    })
}

async fn handle_alis_socket(
    socket: ws::WebSocket,
    state: Arc<LivePreviewServerState>,
) -> Result<(), axum::Error> {
    let (sink, stream) = socket.split();
    let drainer = tokio::spawn(stream.map(Ok).forward(sink::drain()));

    let init_message = alis_init_message(&state);
    let init_stream = stream::iter(std::iter::once(Ok(init_message)));
    let events = BroadcastStream::new(state.broadcaster.subscribe()).filter_map(alis_message);

    let result = init_stream.chain(events).forward(sink).await;

    drainer.abort();
    result
}

fn alis_init_message(state: &LivePreviewServerState) -> ws::Message {
    let snapshot = state.snapshot();
    let seq = build_frame(&snapshot.screen, &snapshot.cursor);
    let message = json!({
        "time": state.now(),
        "cols": snapshot.size.0,
        "rows": snapshot.size.1,
        "init": seq
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
struct EventsParams {
    sub: Option<String>,
}

async fn events_handler(
    ws: ws::WebSocketUpgrade,
    Query(params): Query<EventsParams>,
    State(state): State<Arc<LivePreviewServerState>>,
) -> impl IntoResponse {
    let sub: Subscription = params.sub.unwrap_or_default().parse().unwrap_or_default();

    ws.on_upgrade(move |socket| async move {
        let _ = handle_events_socket(socket, state, sub).await;
    })
}

async fn handle_events_socket(
    socket: ws::WebSocket,
    state: Arc<LivePreviewServerState>,
    sub: Subscription,
) -> Result<(), axum::Error> {
    let (sink, stream) = socket.split();
    let drainer = tokio::spawn(stream.map(Ok).forward(sink::drain()));

    let init = if sub.init {
        Some(event_init_message(&state))
    } else {
        None
    };

    let init_stream = stream::iter(init.into_iter().map(Ok));
    let events = BroadcastStream::new(state.broadcaster.subscribe())
        .filter_map(move |event| events_message(event, sub));

    let result = init_stream.chain(events).forward(sink).await;

    drainer.abort();
    result
}

fn event_init_message(state: &LivePreviewServerState) -> ws::Message {
    let snapshot = state.snapshot();
    let seq = build_frame(&snapshot.screen, &snapshot.cursor);
    let message = json!({
        "type": "init",
        "data": {
            "cols": snapshot.size.0,
            "rows": snapshot.size.1,
            "pid": 0,
            "seq": seq,
            "text": snapshot.screen
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

fn build_frame(screen: &str, cursor: &CursorPosition) -> String {
    let mut seq = String::new();
    seq.push_str("\u{1b}[2J\u{1b}[H");
    if !screen.is_empty() {
        seq.push_str(screen);
    }
    let row = cursor.row.saturating_add(1);
    let col = cursor.col.saturating_add(1);
    seq.push_str(&format!("\u{1b}[{row};{col}H"));
    if cursor.visible {
        seq.push_str("\u{1b}[?25h");
    } else {
        seq.push_str("\u{1b}[?25l");
    }
    seq
}
