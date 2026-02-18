//! Shared daemon RPC core used by Unix and WebSocket transports.

use crate::adapters::attach_output_to_response;
use crate::adapters::daemon::Router;
use crate::adapters::daemon::UseCaseContainer;
use crate::adapters::parse_attach_input;
use crate::adapters::parse_session_selector;
use crate::adapters::rpc::RpcRequest;
use crate::adapters::rpc::RpcResponse;
use crate::adapters::session_error_response;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Serialize;
use serde_json::json;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use crate::infra::daemon::DaemonConfig;
use crate::infra::daemon::SessionManager;
use crate::infra::daemon::SystemClock;
use crate::usecases::AttachUseCase;
use crate::usecases::ports::SessionRepository;
use crate::usecases::ports::ShutdownNotifierHandle;
use crate::usecases::ports::StreamCursor;
use crate::usecases::ports::StreamWaiterHandle;

const ATTACH_STREAM_MAX_CHUNK_BYTES: usize = 64 * 1024;
const ATTACH_STREAM_MAX_TICK_BYTES: usize = 512 * 1024;
const ATTACH_STREAM_HEARTBEAT: Duration = Duration::from_secs(30);
const LIVE_PREVIEW_STREAM_MAX_CHUNK_BYTES: usize = 64 * 1024;
const LIVE_PREVIEW_STREAM_MAX_TICK_BYTES: usize = 256 * 1024;
const LIVE_PREVIEW_STREAM_HEARTBEAT: Duration = Duration::from_secs(5);
const FLIGHTDECK_STREAM_DEFAULT_INTERVAL_MS: u64 = 1000;
const FLIGHTDECK_STREAM_MIN_INTERVAL_MS: u64 = 250;
const FLIGHTDECK_STREAM_MAX_INTERVAL_MS: u64 = 5000;
const FLIGHTDECK_STREAM_HEARTBEAT: Duration = Duration::from_secs(5);
const STREAM_WAIT_SLICE: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamWaitStatus {
    Notified,
    HeartbeatElapsed,
    Terminated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StreamKind {
    Attach,
    LivePreview,
    Flightdeck,
}

#[derive(Debug)]
pub(crate) enum RpcCoreError {
    ConnectionClosed,
    Other(String),
}

impl fmt::Display for RpcCoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionClosed => write!(f, "connection closed"),
            Self::Other(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for RpcCoreError {}

pub(crate) trait RpcResponseWriter {
    fn write_response(&mut self, response: &RpcResponse) -> Result<(), RpcCoreError>;
}

pub(crate) struct RpcCore {
    session_manager: Arc<SessionManager>,
    usecases: UseCaseContainer<SessionManager>,
    shutdown_flag: Arc<AtomicBool>,
}

impl RpcCore {
    pub fn with_config(
        config: DaemonConfig,
        shutdown_flag: Arc<AtomicBool>,
        shutdown_notifier: ShutdownNotifierHandle,
    ) -> Self {
        let session_manager = Arc::new(SessionManager::with_max_sessions(config.max_sessions()));
        let clock = Arc::new(SystemClock::new());
        let usecases = UseCaseContainer::new(
            Arc::clone(&session_manager),
            clock,
            Arc::clone(&shutdown_flag),
            shutdown_notifier,
        );
        Self {
            session_manager,
            usecases,
            shutdown_flag,
        }
    }

    pub fn session_repository_handle(&self) -> Arc<dyn SessionRepository> {
        let repository: Arc<dyn SessionRepository> = self.session_manager.clone();
        repository
    }

    pub fn shutdown_all_sessions(&self) {
        let sessions = self.session_manager.list();
        for info in sessions {
            if let Err(err) = self.session_manager.kill(info.id.as_str()) {
                tracing::warn!(session_id = %info.id, error = %err, "Failed to kill session during shutdown");
            }
        }
    }

    pub fn route(&self, request: RpcRequest) -> RpcResponse {
        let router = Router::new(&self.usecases);
        router.route(request)
    }

    pub fn stream_kind_for_method(method: &str) -> Option<StreamKind> {
        match method {
            "attach_stream" => Some(StreamKind::Attach),
            "live_preview_stream" => Some(StreamKind::LivePreview),
            "flightdeck_stream" => Some(StreamKind::Flightdeck),
            _ => None,
        }
    }

    pub fn handle_stream(
        &self,
        writer: &mut impl RpcResponseWriter,
        request: RpcRequest,
        kind: StreamKind,
        connection_cancelled: Option<&AtomicBool>,
    ) -> Result<(), RpcCoreError> {
        match kind {
            StreamKind::Attach => self.handle_attach_stream(writer, request, connection_cancelled),
            StreamKind::LivePreview => {
                self.handle_live_preview_stream(writer, request, connection_cancelled)
            }
            StreamKind::Flightdeck => {
                self.handle_flightdeck_stream(writer, request, connection_cancelled)
            }
        }
    }

    fn should_shutdown(&self) -> bool {
        self.shutdown_flag.load(Ordering::Relaxed)
    }

    fn should_stream_terminate(&self, connection_cancelled: Option<&AtomicBool>) -> bool {
        self.should_shutdown()
            || connection_cancelled.is_some_and(|flag| flag.load(Ordering::Relaxed))
    }

    fn wait_for_stream_event_or_tick(
        &self,
        subscription: &StreamWaiterHandle,
        heartbeat: Duration,
        connection_cancelled: Option<&AtomicBool>,
    ) -> StreamWaitStatus {
        let deadline = Instant::now() + heartbeat;
        loop {
            if self.should_stream_terminate(connection_cancelled) {
                return StreamWaitStatus::Terminated;
            }
            let now = Instant::now();
            if now >= deadline {
                return StreamWaitStatus::HeartbeatElapsed;
            }
            let wait = deadline
                .saturating_duration_since(now)
                .min(STREAM_WAIT_SLICE);
            if subscription.wait(Some(wait)) {
                return StreamWaitStatus::Notified;
            }
        }
    }

    fn handle_attach_stream(
        &self,
        writer: &mut impl RpcResponseWriter,
        request: RpcRequest,
        connection_cancelled: Option<&AtomicBool>,
    ) -> Result<(), RpcCoreError> {
        let req_id = request.id;
        let input = match parse_attach_input(&request) {
            Ok(input) => input,
            Err(response) => {
                let _ = writer.write_response(&response);
                return Ok(());
            }
        };

        let session_id = match self.usecases.session.attach.execute(input) {
            Ok(output) => {
                let response = attach_output_to_response(req_id, &output);
                writer.write_response(&response)?;
                output.session_id
            }
            Err(err) => {
                let response = session_error_response(req_id, err);
                let _ = writer.write_response(&response);
                return Ok(());
            }
        };

        let session =
            match SessionRepository::resolve(self.session_manager.as_ref(), Some(&session_id)) {
                Ok(session) => session,
                Err(err) => {
                    let response = session_error_response(req_id, err);
                    let _ = writer.write_response(&response);
                    return Ok(());
                }
            };

        if let Err(err) = session.update() {
            let response = session_error_response(req_id, err);
            let _ = writer.write_response(&response);
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
                session_id: session_id.as_str(),
            },
        );
        writer.write_response(&ready)?;

        let stream_seq = session.live_preview_snapshot().stream_seq;
        let subscription = session.stream_subscribe();
        let mut cursor = StreamCursor { seq: stream_seq };

        loop {
            if self.should_stream_terminate(connection_cancelled) {
                let response = RpcResponse::success_json(req_id, &AttachEvent { event: "closed" });
                let _ = writer.write_response(&response);
                return Ok(());
            }
            let mut budget = ATTACH_STREAM_MAX_TICK_BYTES;
            let mut sent_any = false;

            loop {
                if self.should_stream_terminate(connection_cancelled) {
                    let response =
                        RpcResponse::success_json(req_id, &AttachEvent { event: "closed" });
                    let _ = writer.write_response(&response);
                    return Ok(());
                }
                if budget == 0 {
                    break;
                }

                let max_chunk = budget.min(ATTACH_STREAM_MAX_CHUNK_BYTES);
                let read = match session.stream_read(&mut cursor, max_chunk, 0) {
                    Ok(read) => read,
                    Err(err) => {
                        let response = session_error_response(req_id, err);
                        let _ = writer.write_response(&response);
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
                    writer.write_response(&response)?;
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
                    writer.write_response(&response)?;
                    sent_any = true;
                    budget = budget.saturating_sub(read.data.len());
                    if read.closed {
                        let response =
                            RpcResponse::success_json(req_id, &AttachEvent { event: "closed" });
                        let _ = writer.write_response(&response);
                        return Ok(());
                    }
                    continue;
                }

                if read.closed {
                    let response =
                        RpcResponse::success_json(req_id, &AttachEvent { event: "closed" });
                    let _ = writer.write_response(&response);
                    return Ok(());
                }

                break;
            }

            if sent_any && budget == 0 {
                continue;
            }

            match self.wait_for_stream_event_or_tick(
                &subscription,
                ATTACH_STREAM_HEARTBEAT,
                connection_cancelled,
            ) {
                StreamWaitStatus::Notified => {}
                StreamWaitStatus::HeartbeatElapsed => {
                    let response =
                        RpcResponse::success_json(req_id, &AttachEvent { event: "heartbeat" });
                    writer.write_response(&response)?;
                }
                StreamWaitStatus::Terminated => {
                    let response =
                        RpcResponse::success_json(req_id, &AttachEvent { event: "closed" });
                    let _ = writer.write_response(&response);
                    return Ok(());
                }
            }
        }
    }

    fn handle_live_preview_stream(
        &self,
        writer: &mut impl RpcResponseWriter,
        request: RpcRequest,
        connection_cancelled: Option<&AtomicBool>,
    ) -> Result<(), RpcCoreError> {
        let req_id = request.id;
        let session_param = parse_live_preview_session_selector(&request);

        let session =
            match SessionRepository::resolve(self.session_manager.as_ref(), session_param.as_ref())
            {
                Ok(session) => session,
                Err(err) => {
                    let response = session_error_response(req_id, err);
                    let _ = writer.write_response(&response);
                    return Ok(());
                }
            };

        if let Err(err) = session.update() {
            let response = session_error_response(req_id, err);
            let _ = writer.write_response(&response);
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
        writer.write_response(&ready)?;

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
        writer.write_response(&init)?;

        let subscription = session.stream_subscribe();
        let mut cursor = live_preview_initial_cursor(&snapshot);
        let mut last_size = (snapshot.cols, snapshot.rows);

        loop {
            if self.should_stream_terminate(connection_cancelled) {
                let response = RpcResponse::success_json(
                    req_id,
                    &LivePreviewClosed {
                        event: "closed",
                        time: start_time.elapsed().as_secs_f64(),
                    },
                );
                let _ = writer.write_response(&response);
                return Ok(());
            }
            let mut budget = LIVE_PREVIEW_STREAM_MAX_TICK_BYTES;
            let mut sent_any = false;

            loop {
                if self.should_stream_terminate(connection_cancelled) {
                    let response = RpcResponse::success_json(
                        req_id,
                        &LivePreviewClosed {
                            event: "closed",
                            time: start_time.elapsed().as_secs_f64(),
                        },
                    );
                    let _ = writer.write_response(&response);
                    return Ok(());
                }
                if budget == 0 {
                    break;
                }

                let max_chunk = budget.min(LIVE_PREVIEW_STREAM_MAX_CHUNK_BYTES);
                let read = match session.stream_read(&mut cursor, max_chunk, 0) {
                    Ok(read) => read,
                    Err(err) => {
                        let response = session_error_response(req_id, err);
                        let _ = writer.write_response(&response);
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
                    writer.write_response(&dropped)?;
                    if let Err(err) = session.update() {
                        let response = session_error_response(req_id, err);
                        let _ = writer.write_response(&response);
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
                    writer.write_response(&init)?;
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
                    writer.write_response(&response)?;
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
                        let _ = writer.write_response(&response);
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
                    let _ = writer.write_response(&response);
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
                writer.write_response(&resize)?;
                last_size = size;
                sent_any = true;
            }

            if sent_any && budget == 0 {
                continue;
            }

            match self.wait_for_stream_event_or_tick(
                &subscription,
                LIVE_PREVIEW_STREAM_HEARTBEAT,
                connection_cancelled,
            ) {
                StreamWaitStatus::Notified => {}
                StreamWaitStatus::HeartbeatElapsed => {
                    let response = RpcResponse::success_json(
                        req_id,
                        &LivePreviewHeartbeat {
                            event: "heartbeat",
                            time: start_time.elapsed().as_secs_f64(),
                        },
                    );
                    writer.write_response(&response)?;
                }
                StreamWaitStatus::Terminated => {
                    let response = RpcResponse::success_json(
                        req_id,
                        &LivePreviewClosed {
                            event: "closed",
                            time: start_time.elapsed().as_secs_f64(),
                        },
                    );
                    let _ = writer.write_response(&response);
                    return Ok(());
                }
            }
        }
    }

    fn flightdeck_snapshot(&self) -> FlightdeckSnapshot {
        let mut sessions = self
            .session_manager
            .list()
            .into_iter()
            .map(|session| FlightdeckSessionSnapshot {
                id: session.id.to_string(),
                command: session.command,
                pid: session.pid,
                running: session.running,
                created_at: session.created_at,
                cols: session.size.cols(),
                rows: session.size.rows(),
            })
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| left.id.cmp(&right.id));
        FlightdeckSnapshot {
            active_session: self
                .session_manager
                .active_session_id()
                .map(|session_id| session_id.to_string()),
            sessions,
        }
    }

    fn handle_flightdeck_stream(
        &self,
        writer: &mut impl RpcResponseWriter,
        request: RpcRequest,
        connection_cancelled: Option<&AtomicBool>,
    ) -> Result<(), RpcCoreError> {
        let req_id = request.id;
        let interval_ms = request
            .param_u64("interval_ms", FLIGHTDECK_STREAM_DEFAULT_INTERVAL_MS)
            .clamp(
                FLIGHTDECK_STREAM_MIN_INTERVAL_MS,
                FLIGHTDECK_STREAM_MAX_INTERVAL_MS,
            );
        let interval = Duration::from_millis(interval_ms);
        let start_time = Instant::now();

        #[derive(Serialize)]
        struct FlightdeckEvent {
            event: &'static str,
            active_session: Option<String>,
            sessions: Vec<serde_json::Value>,
            #[serde(skip_serializing_if = "Option::is_none")]
            time: Option<f64>,
        }

        #[derive(Serialize)]
        struct FlightdeckHeartbeat {
            event: &'static str,
            time: f64,
        }

        #[derive(Serialize)]
        struct FlightdeckClosed {
            event: &'static str,
            time: f64,
        }

        let mut snapshot = self.flightdeck_snapshot();
        writer.write_response(&RpcResponse::success_json(
            req_id,
            &FlightdeckEvent {
                event: "ready",
                active_session: snapshot.active_session.clone(),
                sessions: snapshot.to_json_sessions(),
                time: None,
            },
        ))?;

        let mut next_snapshot_deadline = Instant::now() + interval;
        let mut next_heartbeat_deadline = Instant::now() + FLIGHTDECK_STREAM_HEARTBEAT;

        loop {
            if self.should_stream_terminate(connection_cancelled) {
                let _ = writer.write_response(&RpcResponse::success_json(
                    req_id,
                    &FlightdeckClosed {
                        event: "closed",
                        time: start_time.elapsed().as_secs_f64(),
                    },
                ));
                return Ok(());
            }

            let now = Instant::now();
            if now >= next_snapshot_deadline {
                let next_snapshot = self.flightdeck_snapshot();
                if next_snapshot != snapshot {
                    writer.write_response(&RpcResponse::success_json(
                        req_id,
                        &FlightdeckEvent {
                            event: "sessions",
                            active_session: next_snapshot.active_session.clone(),
                            sessions: next_snapshot.to_json_sessions(),
                            time: Some(start_time.elapsed().as_secs_f64()),
                        },
                    ))?;
                    snapshot = next_snapshot;
                }
                next_snapshot_deadline = now + interval;
                continue;
            }

            if now >= next_heartbeat_deadline {
                writer.write_response(&RpcResponse::success_json(
                    req_id,
                    &FlightdeckHeartbeat {
                        event: "heartbeat",
                        time: start_time.elapsed().as_secs_f64(),
                    },
                ))?;
                next_heartbeat_deadline = now + FLIGHTDECK_STREAM_HEARTBEAT;
                continue;
            }

            let next_deadline = if next_snapshot_deadline <= next_heartbeat_deadline {
                next_snapshot_deadline
            } else {
                next_heartbeat_deadline
            };
            let wait = next_deadline
                .saturating_duration_since(now)
                .min(STREAM_WAIT_SLICE);
            std::thread::park_timeout(wait);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FlightdeckSessionSnapshot {
    id: String,
    command: String,
    pid: u32,
    running: bool,
    created_at: String,
    cols: u16,
    rows: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FlightdeckSnapshot {
    active_session: Option<String>,
    sessions: Vec<FlightdeckSessionSnapshot>,
}

impl FlightdeckSnapshot {
    fn to_json_sessions(&self) -> Vec<serde_json::Value> {
        self.sessions
            .iter()
            .map(|session| {
                json!({
                    "id": session.id,
                    "command": session.command,
                    "pid": session.pid,
                    "running": session.running,
                    "created_at": session.created_at,
                    "size": {
                        "cols": session.cols,
                        "rows": session.rows,
                    }
                })
            })
            .collect()
    }
}

fn parse_live_preview_session_selector(request: &RpcRequest) -> Option<crate::domain::SessionId> {
    parse_session_selector(request.param_str("session").map(String::from))
}

fn live_preview_initial_cursor(
    snapshot: &crate::usecases::ports::LivePreviewSnapshot,
) -> StreamCursor {
    StreamCursor {
        seq: snapshot.stream_seq,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;
    use std::time::Duration;
    use std::time::Instant;

    fn make_request(params_json: Option<&str>) -> RpcRequest {
        let request_json = match params_json {
            Some(params) => {
                format!(
                    r#"{{"jsonrpc":"2.0","id":1,"method":"live_preview_stream","params":{params}}}"#
                )
            }
            None => r#"{"jsonrpc":"2.0","id":1,"method":"live_preview_stream"}"#.to_string(),
        };
        serde_json::from_str(&request_json).expect("valid rpc request")
    }

    fn make_flightdeck_request(params_json: Option<&str>) -> RpcRequest {
        let request_json = match params_json {
            Some(params) => {
                format!(r#"{{"jsonrpc":"2.0","id":7,"method":"flightdeck_stream","params":{params}}}"#)
            }
            None => r#"{"jsonrpc":"2.0","id":7,"method":"flightdeck_stream"}"#.to_string(),
        };
        serde_json::from_str(&request_json).expect("valid rpc request")
    }

    #[derive(Clone, Default)]
    struct RecordingWriterHandle {
        values: Arc<Mutex<Vec<Value>>>,
    }

    impl RecordingWriterHandle {
        fn snapshot(&self) -> Vec<Value> {
            self.values
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .clone()
        }

        fn wait_for_event(&self, event: &str, timeout: Duration) -> Option<Value> {
            let deadline = Instant::now() + timeout;
            loop {
                let values = self.snapshot();
                if let Some(found) = values
                    .into_iter()
                    .find(|value| value["result"]["event"] == event)
                {
                    return Some(found);
                }
                if Instant::now() >= deadline {
                    return None;
                }
                std::thread::park_timeout(Duration::from_millis(20));
            }
        }
    }

    struct RecordingWriter {
        handle: RecordingWriterHandle,
    }

    impl RecordingWriter {
        fn new() -> (Self, RecordingWriterHandle) {
            let handle = RecordingWriterHandle::default();
            (
                Self {
                    handle: handle.clone(),
                },
                handle,
            )
        }
    }

    impl RpcResponseWriter for RecordingWriter {
        fn write_response(&mut self, response: &RpcResponse) -> Result<(), RpcCoreError> {
            let value = serde_json::to_value(response)
                .map_err(|err| RpcCoreError::Other(format!("serialize response: {err}")))?;
            self.handle
                .values
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .push(value);
            Ok(())
        }
    }

    #[test]
    fn live_preview_selector_maps_active_to_none() {
        let request = make_request(Some(r#"{"session":"active"}"#));
        let parsed = parse_live_preview_session_selector(&request);
        assert!(parsed.is_none());
    }

    #[test]
    fn live_preview_selector_defaults_to_none_when_omitted() {
        let request = make_request(None);
        let parsed = parse_live_preview_session_selector(&request);
        assert!(parsed.is_none());
    }

    #[test]
    fn live_preview_selector_keeps_explicit_session_id() {
        let request = make_request(Some(r#"{"session":"sess-1"}"#));
        let parsed = parse_live_preview_session_selector(&request).expect("session id");
        assert_eq!(parsed.as_str(), "sess-1");
    }

    #[test]
    fn live_preview_initial_cursor_uses_snapshot_stream_seq() {
        let snapshot = crate::usecases::ports::LivePreviewSnapshot {
            cols: 80,
            rows: 24,
            seq: String::new(),
            stream_seq: 123,
        };
        let cursor = live_preview_initial_cursor(&snapshot);
        assert_eq!(cursor.seq, 123);
    }

    struct SleepWaiter;

    impl crate::usecases::ports::StreamWaiter for SleepWaiter {
        fn wait(&self, timeout: Option<Duration>) -> bool {
            if let Some(timeout) = timeout {
                std::thread::park_timeout(timeout);
            }
            false
        }
    }

    #[test]
    fn stream_wait_exits_early_when_connection_is_cancelled() {
        let shutdown = Arc::new(AtomicBool::new(false));
        let notifier: crate::usecases::ports::ShutdownNotifierHandle =
            Arc::new(crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier);
        let core = RpcCore::with_config(
            crate::infra::daemon::DaemonConfig::default(),
            shutdown,
            notifier,
        );

        let subscription: crate::usecases::ports::StreamWaiterHandle = Arc::new(SleepWaiter);
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancelled_for_thread = Arc::clone(&cancelled);
        let join = std::thread::spawn(move || {
            std::thread::park_timeout(Duration::from_millis(100));
            cancelled_for_thread.store(true, Ordering::Relaxed);
        });

        let start = Instant::now();
        let status = core.wait_for_stream_event_or_tick(
            &subscription,
            Duration::from_secs(30),
            Some(cancelled.as_ref()),
        );
        let _ = join.join();

        assert_eq!(status, StreamWaitStatus::Terminated);
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "wait should terminate quickly once cancelled"
        );
    }

    #[test]
    fn stream_kind_recognizes_flightdeck_stream() {
        assert_eq!(
            RpcCore::stream_kind_for_method("flightdeck_stream"),
            Some(StreamKind::Flightdeck)
        );
    }

    #[test]
    fn flightdeck_stream_emits_ready_with_sessions_payload() {
        let shutdown = Arc::new(AtomicBool::new(false));
        let notifier: crate::usecases::ports::ShutdownNotifierHandle =
            Arc::new(crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier);
        let core = Arc::new(RpcCore::with_config(
            crate::infra::daemon::DaemonConfig::default(),
            shutdown,
            notifier,
        ));

        let (mut writer, handle) = RecordingWriter::new();
        let cancelled = Arc::new(AtomicBool::new(false));
        let request = make_flightdeck_request(Some(r#"{"interval_ms":250}"#));

        let core_for_stream = Arc::clone(&core);
        let cancelled_for_stream = Arc::clone(&cancelled);
        let join = std::thread::spawn(move || {
            let _ = core_for_stream.handle_stream(
                &mut writer,
                request,
                StreamKind::Flightdeck,
                Some(cancelled_for_stream.as_ref()),
            );
        });

        let ready = handle.wait_for_event("ready", Duration::from_secs(2));
        cancelled.store(true, Ordering::Relaxed);
        let _ = join.join();

        let Some(ready) = ready else {
            panic!("flightdeck stream did not emit ready event");
        };
        assert!(ready["result"]["sessions"].is_array());
        assert!(
            ready["result"].get("active_session").is_some(),
            "ready payload should include active_session"
        );
    }

    #[cfg(unix)]
    #[test]
    fn flightdeck_stream_emits_sessions_event_when_inventory_changes() {
        let shutdown = Arc::new(AtomicBool::new(false));
        let notifier: crate::usecases::ports::ShutdownNotifierHandle =
            Arc::new(crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier);
        let core = Arc::new(RpcCore::with_config(
            crate::infra::daemon::DaemonConfig::default(),
            shutdown,
            notifier,
        ));

        let (mut writer, handle) = RecordingWriter::new();
        let cancelled = Arc::new(AtomicBool::new(false));
        let request = make_flightdeck_request(Some(r#"{"interval_ms":250}"#));

        let core_for_stream = Arc::clone(&core);
        let cancelled_for_stream = Arc::clone(&cancelled);
        let join = std::thread::spawn(move || {
            let _ = core_for_stream.handle_stream(
                &mut writer,
                request,
                StreamKind::Flightdeck,
                Some(cancelled_for_stream.as_ref()),
            );
        });

        let _ = handle.wait_for_event("ready", Duration::from_secs(2));
        let spawn_result =
            core.session_manager
                .spawn("sh", &[], None, None, Some("flightdeck-new".to_string()), 80, 24);
        if spawn_result.is_err() {
            cancelled.store(true, Ordering::Relaxed);
            let _ = join.join();
            return;
        }

        let sessions = handle.wait_for_event("sessions", Duration::from_secs(3));
        cancelled.store(true, Ordering::Relaxed);
        let _ = join.join();
        core.shutdown_all_sessions();

        let Some(sessions) = sessions else {
            panic!("flightdeck stream did not emit sessions event");
        };
        let contains_new = sessions["result"]["sessions"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .any(|item| item.get("id").and_then(|v| v.as_str()) == Some("flightdeck-new"))
            })
            .unwrap_or(false);
        assert!(contains_new, "sessions event should include newly spawned session");
    }

    #[test]
    fn flightdeck_stream_emits_closed_and_exits_on_cancellation() {
        let shutdown = Arc::new(AtomicBool::new(false));
        let notifier: crate::usecases::ports::ShutdownNotifierHandle =
            Arc::new(crate::usecases::ports::shutdown_notifier::NoopShutdownNotifier);
        let core = Arc::new(RpcCore::with_config(
            crate::infra::daemon::DaemonConfig::default(),
            shutdown,
            notifier,
        ));

        let (mut writer, handle) = RecordingWriter::new();
        let cancelled = Arc::new(AtomicBool::new(false));
        let request = make_flightdeck_request(Some(r#"{"interval_ms":250}"#));

        let core_for_stream = Arc::clone(&core);
        let cancelled_for_stream = Arc::clone(&cancelled);
        let start = Instant::now();
        let join = std::thread::spawn(move || {
            let _ = core_for_stream.handle_stream(
                &mut writer,
                request,
                StreamKind::Flightdeck,
                Some(cancelled_for_stream.as_ref()),
            );
        });

        let _ = handle.wait_for_event("ready", Duration::from_secs(2));
        cancelled.store(true, Ordering::Relaxed);
        let _ = join.join();

        let closed = handle.wait_for_event("closed", Duration::from_secs(1));
        assert!(closed.is_some(), "expected closed event on cancellation");
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "stream should exit quickly when cancelled"
        );
    }
}
