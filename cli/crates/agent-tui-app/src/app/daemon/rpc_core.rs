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

use crate::domain::TerminalSize;
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

#[derive(Debug, Clone, Copy)]
struct StreamTiming {
    attach_heartbeat: Duration,
    live_preview_heartbeat: Duration,
    flightdeck_heartbeat: Duration,
    wait_slice: Duration,
}

impl Default for StreamTiming {
    fn default() -> Self {
        Self {
            attach_heartbeat: ATTACH_STREAM_HEARTBEAT,
            live_preview_heartbeat: LIVE_PREVIEW_STREAM_HEARTBEAT,
            flightdeck_heartbeat: FLIGHTDECK_STREAM_HEARTBEAT,
            wait_slice: STREAM_WAIT_SLICE,
        }
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct RpcCoreTestConfig {
    pub stream_max_buffer_bytes: usize,
    pub attach_heartbeat: Duration,
    pub live_preview_heartbeat: Duration,
    pub flightdeck_heartbeat: Duration,
    pub wait_slice: Duration,
}

#[cfg(test)]
impl Default for RpcCoreTestConfig {
    fn default() -> Self {
        Self {
            stream_max_buffer_bytes: 8 * 1024 * 1024,
            attach_heartbeat: ATTACH_STREAM_HEARTBEAT,
            live_preview_heartbeat: LIVE_PREVIEW_STREAM_HEARTBEAT,
            flightdeck_heartbeat: FLIGHTDECK_STREAM_HEARTBEAT,
            wait_slice: STREAM_WAIT_SLICE,
        }
    }
}

#[cfg(test)]
impl RpcCoreTestConfig {
    fn stream_timing(self) -> StreamTiming {
        StreamTiming {
            attach_heartbeat: self.attach_heartbeat,
            live_preview_heartbeat: self.live_preview_heartbeat,
            flightdeck_heartbeat: self.flightdeck_heartbeat,
            wait_slice: self.wait_slice,
        }
    }
}

fn validated_terminal_size(cols: u16, rows: u16) -> TerminalSize {
    match TerminalSize::try_new(cols, rows) {
        Ok(size) => size,
        Err(err) => {
            debug_assert!(
                false,
                "live preview snapshot should carry validated sizes: {err}"
            );
            tracing::warn!(
                cols,
                rows,
                error = %err,
                "Live preview snapshot carried invalid terminal dimensions; falling back to default size",
            );
            TerminalSize::default()
        }
    }
}
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
    stream_timing: StreamTiming,
}

impl RpcCore {
    fn build(
        session_manager: Arc<SessionManager>,
        shutdown_flag: Arc<AtomicBool>,
        shutdown_notifier: ShutdownNotifierHandle,
        stream_timing: StreamTiming,
    ) -> Self {
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
            stream_timing,
        }
    }

    pub fn with_config(
        config: DaemonConfig,
        shutdown_flag: Arc<AtomicBool>,
        shutdown_notifier: ShutdownNotifierHandle,
    ) -> Result<Self, crate::infra::daemon::SessionError> {
        let session_manager = Arc::new(SessionManager::with_max_sessions(config.max_sessions())?);
        Ok(Self::build(
            session_manager,
            shutdown_flag,
            shutdown_notifier,
            StreamTiming::default(),
        ))
    }

    #[cfg(test)]
    pub(crate) fn with_test_config(
        config: DaemonConfig,
        shutdown_flag: Arc<AtomicBool>,
        shutdown_notifier: ShutdownNotifierHandle,
        test_config: RpcCoreTestConfig,
    ) -> Result<Self, crate::infra::daemon::SessionError> {
        let session_manager = Arc::new(SessionManager::with_test_limits(
            config.max_sessions(),
            test_config.stream_max_buffer_bytes,
        )?);
        Ok(Self::build(
            session_manager,
            shutdown_flag,
            shutdown_notifier,
            test_config.stream_timing(),
        ))
    }

    pub fn session_repository_handle(&self) -> Arc<dyn SessionRepository> {
        let repository: Arc<dyn SessionRepository> = self.session_manager.clone();
        repository
    }

    pub fn shutdown_all_sessions(&self) {
        let sessions = self.session_manager.list();
        for info in sessions {
            if let Err(err) = self.session_manager.kill(&info.id) {
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
                .min(self.stream_timing.wait_slice);
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
                self.stream_timing.attach_heartbeat,
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
        let session_param = match parse_live_preview_session_selector(&request) {
            Ok(session_id) => session_id,
            Err(response) => {
                let _ = writer.write_response(&response);
                return Ok(());
            }
        };

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
        let mut last_size = validated_terminal_size(snapshot.cols, snapshot.rows);

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
                    last_size = validated_terminal_size(snapshot.cols, snapshot.rows);
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
                        cols: size.cols(),
                        rows: size.rows(),
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
                self.stream_timing.live_preview_heartbeat,
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
        let mut next_heartbeat_deadline = Instant::now() + self.stream_timing.flightdeck_heartbeat;

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
                next_heartbeat_deadline = now + self.stream_timing.flightdeck_heartbeat;
                continue;
            }

            let next_deadline = if next_snapshot_deadline <= next_heartbeat_deadline {
                next_snapshot_deadline
            } else {
                next_heartbeat_deadline
            };
            let wait = next_deadline
                .saturating_duration_since(now)
                .min(self.stream_timing.wait_slice);
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

#[allow(clippy::result_large_err)]
fn parse_live_preview_session_selector(
    request: &RpcRequest,
) -> Result<Option<crate::domain::SessionId>, RpcResponse> {
    parse_session_selector(request.id, request.param_str("session").map(String::from))
}

fn live_preview_initial_cursor(
    snapshot: &crate::usecases::ports::LivePreviewSnapshot,
) -> StreamCursor {
    StreamCursor {
        seq: snapshot.stream_seq,
    }
}

#[cfg(test)]
#[path = "rpc_core_tests.rs"]
mod tests;
